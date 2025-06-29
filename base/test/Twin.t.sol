// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Twin} from "../src/Twin.sol";
import {Call, CallLib, CallType} from "../src/libraries/CallLib.sol";
import {Test, console} from "forge-std/Test.sol";

contract TwinTest is Test {
    Twin public twin;
    address public portal;
    address public unauthorized;

    // Mock contracts for testing
    MockTarget public mockTarget;
    MockRevertingTarget public mockRevertingTarget;

    // Events (Twin doesn't emit any, but CallLib operations might trigger events from targets)
    event MockEvent(uint256 value);

    function setUp() public {
        portal = makeAddr("portal");
        unauthorized = makeAddr("unauthorized");

        // Deploy Twin with portal address
        twin = new Twin();

        // Deploy mock contracts for testing
        mockTarget = new MockTarget();
        mockRevertingTarget = new MockRevertingTarget();

        // Fund the twin contract with some ETH for testing
        vm.deal(address(twin), 10 ether);
        vm.deal(address(this), 10 ether);
    }

    //////////////////////////////////////////////////////////////
    ///                   Constructor Tests                    ///
    //////////////////////////////////////////////////////////////

    function test_constructor_setsPortalCorrectly() public {
        // The constructor sets BRIDGE to msg.sender (the deployer)
        Twin testTwin = new Twin();

        // Since this test contract deployed it, BRIDGE should be set to this contract's address
        assertEq(testTwin.BRIDGE(), address(this));
    }

    function test_constructor_setBridgeToDeployer() public {
        // Constructor should set BRIDGE to the deployer (msg.sender)
        Twin testTwin = new Twin();
        assertEq(testTwin.BRIDGE(), address(this));

        // Verify BRIDGE is immutable and set correctly
        assertTrue(testTwin.BRIDGE() != address(0));
    }

    //////////////////////////////////////////////////////////////
    ///                   Receive Tests                        ///
    //////////////////////////////////////////////////////////////

    function test_receive_acceptsEther() public {
        uint256 initialBalance = address(twin).balance;
        uint256 sendAmount = 1 ether;

        (bool success,) = address(twin).call{value: sendAmount}("");

        assertTrue(success);
        assertEq(address(twin).balance, initialBalance + sendAmount);
    }

    function test_receive_acceptsZeroEther() public {
        uint256 initialBalance = address(twin).balance;

        (bool success,) = address(twin).call{value: 0}("");

        assertTrue(success);
        assertEq(address(twin).balance, initialBalance);
    }

    //////////////////////////////////////////////////////////////
    ///                 Execute Access Control Tests          ///
    //////////////////////////////////////////////////////////////

    function test_execute_allowsBridgeCaller() public {
        Call memory call = Call({
            ty: CallType.Call,
            to: address(mockTarget),
            value: 0,
            data: abi.encodeWithSelector(MockTarget.setValue.selector, 42)
        });

        // The test contract is the BRIDGE since it deployed the Twin
        // No need to prank since we're already the authorized caller
        twin.execute(call);

        assertEq(mockTarget.value(), 42);
    }

    function test_execute_allowsSelfCaller() public {
        // Create a call that will be executed by the twin calling itself
        Call memory call = Call({
            ty: CallType.Call,
            to: address(mockTarget),
            value: 0,
            data: abi.encodeWithSelector(MockTarget.setValue.selector, 123)
        });

        // Create a call to execute the above call (recursive call)
        Call memory selfCall = Call({
            ty: CallType.Call,
            to: address(twin),
            value: 0,
            data: abi.encodeWithSelector(Twin.execute.selector, call)
        });

        // Test contract is the authorized caller
        twin.execute(selfCall);

        assertEq(mockTarget.value(), 123);
    }

    function test_execute_revertsOnUnauthorizedCaller_withExpectRevert() public {
        Call memory call = Call({
            ty: CallType.Call,
            to: address(mockTarget),
            value: 0,
            data: abi.encodeWithSelector(MockTarget.setValue.selector, 42)
        });

        vm.expectRevert(Twin.Unauthorized.selector);
        vm.prank(unauthorized);
        twin.execute(call);
    }

    //////////////////////////////////////////////////////////////
    ///              Execute Call Type Tests                   ///
    //////////////////////////////////////////////////////////////

    function test_execute_regularCall_success() public {
        Call memory call = Call({
            ty: CallType.Call,
            to: address(mockTarget),
            value: 0,
            data: abi.encodeWithSelector(MockTarget.setValue.selector, 999)
        });

        twin.execute(call);

        assertEq(mockTarget.value(), 999);
    }

    function test_execute_regularCall_withValue() public {
        uint256 initialBalance = address(mockTarget).balance;
        uint256 sendValue = 1 ether;

        Call memory call = Call({
            ty: CallType.Call,
            to: address(mockTarget),
            value: uint128(sendValue),
            data: abi.encodeWithSelector(MockTarget.setValue.selector, 555)
        });

        twin.execute(call);

        assertEq(mockTarget.value(), 555);
        assertEq(address(mockTarget).balance, initialBalance + sendValue);
    }

    function test_execute_regularCall_revertsOnTargetRevert() public {
        Call memory call = Call({
            ty: CallType.Call,
            to: address(mockRevertingTarget),
            value: 0,
            data: abi.encodeWithSelector(MockRevertingTarget.alwaysReverts.selector)
        });

        vm.expectRevert();
        twin.execute(call);
    }

    function test_execute_delegateCall_success() public {
        // Deploy a simple contract that sets a storage slot
        MockDelegateTarget delegateTarget = new MockDelegateTarget();

        Call memory call = Call({
            ty: CallType.DelegateCall,
            to: address(delegateTarget),
            value: 0,
            data: abi.encodeWithSelector(MockDelegateTarget.setStorageSlot.selector, 42)
        });

        twin.execute(call);

        // Check that the storage was set in the Twin contract's context
        bytes32 slot = vm.load(address(twin), bytes32(uint256(0)));
        assertEq(uint256(slot), 42);
    }

    function test_execute_delegateCall_revertsWithValue() public {
        MockDelegateTarget delegateTarget = new MockDelegateTarget();

        Call memory call = Call({
            ty: CallType.DelegateCall,
            to: address(delegateTarget),
            value: 1, // This should cause a revert
            data: abi.encodeWithSelector(MockDelegateTarget.setStorageSlot.selector, 42)
        });

        vm.expectRevert(CallLib.DelegateCallCannotHaveValue.selector);
        twin.execute(call);
    }

    function test_execute_create_success() public {
        // Simple contract bytecode: empty contract that compiles successfully
        bytes memory bytecode =
            hex"6080604052348015600f57600080fd5b50603f80601d6000396000f3fe6080604052600080fdfea26469706673582212200000000000000000000000000000000000000000000000000000000000000000000064736f6c63430008000033";

        Call memory call = Call({
            ty: CallType.Create,
            to: address(0), // Not used for CREATE
            value: 0,
            data: bytecode
        });

        twin.execute(call);

        // If we reach here, the CREATE was successful
        assertTrue(true);
    }

    function test_execute_create2_success() public {
        bytes32 salt = keccak256("test_salt");
        bytes memory bytecode =
            hex"6080604052348015600f57600080fd5b50603f80601d6000396000f3fe6080604052600080fdfea26469706673582212200000000000000000000000000000000000000000000000000000000000000000000064736f6c63430008000033";

        Call memory call = Call({
            ty: CallType.Create2,
            to: address(0), // Not used for CREATE2
            value: 0,
            data: abi.encode(salt, bytecode)
        });

        twin.execute(call);

        // If we reach here, the CREATE2 was successful
        assertTrue(true);
    }

    //////////////////////////////////////////////////////////////
    ///                   Edge Case Tests                      ///
    //////////////////////////////////////////////////////////////

    function test_execute_withMaxValue() public {
        // Test with maximum uint128 value
        Call memory call = Call({
            ty: CallType.Call,
            to: address(mockTarget),
            value: type(uint128).max,
            data: abi.encodeWithSelector(MockTarget.setValue.selector, 1)
        });

        // This should revert due to insufficient balance
        vm.expectRevert();
        twin.execute(call);
    }

    function test_execute_withEmptyData() public {
        Call memory call = Call({ty: CallType.Call, to: address(mockTarget), value: 0, data: ""});

        twin.execute(call);

        // Should succeed (calls fallback/receive)
        assertTrue(true);
    }

    function test_execute_toNonContract() public {
        address nonContract = makeAddr("nonContract");

        Call memory call = Call({ty: CallType.Call, to: nonContract, value: 1 ether, data: ""});

        twin.execute(call);

        assertEq(nonContract.balance, 1 ether);
    }

    //////////////////////////////////////////////////////////////
    ///                 Gas Estimation Tests                   ///
    //////////////////////////////////////////////////////////////

    function test_execute_gasUsage() public {
        Call memory call = Call({
            ty: CallType.Call,
            to: address(mockTarget),
            value: 0,
            data: abi.encodeWithSelector(MockTarget.setValue.selector, 42)
        });

        uint256 gasBefore = gasleft();
        twin.execute(call);
        uint256 gasUsed = gasBefore - gasleft();

        // Should use reasonable amount of gas (this is just a sanity check)
        assertLt(gasUsed, 100000);
    }

    //////////////////////////////////////////////////////////////
    ///                 Fuzz Tests                             ///
    //////////////////////////////////////////////////////////////

    function testFuzz_execute_regularCall_withDifferentValues(uint128 value, uint256 setValue) public {
        vm.assume(value <= address(twin).balance);

        Call memory call = Call({
            ty: CallType.Call,
            to: address(mockTarget),
            value: value,
            data: abi.encodeWithSelector(MockTarget.setValue.selector, setValue)
        });

        uint256 initialBalance = address(mockTarget).balance;

        twin.execute(call);

        assertEq(mockTarget.value(), setValue);
        assertEq(address(mockTarget).balance, initialBalance + value);
    }

    function testFuzz_constructor_alwaysSetsToBridgeToDeployer(address) public {
        // Regardless of any input, constructor always sets BRIDGE to msg.sender
        Twin testTwin = new Twin();
        assertEq(testTwin.BRIDGE(), address(this));
    }
}

//////////////////////////////////////////////////////////////
///                    Mock Contracts                      ///
//////////////////////////////////////////////////////////////

contract MockTarget {
    uint256 public value;

    event MockEvent(uint256 value);

    receive() external payable {}

    function setValue(uint256 _value) external payable {
        value = _value;
        emit MockEvent(_value);
    }
}

contract MockRevertingTarget {
    function alwaysReverts() external pure {
        revert("Always reverts");
    }
}

contract MockDelegateTarget {
    function setStorageSlot(uint256 _value) external {
        assembly {
            sstore(0, _value)
        }
    }
}
