// 1. Empty blocknums test with a particular count (SetBlocks(empty
//    with N amount of empty epochs))

local messages = import 'messages.libsonnet';

[
  messages.empty_block_numbers(100),
]
