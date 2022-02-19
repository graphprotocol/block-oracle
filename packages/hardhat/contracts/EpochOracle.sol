//SPDX-License-Identifier: MIT

pragma solidity >=0.8.0 <0.9.0;

import "@openzeppelin/contracts/access/Ownable.sol";

contract EpochOracle is Ownable {
    // TODO: do we know if block hash could always fit in 32 bytes?
    event NewEpochBlock(uint256 epoch, uint256 networkId, bytes32 blockHash);

    struct EpochBlockUpdate {
        // EIP-155 (chainid) https://github.com/ethereum/EIPs/blob/master/EIPS/eip-155.md
        uint16 networkId;
        bytes32 blockHash;
    }

    constructor(address _owner) {
        transferOwnership(_owner);
    }

    // set multiple epoch blocks
    function setEpochBlocks(
        uint256 _epoch,
        EpochBlockUpdate[] calldata _updates
    ) public {
        for (uint256 i = 0; i < _updates.length; i++) {
            uint256 networkId = _updates[i].networkId;
            emit NewEpochBlock(_epoch, networkId, _updates[i].blockHash);
        }
    }
}
