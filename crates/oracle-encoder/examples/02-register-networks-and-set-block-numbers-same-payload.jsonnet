// 2. Register(A) then SetBlocks(A) in the same payload

local messages = import 'messages.libsonnet';

[
  [  messages.add_networks(["A"]),
     messages.set_block_numbers([15]),
  ]
]
