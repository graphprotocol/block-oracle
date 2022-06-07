// 2. Register(A) then SetBlocks(A) (same payload)

// TODO: we must put the two messages inside the same payload, but we
//       haven't implemented that int he oracle encoder yet. So, for
//       now, this example is the same as 03.

local messages = import 'messages.libsonnet';

[
  messages.add_networks(["A"]),
  messages.set_block_numbers([15]),
]
