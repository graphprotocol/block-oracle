// 5. Register(A,B,C,D) then SetBlocks(A,B,C,D) (same payload), then
//    Unregister(B,D) and SetBlocks(A,C) (both on a different payload
//    than the original)

local messages = import 'messages.libsonnet';

[
  [
    messages.add_networks(["A", "B", "C", "D"]),
    messages.set_block_numbers([1,  2,  3,  4]),
  ],
  [
    messages.remove_networks([1, 3]),
    messages.set_block_numbers([5, 6]),
  ]
]
