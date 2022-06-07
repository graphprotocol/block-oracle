// 5. Register(A,B,C,D) then SetBlocks(A,B,C,D) (same payload), then
//    Unregister(B) and SetBlocks(A,D,C) (both on a different payload
//    than the original)

// TODO: we must group those messages into two payloads, but we
//       haven't implemented that in the oracle-encoder yet.

local messages = import 'messages.libsonnet';

[
  // TODO: Payload #1 begins here
  messages.add_networks(["A", "B", "C", "D"]),
  messages.set_block_numbers([1,  2,  3,  4]),

  // TODO: Payload #2 begins here
  messages.remove_networks([1]),
  messages.set_block_numbers([5, 6, 7]),
]
