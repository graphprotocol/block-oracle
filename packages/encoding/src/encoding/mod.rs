mod varint;

use crate::{messages::*, NetworkId};

pub fn decode_messages(bytes: &[u8]) -> Vec<CompressedMessage> {
    let preamble = bytes[0] >> 4;
    let mut messages = vec![];

    let tags: [u8; 4] = [
        (preamble >> 6) & 0b11,
        (preamble >> 4) & 0b11,
        (preamble >> 2) & 0b11,
        (preamble >> 0) & 0b11,
    ];

    let offset = &mut 1usize;
    for tag in tags {
        match tag {
            0 => {}
            1 => todo!(),
            2 => todo!(),
            3 => {
                let removed_count = decode_u64(bytes, offset).unwrap();
                let mut removed = Vec::with_capacity(removed_count as usize);
                for _ in 0..removed_count {
                    let network_id = decode_u64(bytes, offset).unwrap();
                    removed.push(network_id);
                }

                let added_count = decode_u64(bytes, offset).unwrap();
                let mut added = Vec::with_capacity(added_count as usize);
                for _ in 0..added_count {
                    let network_name = decode_str(bytes, offset).unwrap();
                    added.push(network_name);
                }
            }
            _ => panic!("Invalid tag"),
        }
    }

    messages
}

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
            CompressedMessage::RegisterNetworks { .. } => 3,
        }
    }

    let mut preamble = 0;
    for (i, message) in messages.iter().enumerate() {
        preamble |= tag(message) << (i * 2);
    }

    bytes.push(preamble)
}

fn encode_message(message: &CompressedMessage, bytes: &mut Vec<u8>) {
    match message {
        CompressedMessage::SetBlockNumbersForNextEpoch(compressed_block_numbers) => {
            encode_set_block_numbers_for_next_block(compressed_block_numbers, bytes)
        }
        CompressedMessage::RegisterNetworks { add, remove } => {
            encode_register_networks(add, remove, bytes)
        }

        _ => todo!(),
    }
}

fn encode_set_block_numbers_for_next_block(
    block_numbers: &CompressedSetBlockNumbersForNextEpoch,
    bytes: &mut Vec<u8>,
) {
    match block_numbers {
        CompressedSetBlockNumbersForNextEpoch::Empty { count } => encode_u64(*count, bytes),
        CompressedSetBlockNumbersForNextEpoch::NonEmpty {
            accelerations,
            root,
        } => {
            bytes.extend_from_slice(root);
            for acceleration in accelerations {
                encode_i64(*acceleration, bytes);
            }
        }
    }
}

fn encode_register_networks(add: &[String], remove: &[NetworkId], bytes: &mut Vec<u8>) {
    // Write count for remove, followed by network ids
    encode_u64(remove.len() as u64, bytes);
    for remove in remove {
        // TODO: Compression - could delta encode series here. Probably not worth it.
        encode_u64(*remove, bytes);
    }

    encode_u64(add.len() as u64, bytes);
    for add in add {
        encode_str(add, bytes);
    }
}

fn encode_str(value: &str, bytes: &mut Vec<u8>) {
    encode_u64(value.len() as u64, bytes);
    bytes.extend_from_slice(value.as_bytes());
}

fn decode_u64(bytes: &[u8], offset: &mut usize) -> Option<u64> {
    varint::decode_prefix_varint(bytes, offset)
}

fn decode_str<'a>(bytes: &'a [u8], offset: &mut usize) -> Option<&'a str> {
    let length = decode_u64(bytes, offset)? as usize;
    std::str::from_utf8(bytes.get(*offset..*offset + length)?).ok()
}

fn encode_u64(value: u64, bytes: &mut Vec<u8>) {
    varint::encode_prefix_varint(value, bytes);
}

fn encode_i64(value: i64, bytes: &mut Vec<u8>) {
    encode_u64(zigzag::ZigZag::encode(value), bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
}
