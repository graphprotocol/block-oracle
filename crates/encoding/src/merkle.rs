use crate::NetworkId;

use super::Bytes32;
use tiny_keccak::{Hasher, Keccak};

pub struct MerkleLeaf {
    pub network_id: NetworkId,
    pub block_number: u64,
    pub block_hash: Bytes32,
}

impl MerkleLeaf {
    fn hash(&self) -> Bytes32 {
        keccak([
            &self.network_id.to_le_bytes(),
            &self.block_number.to_le_bytes(),
            &self.block_hash,
        ])
    }
}

pub fn merkle_root(data: &[MerkleLeaf]) -> Bytes32 {
    if data.len() == 0 {
        return Default::default();
    }

    let mut scratch: Vec<_> = data.iter().map(MerkleLeaf::hash).collect();

    let mut scratch = &mut scratch[..];

    while scratch.len() > 1 {
        let mut write = 0;
        let mut read = 0;
        while read + 1 < scratch.len() {
            let a = scratch[read];
            let b = scratch[read + 1];
            read += 2;
            scratch[write] = combine(&a, &b);
            write += 1;
        }
        if read < scratch.len() {
            scratch[write] = scratch[read];
            write += 1;
        }

        scratch = &mut scratch[0..write];
    }

    scratch.get(0).cloned().unwrap_or_default()
}

fn keccak<const N: usize>(data: [&[u8]; N]) -> Bytes32 {
    let mut hasher = Keccak::v256();
    for elem in data {
        hasher.update(elem);
    }
    let mut hash = [0; 32];
    hasher.finalize(&mut hash);
    hash
}

fn combine(a: &Bytes32, b: &Bytes32) -> Bytes32 {
    let (first, second) = if a < b { (a, b) } else { (b, a) };

    keccak([first, second])
}
