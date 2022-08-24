// Functions for creating the JSON for each message type.

local register_tag = "RegisterNetworks";
local set_blocks_tag = "SetBlockNumbersForNextEpoch";
local change_ownership_tag = "ChangeOwnership";
local merkle_root = "0x66ebb0afd80c906e2b0564e921c3feefa9a5ecb71e98e3c7b7e661515e87dc49";

{
  register_networks(add, remove):: {
    message: "RegisterNetworks",
    add: add,
    remove: remove,
  },

  add_networks(networks):: self.register_networks(networks,[]),

  remove_networks(networks):: self.register_networks([], networks),

  empty_block_numbers(count):: {
    message: set_blocks_tag,
    count: count,
  },

  set_block_numbers(accelerations):: {
    message: set_blocks_tag,
    merkleRoot: merkle_root,
    accelerations: accelerations,
  },

  change_ownership(new_owner):: {
    message: change_ownership_tag,
    newOwnerAddress: new_owner,
  },
}
