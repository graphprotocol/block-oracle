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
