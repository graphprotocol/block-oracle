// SPDX-License-Identifier: GPL-2.0-or-later

pragma solidity ^0.8.12;

import "@openzeppelin/contracts/access/Ownable.sol";

/// @title Data Vault contract is only used to store on-chain data, it does not
///        perform execution. On-chain client services can read the data
///        and decode the payload for different purposes.
contract DataVault is Ownable {
    /// @dev Fallback function, accepts any payload
    fallback() external {
        // no-op
    }
}
