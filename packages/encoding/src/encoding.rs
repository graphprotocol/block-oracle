use crate::{messages::*, Bytes32};

pub fn encode_messages(messages: &[CompressedMessage]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let message_blocks = messages.chunks(4);
    for message_block in message_blocks {
        encode_preamble(message_block, &mut bytes);

        for message in message_block {
            encode_message(message, &mut bytes)
        }
    }
    bytes
}

fn encode_preamble(messages: &[CompressedMessage], bytes: &mut Vec<u8>) {
    assert!(messages.len() > 0);
    assert!(messages.len() < 5);

    fn tag(message: &CompressedMessage) -> u8 {
        match message {
            CompressedMessage::SetBlockNumbersForNextEpoch { .. } => 0u8,
            CompressedMessage::CorrectEpochs => 1,
            CompressedMessage::UpdateVersion => 2,
            CompressedMessage::RegisterNetworks => 3,
        }
    }

    let mut preamble = 0;
    for (i, message) in messages.iter().enumerate() {
        preamble &= tag(message) << (i * 2);
    }

    bytes.push(preamble)
}

fn encode_message(message: &CompressedMessage, bytes: &mut Vec<u8>) {
    match message {
        CompressedMessage::SetBlockNumbersForNextEpoch {
            accelerations,
            root,
        } => encode_set_block_numbers_for_next_block(accelerations, root, bytes),
        _ => todo!(),
    }
}

fn encode_set_block_numbers_for_next_block(
    accelerations: &[i64],
    root: &Bytes32,
    bytes: &mut Vec<u8>,
) {
    bytes.extend_from_slice(root);
    for acceleration in accelerations {
        encode_i64(*acceleration, bytes);
    }
}

fn encode_u64(value: u64, bytes: &mut Vec<u8>) {
    crate::varint::encode_prefix_varint(value, bytes);
}

fn encode_i64(value: i64, bytes: &mut Vec<u8>) {
    encode_u64(zigzag::ZigZag::encode(value), bytes);
}
