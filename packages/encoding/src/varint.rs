// TODO: Split this off into a library. Current code copied from tree-buf

pub fn encode_prefix_varint(value: u64, into: &mut Vec<u8>) {
    if value < (1 << 7) {
        into.push((value << 1) as u8 | 1);
    } else if value < (1 << 14) {
        into.extend_from_slice(&[(value << 2) as u8 | (1 << 1), (value >> 6) as u8]);
    } else if value < (1 << 21) {
        into.extend_from_slice(&[
            (value << 3) as u8 | (1 << 2),
            (value >> 5) as u8,
            (value >> 13) as u8,
        ]);
    } else if value < (1 << 28) {
        into.extend_from_slice(&[
            (value << 4) as u8 | (1 << 3),
            (value >> 4) as u8,
            (value >> 12) as u8,
            (value >> 20) as u8,
        ]);
    } else if value < (1 << 35) {
        into.extend_from_slice(&[
            (value << 5) as u8 | (1 << 4),
            (value >> 3) as u8,
            (value >> 11) as u8,
            (value >> 19) as u8,
            (value >> 27) as u8,
        ]);
    } else if value < (1 << 42) {
        into.extend_from_slice(&[
            (value << 6) as u8 | (1 << 5),
            (value >> 2) as u8,
            (value >> 10) as u8,
            (value >> 18) as u8,
            (value >> 26) as u8,
            (value >> 34) as u8,
        ]);
    } else if value < (1 << 49) {
        into.extend_from_slice(&[
            (value << 7) as u8 | (1 << 6),
            (value >> 1) as u8,
            (value >> 9) as u8,
            (value >> 17) as u8,
            (value >> 25) as u8,
            (value >> 33) as u8,
            (value >> 41) as u8,
        ]);
    } else if value < (1 << 56) {
        into.extend_from_slice(&[
            (1 << 7),
            value as u8,
            (value >> 8) as u8,
            (value >> 16) as u8,
            (value >> 24) as u8,
            (value >> 32) as u8,
            (value >> 40) as u8,
            (value >> 48) as u8,
        ]);
    } else {
        into.extend_from_slice(&[
            0,
            value as u8,
            (value >> 8) as u8,
            (value >> 16) as u8,
            (value >> 24) as u8,
            (value >> 32) as u8,
            (value >> 40) as u8,
            (value >> 48) as u8,
            (value >> 56) as u8,
        ]);
    }
}

pub fn decode_prefix_varint(bytes: &[u8], offset: &mut usize) -> Option<u64> {
    // TODO: (Performance) When reading from an array, a series of values can be decoded unchecked.
    // Eg: If there are 100 bytes, each number taken can read at most 9 bytes,
    // so 11 values can be taken unchecked (up to 99 bytes). This will likely read less,
    // so this can remain in an amortized check loop until the size of the remainder
    // is less than 9 bytes.

    let first = bytes.get(*offset)?;
    let shift = first.trailing_zeros();

    // TODO: Check that the compiler does unchecked indexing after this
    if (*offset + (shift as usize)) >= bytes.len() {
        return None;
    }

    let result = match shift {
        0 => (first >> 1) as u64,
        1 => (first >> 2) as u64 | ((bytes[*offset + 1] as u64) << 6),
        2 => {
            (first >> 3) as u64
                | ((bytes[*offset + 1] as u64) << 5)
                | ((bytes[*offset + 2] as u64) << 13)
        }
        3 => {
            (first >> 4) as u64
                | ((bytes[*offset + 1] as u64) << 4)
                | ((bytes[*offset + 2] as u64) << 12)
                | ((bytes[*offset + 3] as u64) << 20)
        }
        4 => {
            (first >> 5) as u64
                | ((bytes[*offset + 1] as u64) << 3)
                | ((bytes[*offset + 2] as u64) << 11)
                | ((bytes[*offset + 3] as u64) << 19)
                | ((bytes[*offset + 4] as u64) << 27)
        }
        5 => {
            (first >> 6) as u64
                | ((bytes[*offset + 1] as u64) << 2)
                | ((bytes[*offset + 2] as u64) << 10)
                | ((bytes[*offset + 3] as u64) << 18)
                | ((bytes[*offset + 4] as u64) << 26)
                | ((bytes[*offset + 5] as u64) << 34)
        }
        6 => {
            (first >> 7) as u64
                | ((bytes[*offset + 1] as u64) << 1)
                | ((bytes[*offset + 2] as u64) << 9)
                | ((bytes[*offset + 3] as u64) << 17)
                | ((bytes[*offset + 4] as u64) << 25)
                | ((bytes[*offset + 5] as u64) << 33)
                | ((bytes[*offset + 6] as u64) << 41)
        }
        7 => {
            (bytes[*offset + 1] as u64)
                | ((bytes[*offset + 2] as u64) << 8)
                | ((bytes[*offset + 3] as u64) << 16)
                | ((bytes[*offset + 4] as u64) << 24)
                | ((bytes[*offset + 5] as u64) << 32)
                | ((bytes[*offset + 6] as u64) << 40)
                | ((bytes[*offset + 7] as u64) << 48)
        }
        8 => {
            (bytes[*offset + 1] as u64)
                | ((bytes[*offset + 2] as u64) << 8)
                | ((bytes[*offset + 3] as u64) << 16)
                | ((bytes[*offset + 4] as u64) << 24)
                | ((bytes[*offset + 5] as u64) << 32)
                | ((bytes[*offset + 6] as u64) << 40)
                | ((bytes[*offset + 7] as u64) << 48)
                | ((bytes[*offset + 8] as u64) << 56)
        }
        _ => unreachable!(),
    };
    *offset += (shift + 1) as usize;
    Some(result)
}
