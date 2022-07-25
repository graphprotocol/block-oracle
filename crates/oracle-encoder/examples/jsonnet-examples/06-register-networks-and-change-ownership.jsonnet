// 2. Register(A) then ChangeOwnership in the same payload

local messages = import 'messages.libsonnet';

[
  [  messages.add_networks(["A"]),
     messages.change_ownership("0x90F8bf6A479f320ead074411a4B0e7944Ea8c9C1")
  ]
]
