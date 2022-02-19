pragma solidity >=0.8.0 <0.9.0;
//SPDX-License-Identifier: MIT

import "hardhat/console.sol";
// import "@openzeppelin/contracts/access/Ownable.sol";
// https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/access/Ownable.sol

contract EpochOracle {

  event newEpochBlock(uint256 epoch, uint256 networkId, string blockHash);

  constructor() {
    // what should we do on deploy?
  }

  struct epochBlockUpdate {
          uint256 networkId;
          string blockHash;
      }

  // set multiple epoch blocks
  function setEpochBlocks(uint256 _epoch, epochBlockUpdate[] calldata _updates) public {

      for (uint256 i = 0; i < _updates.length; i++) {
        uint256 networkId = _updates[i].networkId;
        emit newEpochBlock(_epoch, networkId, _updates[i].blockHash);
        }
    }
}
