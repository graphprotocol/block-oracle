use crate::{messages::*, NetworkIndex};

const PREAMBLE_BIT_LENGTH: usize = 8;
const TAG_BIT_LENGTH: usize = 4;
const PREAMBLE_CAPACITY: usize = PREAMBLE_BIT_LENGTH / TAG_BIT_LENGTH;

pub fn serialize_messages(messages: &[CompressedMessage], bytes: &mut Vec<u8>) {
    let chunks = messages.chunks(PREAMBLE_CAPACITY);
    for chunk in chunks {
        serialize_preamble(chunk, bytes);

        for message in chunk {
            serialize_message(message, bytes)
        }
    }
}

fn serialize_preamble(messages: &[CompressedMessage], bytes: &mut Vec<u8>) {
    assert!(messages.len() <= PREAMBLE_CAPACITY);

    let mut preamble = 0;
    for (i, msg) in messages.iter().enumerate() {
        preamble |= message_tag(msg) << (TAG_BIT_LENGTH * i);
    }

    bytes.push(preamble)
}

fn serialize_message(message: &CompressedMessage, bytes: &mut Vec<u8>) {
    match message {
        CompressedMessage::SetBlockNumbersForNextEpoch(compressed_block_numbers) => {
            serialize_set_block_numbers_for_next_block(compressed_block_numbers, bytes)
        }
        CompressedMessage::RegisterNetworks { add, remove } => {
            serialize_register_networks(add, remove, bytes)
        }
        CompressedMessage::UpdateVersion { version_number } => {
            serialize_u64(*version_number, bytes);
        }
        CompressedMessage::Reset => serialize_u64(0, bytes),
        CompressedMessage::CorrectEpochs { .. } => {
            todo!()
        }
        CompressedMessage::RegisterNetworksAndAliases { add, remove } => {
            serialize_register_networks_and_aliases(add, remove, bytes)
        }
        CompressedMessage::ChangePermissions {
            address,
            valid_through,
            permissions,
        } => serialize_change_permissions(address, *valid_through, permissions, bytes),
    }
}

fn serialize_set_block_numbers_for_next_block(
    block_numbers: &CompressedSetBlockNumbersForNextEpoch,
    bytes: &mut Vec<u8>,
) {
    match block_numbers {
        CompressedSetBlockNumbersForNextEpoch::Empty { count } => serialize_u64(*count, bytes),
        CompressedSetBlockNumbersForNextEpoch::NonEmpty {
            accelerations,
            root,
        } => {
            bytes.extend_from_slice(root);
            for acceleration in accelerations {
                serialize_i64(*acceleration, bytes);
            }
        }
    }
}

fn serialize_register_networks(add: &[String], remove: &[NetworkIndex], bytes: &mut Vec<u8>) {
    serialize_u64(remove.len() as u64, bytes);
    for id in remove {
        // TODO: Compression - could delta encode series here. Probably not worth it.
        serialize_u64(*id, bytes);
    }

    serialize_u64(add.len() as u64, bytes);
    for add in add {
        serialize_str(add, bytes);
    }
}

fn serialize_register_networks_and_aliases(
    add: &[(String, String)],
    remove: &[NetworkIndex],
    bytes: &mut Vec<u8>,
) {
    serialize_u64(remove.len() as u64, bytes);
    for id in remove {
        // TODO: Compression - could delta encode series here. Probably not worth it.
        serialize_u64(*id, bytes);
    }

    serialize_u64(add.len() as u64, bytes);
    for (add0, add1) in add {
        serialize_str(add0, bytes);
        serialize_str(add1, bytes);
    }
}

fn serialize_change_permissions(
    address: &[u8],
    valid_through: u64,
    permissions: &[String],
    bytes: &mut Vec<u8>,
) {
    bytes.extend_from_slice(address);
    serialize_u64(valid_through, bytes);
    serialize_u64(permissions.len() as u64, bytes);
    for permission in permissions {
        serialize_str(permission, bytes);
    }
}

fn serialize_str(value: &str, bytes: &mut Vec<u8>) {
    serialize_u64(value.len() as u64, bytes);
    bytes.extend_from_slice(value.as_bytes());
}

fn serialize_i64(value: i64, bytes: &mut Vec<u8>) {
    // Uses ZigZag encoding. See
    // <https://developers.google.com/protocol-buffers/docs/encoding#signed-ints>.
    let unsigned = (value << 1) ^ (value >> 63);

    serialize_u64(unsigned as u64, bytes);
}

fn serialize_u64(mut value: u64, bytes: &mut Vec<u8>) {
    // The number of meaningful bits in `value`.
    let num_bits_to_encode = 64 - value.leading_zeros();
    // The number of bytes that are needed to encode `value`. It is
    // calculated by finding the next multiple of 7 after `num_bits_to_encode`.
    // Range bounds are tricky and must be handled separately.
    let num_bytes = (num_bits_to_encode.clamp(1, 63) - 1) / 7 + 1;

    debug_assert!(num_bytes >= 1);
    debug_assert!(num_bytes <= 9);

    bytes.push((value << num_bytes) as u8 | (1 << (num_bytes - 1)) as u8);
    value >>= 8u32.saturating_sub(num_bytes);

    while value > 0 {
        bytes.push(value as u8);
        value >>= 8;
    }
}

fn message_tag(m: &CompressedMessage) -> u8 {
    match m {
        CompressedMessage::SetBlockNumbersForNextEpoch { .. } => 0u8,
        CompressedMessage::CorrectEpochs { .. } => 1,
        CompressedMessage::UpdateVersion { .. } => 2,
        CompressedMessage::RegisterNetworks { .. } => 3,
        CompressedMessage::ChangePermissions { .. } => 4,
        CompressedMessage::Reset => 5,
        CompressedMessage::RegisterNetworksAndAliases { .. } => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const U64_TESTS: &[(u64, &[u8])] = &[
        (0, &[1]),
        (23, &[47]),
        (9000, &[162, 140]),
        (1455594, &[84, 175, 177]),
        (109771541, &[88, 177, 175, 104]),
        (24345908991, &[240, 223, 34, 100, 181]),
        (1903269233213, &[96, 143, 240, 235, 200, 110]),
        (72057594037927935, &[128, 255, 255, 255, 255, 255, 255, 255]),
        (u64::MAX, &[0, 255, 255, 255, 255, 255, 255, 255, 255]),
    ];

    const ZIGZAG_TESTS: &[(u64, i64)] = &[
        (0, 0),
        (1, -1),
        (4294967294, 2147483647),
        (u64::MAX, i64::MIN),
    ];

    #[test]
    fn encode_u64() {
        for (value, expected) in U64_TESTS.iter() {
            let mut buf = Vec::new();
            serialize_u64(*value, &mut buf);
            assert_eq!(&buf[..], *expected);
        }
    }

    #[test]
    fn encode_i64() {
        for (unsigned, signed) in ZIGZAG_TESTS.iter() {
            let mut buf_u64 = Vec::new();
            serialize_u64(*unsigned as u64, &mut buf_u64);

            let mut buf_i64 = Vec::new();
            serialize_i64(*signed, &mut buf_i64);

            assert_eq!(&buf_i64[..], &buf_u64[..]);
        }
    }
}
