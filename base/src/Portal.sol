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

    /// @notice Thrown when the sender is not the entrypoint.
    error SenderIsNotEntrypoint();

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

    /// @notice Thrown when the call execution fails.
    error ExecutionFailed();

    /// @notice Thrown when the nonce is not incremental.
    error NonceNotIncremental();

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

    /// @notice Address of the trusted relayer.
    address public immutable TRUSTED_RELAYER;

    /// @notice Address of the Twin beacon.
    address public immutable TWIN_BEACON;

    /// @notice Additional gas buffer reserved to ensure the correct execution of the `relayCallEntrypoint` function.
    uint64 private constant _RELAY_CALL_GAS_BUFFER = 40_000;

    /// @notice Gas reserved for the dynamic parts of the `CALL` opcode.
    uint64 private constant _RELAY_CALL_OVERHEAD_GAS = 40_000;

    /// @notice The last nonce used for a call.
    uint64 public lastNonce;

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

    /// @notice Mapping of Solana owner pubkeys to their Twin contract addresses.
    mapping(bytes32 owner => address twinAddress) public twins;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Constructs the Portal contract with immutable references.
    ///
    constructor(address trustedRelayer, address twinBeacon) {
        TRUSTED_RELAYER = trustedRelayer;
        TWIN_BEACON = twinBeacon;
    }

    /// @notice Entrypoint for relaying a call from Solana to Base.
    ///
    /// @param nonce Nonce of the call.
    /// @param sender Solana sender pubkey.
    /// @param call Encoded call to send to the Solana sender's Twin contract.
    /// @param ismData Encoded ISM data used to verify the call.
    function relayCallEntrypoint(uint64 nonce, bytes32 sender, Call calldata call, bytes calldata ismData) external {
        // INVARIANTs for the relay to work properly:
        //      1. The `gasLimit` set on the Solana side must be sufficient to cover the _RELAY_CALL_GAS_BUFFER (+ the
        //         minimum gas to cover the calldata size + tx base gas cost).
        //      2. On first call, the relayer MUST provide a `tx.gas` = `call.gasLimit` + ISM_BUFFER, where ISM_BUFFER
        //         is the gas required to cover the ISM verification (which will be removed once enshrined).
        //      3. In case of replay, the user must provide a `tx.gas` >= `call.gasLimit`.

        bool isTrustedRelayer = msg.sender == TRUSTED_RELAYER;
        if (isTrustedRelayer) {
            _ismVerify({call: call, ismData: ismData});
        }

        bytes32 callHash = keccak256(abi.encode(nonce, sender, call.ty, call.to, call.value, call.data));

        // Ensures sufficient gas for execution and cleanup.
        if (!_hasMinGas({minGas: 0, reservedGas: _RELAY_CALL_GAS_BUFFER})) {
            failedCalls[callHash] = true;
            emit FailedRelayedCall(callHash);

            // Revert for gas estimation.
            if (tx.origin == ESTIMATION_ADDRESS) {
                revert EstimationInsufficientGas();
            }

            return;
        }

        try this.relayCall{gas: gasleft() - _RELAY_CALL_GAS_BUFFER}({
            nonce: nonce,
            sender: sender,
            call: call,
            isTrustedRelayer: isTrustedRelayer,
            callHash: callHash
        }) {
            // Register the call as successful.
            successfulCalls[callHash] = true;
            if (failedCalls[callHash]) {
                delete failedCalls[callHash];
            }

            emit RelayedCall(callHash);
        } catch (bytes memory reason) {
            // Some mandatory invariant has been violated, we should revert.
            if (bytes4(reason) != ExecutionFailed.selector) {
                assembly {
                    revert(add(reason, 32), mload(reason))
                }
            }

            // Otherwise the user call itself reverted, register the call as failed.
            failedCalls[callHash] = true;
            emit FailedRelayedCall(callHash);
        }
    }

    /// @notice Relays a call via the sender's Twin contract.
    ///
    /// @dev This function can only be called by the entrypoint and is here to allow for safe gas accounting.
    ///
    /// @param nonce Nonce of the call.
    /// @param sender Solana sender pubkey.
    /// @param call Encoded call to send to the Solana sender's Twin contract.
    /// @param isTrustedRelayer Whether the relayer is trusted.
    /// @param callHash Keccak256 hash of the call that was successfully relayed.
    function relayCall(uint64 nonce, bytes32 sender, Call calldata call, bool isTrustedRelayer, bytes32 callHash)
        external
    {
        // Check that the caller is the entrypoint.
        require(msg.sender == address(this), SenderIsNotEntrypoint());

        // Check that the relay is allowed.
        if (isTrustedRelayer) {
            require(nonce == lastNonce + 1, NonceNotIncremental());
            lastNonce = nonce;

            require(!failedCalls[callHash], CallAlreadyFailed());
        } else {
            require(failedCalls[callHash], CallNotAlreadyFailed());
        }

        require(!successfulCalls[callHash], CallAlreadyRelayed());

        // Get (and deploy if needed) the Twin contract.
        address twinAddress = twins[sender];
        if (twinAddress == address(0)) {
            twinAddress = LibClone.deployDeterministicERC1967BeaconProxy({beacon: TWIN_BEACON, salt: sender});
            twins[sender] = twinAddress;
        }

        // Execute the call via the Twin contract.
        try Twin(payable(twinAddress)).execute{value: call.value}(call) {}
        catch {
            revert ExecutionFailed();
        }
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
    ///      1.) The _RELAY_CALL_OVERHEAD_GAS base buffer is to account for the worst case of the dynamic cost of the
    ///          `CALL` opcode's `address_access_cost`, `positive_value_cost`, and `value_to_empty_account_cost` factors
    ///          with an added buffer of 5,700 gas. It is still possible to self-rekt by initiating a withdrawal with a
    ///          minimum gas limit that does not account for the `memory_expansion_cost` & `code_execution_cost` factors
    ///          of the dynamic cost of the `CALL` opcode.
    ///      2.) This function should *directly* precede the external call if possible. There is an added buffer to
    ///          account for gas consumed between this check and the call, but it is only 5,700 gas.
    ///      3.) Because EIP-150 ensures that a maximum of 63/64ths of the remaining gas in the call frame may be passed
    ///          to a subcontext, we need to ensure that the gas will not be truncated.
    ///      4.) Use wisely. This function is not a silver bullet.
    ///
    /// @param minGas Minimum amount of gas that is forwarded to the Solana sender's Twin contract.
    /// @param reservedGas Amount of gas that is reserved for the caller after the execution of the target context.
    ///
    /// @return hasMinGas `true` if there is enough gas remaining to safely supply `minGas` to the target
    ///                    context as well as reserve `reservedGas` for the caller after the execution of
    ///                    the target context.
    function _hasMinGas(uint256 minGas, uint256 reservedGas) private view returns (bool hasMinGas) {
        assembly {
            // Equation: gas × 63 - 63(40_000 + reservedGas) ≥ minGas × 64
            //       =>  gas × 63 ≥ minGas × 64 + 63(40_000 + reservedGas)
            hasMinGas :=
                iszero(lt(mul(gas(), 63), add(mul(minGas, 64), mul(add(_RELAY_CALL_OVERHEAD_GAS, reservedGas), 63))))
        }
    }

    /// @notice Checks whether the ISM verification is successful.
    ///
    /// @param call Encoded call to send to the Solana sender's Twin contract.
    /// @param ismData Encoded ISM data used to verify the call.
    function _ismVerify(Call calldata call, bytes calldata ismData) private pure {
        call; // Silence unused variable warning.
        ismData; // Silence unused variable warning.

        // TODO: Plug some ISM verification here.
        require(true, ISMVerificationFailed());
    }
}
