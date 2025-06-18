// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {LibClone} from "solady/utils/LibClone.sol";
import {ReentrancyGuardTransient} from "solady/utils/ReentrancyGuardTransient.sol";

import {Twin} from "./Twin.sol";
import {Call, CallLib} from "./libraries/CallLib.sol";

/// @title Portal
///
/// @notice The Portal enables sending calls from Solana to Base.
///
/// @dev Calls sent from Solana to Base are relayed via a Twin contract that is specific per Solana sender pubkey.
contract Portal is ReentrancyGuardTransient {
    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted whenever a call is successfully relayed and executed on Base.
    ///
    /// @param callHash Keccak256 hash of the call that was successfully relayed.
    event RelayedCall(bytes32 indexed callHash);

    /// @notice Emitted whenever a call fails to be relayed on Base.
    ///
    /// @param callHash Keccak256 hash of the call that failed to be relayed.
    event FailedRelayedCall(bytes32 indexed callHash);

    //////////////////////////////////////////////////////////////
    ///                       Errors                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Thrown when doing gas estimation and the call's gas left is insufficient to cover the `minGas` plus the
    ///         `reservedGas`.
    error EstimationInsufficientGas();

    /// @notice Thrown when doing gas estimation and the call fails to be relayed on Base.
    error EstimationFailedRelayedCall();

    /// @notice Thrown when the call value is incorrect.
    error IncorrectMsgValue();

    /// @notice Thrown when the call is already failed.
    error CallAlreadyFailed();

    /// @notice Thrown when the ISM verification fails.
    error ISMVerificationFailed();

    /// @notice Thrown when the call is not already failed.
    error CallNotAlreadyFailed();

    /// @notice Thrown when the call is already relayed.
    error CallAlreadyRelayed();

    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice Special address to be used as the tx origin for gas estimation calls in the
    ///         OptimismPortal and CrossDomainMessenger calls. You only need to use this address if
    ///         the minimum gas limit specified by the user is not actually enough to execute the
    ///         given message and you're attempting to estimate the actual necessary gas limit. We
    ///         use address(1) because it's the ecrecover precompile and therefore guaranteed to
    ///         never have any code on any EVM chain.
    address public constant ESTIMATION_ADDRESS = address(1);

    // TODO: Re-estimate the constants.
    /// @notice Gas reserved for the execution logic between the `_hasMinGas` check and the actual Twin contract
    ///         execution in `relayCall`.
    uint64 public constant RELAY_GAS_CHECK_BUFFER = 5_000;

    /// @notice Gas reserved for finalizing the execution of `relayCall` after the safe call.
    uint64 public constant POST_EXECUTION_RESERVED_GAS = 40_000;

    /// @notice Address of the trusted relayer.
    address public immutable TRUSTED_RELAYER;

    /// @notice Address of the Twin beacon.
    address public immutable TWIN_BEACON;

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Mapping of call hashes to boolean receipt values indicating successful execution. A call will only be
    ///         present in this mapping if it has successfully been relayed on this chain, and therefore cannot be
    ///         relayed again.
    mapping(bytes32 callHash => bool succeeded) public successfulCalls;

    /// @notice Mapping of call hashes to boolean values indicating failed execution attempts. A call will be present in
    ///         this mapping if and only if it has failed to execute at least once. Successfully executed calls on first
    ///         attempt won't appear here.
    mapping(bytes32 callHash => bool failed) public failedCalls;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Constructs the Portal contract with immutable references.
    ///
    constructor(address trustedRelayer_, address twinBeacon_) {
        TRUSTED_RELAYER = trustedRelayer_;
        TWIN_BEACON = twinBeacon_;
    }

    /// @notice Relays calls via the sender's Twin contract.
    ///
    /// @param nonce Unique nonce associated with the calls batch.
    /// @param sender Solana sender pubkey.
    /// @param value Value that is forwarded to the Solana sender's Twin contract.
    /// @param minGasLimit Minimum amount of gas that is forwarded to the Solana sender's Twin contract.
    /// @param call Encoded call to send to the Solana sender's Twin contract.
    /// @param ismData Encoded ISM data used to verify the call.
    function relayCall(
        uint256 nonce,
        bytes32 sender,
        uint256 value,
        uint256 minGasLimit,
        bytes calldata call,
        bytes calldata ismData
    ) external payable nonReentrant {
        bytes32 callHash = keccak256(abi.encode(nonce, sender, value, minGasLimit, call));

        // Check that the call can be relayed.
        if (_isTrustedRelayer()) {
            require(msg.value == value, IncorrectMsgValue());
            require(!failedCalls[callHash], CallAlreadyFailed());
            _ismVerify({call: call, ismData: ismData});
        } else {
            require(msg.value == 0, IncorrectMsgValue());
            require(failedCalls[callHash], CallNotAlreadyFailed());
        }

        require(!successfulCalls[callHash], CallAlreadyRelayed());

        // Get the Twin contract.
        // NOTE: This will deploy the Twin contract behind a beacon proxy if it doesn't exist already.
        uint256 gas = gasleft();
        uint256 gasUsedForDeployment;
        (bool alreadyDeployed, address twinAddress) =
            LibClone.createDeterministicERC1967BeaconProxy({beacon: TWIN_BEACON, salt: sender});

        // Initialize if needed and deduct gas used.
        Twin twin = Twin(payable(twinAddress));
        if (!alreadyDeployed) {
            twin.initialize(sender);
            gasUsedForDeployment = gas - gasleft();
        }

        // Ensures sufficient gas for Twin contract execution and cleanup.
        // NOTE: Adds deployment gas to reserved gas instead of subtracting from minGasLimit to prevent underflow.
        if (
            !_hasMinGas({
                minGas: minGasLimit,
                reservedGas: gasUsedForDeployment + POST_EXECUTION_RESERVED_GAS + RELAY_GAS_CHECK_BUFFER
            })
        ) {
            failedCalls[callHash] = true;
            emit FailedRelayedCall(callHash);

            // Revert for gas estimation.
            if (tx.origin == ESTIMATION_ADDRESS) {
                revert EstimationInsufficientGas();
            }

            return;
        }

        // Relay the calls via the Twin contract.
        try twin.executeBatch{gas: gasleft() - POST_EXECUTION_RESERVED_GAS, value: value}(call) {
            successfulCalls[callHash] = true;
            emit RelayedCall(callHash);
        } catch {
            failedCalls[callHash] = true;

            // Revert for gas estimation.
            if (tx.origin == ESTIMATION_ADDRESS) {
                revert EstimationFailedRelayedCall();
            }

            emit FailedRelayedCall(callHash);
        }
    }

    /// @notice Deploys a Twin contract and initializes it.
    ///
    /// @dev Not really expected to be called directly (as the relayCall function will deploy the Twin
    ///      contract if needed), but can potentially unblock failing calls.
    ///
    /// @param sender Solana sender pubkey.
    ///
    /// @return twinAddress Address of the deployed Twin contract.
    function deployAndInitializeTwin(bytes32 sender) external payable returns (address twinAddress) {
        twinAddress =
            LibClone.deployDeterministicERC1967BeaconProxy({beacon: TWIN_BEACON, salt: sender, value: msg.value});

        Twin(payable(twinAddress)).initialize(sender);
    }

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions                ///
    //////////////////////////////////////////////////////////////

    /// @inheritdoc ReentrancyGuardTransient
    ///
    /// @dev We know Base mainnet supports transient storage.
    function _useTransientReentrancyGuardOnlyOnMainnet() internal pure override returns (bool) {
        return false;
    }

    //////////////////////////////////////////////////////////////
    ///                       Private Functions                ///
    //////////////////////////////////////////////////////////////

    /// @notice Checks whether the `msg.sender` is the trusted relayer.
    ///
    /// @return `true` if the `msg.sender` is the trusted relayer, `false` otherwise.
    function _isTrustedRelayer() private view returns (bool) {
        return msg.sender == TRUSTED_RELAYER;
    }

    /// @notice Checks whether the call's gas left is sufficient to cover the `minGas` plus the `reservedGas`.
    ///
    /// @dev Copied from:
    ///      https://github.com/ethereum-optimism/optimism/blob/4e9ef1aeffd2afd7a2378e2dc5efffa71f04207d/packages/contracts-bedrock/src/libraries/SafeCall.sol#L100
    ///
    /// @dev !!!!! FOOTGUN ALERT !!!!!
    ///      1.) The 40_000 base buffer is to account for the worst case of the dynamic cost of the
    ///          `CALL` opcode's `address_access_cost`, `positive_value_cost`, and
    ///          `value_to_empty_account_cost` factors with an added buffer of 5,700 gas. It is
    ///          still possible to self-rekt by initiating a withdrawal with a minimum gas limit
    ///          that does not account for the `memory_expansion_cost` & `code_execution_cost`
    ///          factors of the dynamic cost of the `CALL` opcode.
    ///      2.) This function should *directly* precede the external call if possible. There is an
    ///          added buffer to account for gas consumed between this check and the call, but it
    ///          is only 5,700 gas.
    ///      3.) Because EIP-150 ensures that a maximum of 63/64ths of the remaining gas in the call
    ///          frame may be passed to a subcontext, we need to ensure that the gas will not be
    ///          truncated.
    ///      4.) Use wisely. This function is not a silver bullet.
    ///
    /// @param minGas Minimum amount of gas that is forwarded to the Solana sender's Twin contract.
    /// @param reservedGas Amount of gas that is reserved for the caller after the execution of the target context.
    ///
    /// @return hasMinGas `true` if there is enough gas remaining to safely supply `minGas` to the target
    ///         context as well as reserve `reservedGas` for the caller after the execution of
    ///         the target context.
    function _hasMinGas(uint256 minGas, uint256 reservedGas) private view returns (bool hasMinGas) {
        assembly {
            // Equation: gas × 63 ≥ minGas × 64 + 63(40_000 + reservedGas)
            hasMinGas := iszero(lt(mul(gas(), 63), add(mul(minGas, 64), mul(add(40000, reservedGas), 63))))
        }
    }

    /// @notice Checks whether the ISM verification is successful.
    ///
    /// @dev TODO: Plug some ISM verification here.
    function _ismVerify(bytes calldata call, bytes calldata ismData) private pure {
        // TODO: Plug some ISM verification here.
        require(true, ISMVerificationFailed());
    }
}
