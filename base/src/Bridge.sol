// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {LibClone} from "solady/utils/LibClone.sol";
import {ReentrancyGuardTransient} from "solady/utils/ReentrancyGuardTransient.sol";

import {Call, CallLib} from "./libraries/CallLib.sol";
import {MessageStorageLib} from "./libraries/MessageStorageLib.sol";
import {Pubkey} from "./libraries/SVMLib.sol";
import {TokenLib, TransferPayload} from "./libraries/TokenLib.sol";

import {Twin} from "./Twin.sol";

/// @title Portal
///
/// @notice The Portal enables sending calls from Solana to Base.
///
/// @dev Calls sent from Solana to Base are relayed via a Twin contract that is specific per Solana sender pubkey.
contract Bridge is ReentrancyGuardTransient {
    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted whenever a bridge payload is successfully relayed and executed.
    ///
    /// @param payloadHash Keccak256 hash of the payload that was successfully relayed.
    event BridgeFinalized(bytes32 indexed payloadHash);

    /// @notice Emitted whenever a bridge payload fails to be relayed.
    ///
    /// @param payloadHash Keccak256 hash of the payload that failed to be relayed.
    event BridgeFailed(bytes32 indexed payloadHash);

    //////////////////////////////////////////////////////////////
    ///                       Errors                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Thrown when the ISM verification fails.
    error ISMVerificationFailed();

    /// @notice Thrown when doing gas estimation and the call's gas left is insufficient to cover the `minGas` plus the
    ///         `reservedGas`.
    error EstimationInsufficientGas();

    /// @notice Thrown when the call execution fails.
    error ExecutionFailed();

    /// @notice Thrown when the sender is not the entrypoint.
    error SenderIsNotEntrypoint();

    /// @notice Thrown when the nonce is not incremental.
    error NonceNotIncremental();

    /// @notice Thrown when a bridge payload has already been finalized but is attempted to be relayed again.
    error BridgeAlreadyFinalized();

    /// @notice Thrown when a bridge payload has already failed and the relayer tries to relay it again.
    error BridgeAlreadyFailed();

    /// @notice Thrown when a bridge payload has not been marked as failed by the relayer but a user tries to relay it
    ///         manually.
    error BridgeNotAlreadyFailed();

    //////////////////////////////////////////////////////////////
    ///                       Structs                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Enum containing bridge types.
    enum BridgeType {
        Transfer,
        Call
    }

    /// @notice Struct containing the data for a bridge.
    ///
    /// @custom:field nonce Unique nonce for the bridge.
    /// @custom:field remoteSender The Solana sender's pubkey.
    /// @custom:field gasLimit The gas limit of the bridge.
    /// @custom:field bridgeType The type of bridge.
    /// @custom:field data The abi encoded data for the bridge.
    ///                    Transfer => abi.encode(TransferPayload)
    ///                    Call => abi.encode(Call)
    struct BridgePayload {
        uint64 nonce;
        Pubkey remoteSender;
        uint256 gasLimit;
        BridgeType bridgeType;
        bytes data;
    }

    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice Special address used as the tx origin for gas estimation calls.
    ///
    /// @dev You only need to use this address if the minimum gas limit specified by the user is not actually enough to
    ///      execute the given message and you're attempting to estimate the actual necessary gas limit. We use
    ///      address(1) because it's the ecrecover precompile and therefore guaranteed to never have any code on any EVM
    ///      chain.
    address public constant ESTIMATION_ADDRESS = address(1);

    /// @notice Address of the trusted relayer.
    address public immutable TRUSTED_RELAYER;

    /// @notice Address of the Twin beacon.
    address public immutable TWIN_BEACON;

    /// @notice Additional gas buffer reserved to ensure the correct execution of `finalizeBridgeEntrypoint`.
    uint64 private constant _FINALIZE_BRIDGE_GAS_BUFFER = 40_000;

    /// @notice Gas reserved for the dynamic parts of the `CALL` opcode.
    uint64 private constant _FINALIZE_BRIDGE_OVERHEAD_GAS = 40_000;

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Mapping of payload hashes to boolean values indicating successful execution. A payload will only be
    ///         present in this mapping if it has successfully been executed, and therefore cannot be executed again.
    mapping(bytes32 payloadHash => bool success) public successes;

    /// @notice Mapping of payload hashes to boolean values indicating failed execution attempts. A payload will be
    ///         present in this mapping if and only if it has failed to execute at least once. Successfully executed
    ///         payloads on first attempt won't appear here.
    mapping(bytes32 payloadHash => bool failure) public failures;

    /// @notice Mapping of Solana owner pubkeys to their Twin contract addresses.
    mapping(Pubkey owner => address twinAddress) public twins;

    /// @notice The last bridge payload's nonce that has been attempted to finalize by the trusted relayer.
    uint64 public lastNonce;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Constructs the Portal contract with immutable references.
    ///
    constructor(address trustedRelayer, address twinBeacon) {
        TRUSTED_RELAYER = trustedRelayer;
        TWIN_BEACON = twinBeacon;
    }

    // TODO: Better naming convention for Solana-to-Base vs Base-to-Solana messages.
    // TODO: Re-implement the initiateXXX functions here once the interface on Solana is defined.

    /// @notice Finalizes a bridge initiated from Solana.
    ///
    /// @param payloads The bridge payloads to finalize.
    /// @param ismData Encoded ISM data used to verify the bridge payloads.
    function finalizeBridgeEntrypoint(BridgePayload[] calldata payloads, bytes calldata ismData)
        external
        nonReentrant
    {
        bool isTrustedRelayer = msg.sender == TRUSTED_RELAYER;
        if (isTrustedRelayer) {
            _ismVerify({payloads: payloads, ismData: ismData});
        }

        for (uint256 i; i < payloads.length; i++) {
            BridgePayload memory payload = payloads[i];

            // NOTE: Intentionally not including the gas limit in the hash to allow for replays with higher gas limits.
            bytes32 payloadHash =
                keccak256(abi.encode(payload.nonce, payload.remoteSender, payload.bridgeType, payload.data));

            // Ensures sufficient gas for execution and cleanup.
            if (!_hasMinGas({minGas: payload.gasLimit, reservedGas: _FINALIZE_BRIDGE_GAS_BUFFER})) {
                failures[payloadHash] = true;
                emit BridgeFailed(payloadHash);

                // Revert for gas estimation.
                if (tx.origin == ESTIMATION_ADDRESS) {
                    revert EstimationInsufficientGas();
                }

                return;
            }

            try this.finalizeBridge{gas: gasleft() - _FINALIZE_BRIDGE_GAS_BUFFER}({
                payload: payload,
                payloadHash: payloadHash,
                isTrustedRelayer: isTrustedRelayer
            }) {
                // Register the call as successful.
                successes[payloadHash] = true;
                if (failures[payloadHash]) {
                    delete failures[payloadHash];
                }

                emit BridgeFinalized(payloadHash);
            } catch (bytes memory reason) {
                // Some mandatory invariant has been violated, we should revert.
                if (bytes4(reason) != ExecutionFailed.selector) {
                    assembly {
                        revert(add(reason, 32), mload(reason))
                    }
                }

                // Otherwise the user call itself reverted, register the call as failed.
                failures[payloadHash] = true;
                emit BridgeFailed(payloadHash);

                // Revert for gas estimation.
                if (tx.origin == ESTIMATION_ADDRESS) {
                    revert ExecutionFailed();
                }

                return;
            }
        }
    }

    /// @notice Finalizes a bridge initiated from Solana.
    ///
    /// @dev This function is called by the entrypoint.
    ///
    /// @param payload The bridge payload to finalize.
    /// @param payloadHash The hash of the bridge payload.
    /// @param isTrustedRelayer Whether the caller was the trusted relayer.
    function finalizeBridge(BridgePayload calldata payload, bytes32 payloadHash, bool isTrustedRelayer) external {
        // Check that the caller is the entrypoint.
        require(msg.sender == address(this), SenderIsNotEntrypoint());

        // Check that the payload has not already been finalized.
        require(!successes[payloadHash], BridgeAlreadyFinalized());

        // Check that the relay is allowed.
        if (isTrustedRelayer) {
            require(payload.nonce == lastNonce + 1, NonceNotIncremental());
            lastNonce = payload.nonce;

            require(!failures[payloadHash], BridgeAlreadyFailed());
        } else {
            require(failures[payloadHash], BridgeNotAlreadyFailed());
        }

        // Get (and deploy if needed) the Twin contract.
        address twinAddress = twins[payload.remoteSender];
        if (twinAddress == address(0)) {
            twinAddress = LibClone.deployDeterministicERC1967BeaconProxy({
                beacon: TWIN_BEACON,
                salt: Pubkey.unwrap(payload.remoteSender)
            });
            twins[payload.remoteSender] = twinAddress;
        }

        if (payload.bridgeType == BridgeType.Transfer) {
            TransferPayload memory transferPayload = abi.decode(payload.data, (TransferPayload));

            // TODO: Do I need to wrap this in a try/catch with ExecutionFailed?
            TokenLib.finalizeTransfer(transferPayload);
        } else if (payload.bridgeType == BridgeType.Call) {
            Call memory callPayload = abi.decode(payload.data, (Call));
            try Twin(payable(twinAddress)).execute{value: callPayload.value}(callPayload) {}
            catch {
                revert ExecutionFailed();
            }
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

    /// @notice Checks whether the call's gas left is sufficient to cover the `minGas` plus the `reservedGas`.
    ///
    /// @dev Copied from:
    ///      https://github.com/ethereum-optimism/optimism/blob/4e9ef1aeffd2afd7a2378e2dc5efffa71f04207d/packages/contracts-bedrock/src/libraries/SafeCall.sol#L100
    ///
    /// @dev !!!!! FOOTGUN ALERT !!!!!
    ///      1.) The _FINALIZE_BRIDGE_OVERHEAD_GAS base buffer is to account for the worst case of the dynamic cost of
    ///          the `CALL` opcode's `address_access_cost`, `positive_value_cost`, and `value_to_empty_account_cost`
    ///          factors with an added buffer of 5,700 gas. It is still possible to self-rekt by initiating a withdrawal
    ///          with a minimum gas limit that does not account for the `memory_expansion_cost` & `code_execution_cost`
    ///          factors of the dynamic cost of the `CALL` opcode.
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
                iszero(lt(mul(gas(), 63), add(mul(minGas, 64), mul(add(_FINALIZE_BRIDGE_OVERHEAD_GAS, reservedGas), 63))))
        }
    }

    /// @notice Checks whether the ISM verification is successful.
    ///
    /// @param payloads The payloads to verify.
    /// @param ismData Encoded ISM data used to verify the call.
    function _ismVerify(BridgePayload[] calldata payloads, bytes calldata ismData) private pure {
        payloads; // Silence unused variable warning.
        ismData; // Silence unused variable warning.

        // TODO: Plug some ISM verification here.
        require(true, ISMVerificationFailed());
    }
}
