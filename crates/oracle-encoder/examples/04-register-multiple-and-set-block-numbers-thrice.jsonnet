// 4. Register(A,B,C,D) then SetBlocks(A,B,C,D) x3 (same payload)

// TODO: we must put those messages inside the same payload, but we
//       haven't implemented that in the oracle-encoder yet.

local messages = import 'messages.libsonnet';

[
  messages.add_networks(["A", "B", "C", "D"]),
  messages.set_block_numbers([1,  2,  3,  4]),
  messages.set_block_numbers([5,  6,  7,  8]),
  messages.set_block_numbers([9, 10, 11, 12]),
]
