//SPDX-License-Identifier: MIT

pragma solidity >=0.8.0 <0.9.0;

import "@openzeppelin/contracts/access/Ownable.sol";

contract EpochOracle is Ownable {
    event NewEpochBlock(uint256 epoch, uint256 networkId, bytes32 blockHash);

    struct EpochBlockUpdate {
        // EIP-155 (chainid) https://github.com/ethereum/EIPs/blob/master/EIPS/eip-155.md
        uint16 networkId;
        bytes32 blockHash;
    }

    // Payload format
    // {
    //     length: 1 byte
    //     arrayOfUpdates: [{
    //        networkId: 2 bytes
    //        blockHash: 32 bytes
    //     }, ...]
    // }

    constructor(address _owner) {
        transferOwnership(_owner);
    }

    /// @notice Set multiple epoch blocks
    /// @dev Emits events with the updates
    function setEpochBlocks(
        uint256 _epoch,
        EpochBlockUpdate[] calldata _updates
    ) external {
        for (uint256 i = 0; i < _updates.length; i++) {
            uint256 networkId = _updates[i].networkId;
            emit NewEpochBlock(_epoch, networkId, _updates[i].blockHash);
        }
    }

    /// @notice Set multiple epoch blocks
    /// @dev Does not do execution just post payload
    function setEpochBlocksPayload(bytes calldata _payload) external {}
}
