// 3. Register(A) then SetBlocks(A) (same payload)

local messages = import 'messages.libsonnet';

[
  messages.add_networks(["A"]),
  messages.set_block_numbers([15]),
]
