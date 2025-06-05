// // SPDX-License-Identifier: MIT
// pragma solidity 0.8.28;

// import {Script} from "forge-std/Script.sol";
// import {stdJson} from "forge-std/StdJson.sol";
// import {console} from "forge-std/console.sol";
// import {ERC20} from "solady/tokens/ERC20.sol";

// import {TokenBridge} from "../../src/TokenBridge.sol";

// contract BridgeTokensToSolanaScript is Script {
//     using stdJson for string;

//     address public constant ETH_ADDRESS = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE;

//     address public immutable LOCAL_TOKEN = vm.envAddress("LOCAL_TOKEN");
//     bytes32 public immutable REMOTE_TOKEN = vm.envBytes32("REMOTE_TOKEN");
//     bytes32 public immutable TO = vm.envBytes32("TO");
//     uint64 public immutable AMOUNT = uint64(vm.envUint("AMOUNT"));
//     bytes public extraData = bytes("Dummy extra data");

//     TokenBridge public bridge;

//     function setUp() public {
//         Chain memory chain = getChain(block.chainid);
//         console.log("Creating token on chain: %s", chain.name);

//         string memory rootPath = vm.projectRoot();
//         string memory path = string.concat(rootPath, "/deployments/", chain.chainAlias, ".json");
//         address bridgeAddress = vm.readFile(path).readAddress(".TokenBridge");
//         bridge = TokenBridge(bridgeAddress);
//     }

//     function run() public payable {
//         vm.startBroadcast();
//         if (vm.envOr("NEEDS_APPROVAL", false)) {
//             ERC20(LOCAL_TOKEN).approve(address(bridge), AMOUNT);
//         }
//         uint256 value = LOCAL_TOKEN == ETH_ADDRESS ? AMOUNT : 0;
//         bridge.bridgeToken{value: value}(LOCAL_TOKEN, REMOTE_TOKEN, TO, AMOUNT, extraData);
//         vm.stopBroadcast();
//     }
// }
