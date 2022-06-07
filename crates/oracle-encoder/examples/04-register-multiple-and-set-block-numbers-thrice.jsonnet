// 4. Register(A,B,C,D) then SetBlocks(A,B,C,D) x3 in the same payload

local messages = import 'messages.libsonnet';

[
  [
    messages.add_networks(["A", "B", "C", "D"]),
    messages.set_block_numbers([1,  2,  3,  4]),
    messages.set_block_numbers([5,  6,  7,  8]),
    messages.set_block_numbers([9, 10, 11, 12]),
  ]
]
