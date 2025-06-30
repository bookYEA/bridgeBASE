// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Ownable} from "solady/auth/Ownable.sol";

contract CounterValue is Ownable {
    uint256 public count;

    constructor() {
        _initializeOwner(msg.sender);
    }

    function increment() external payable {
        count++;
    }

    function withdraw() external onlyOwner {
        (bool success, ) = msg.sender.call{value: address(this).balance}("");
        require(success, "Call failed");
    }
}
