use super::Bytes32;
use crate::NetworkId;
use tiny_keccak::{Hasher, Keccak};

#[derive(Debug, Clone, PartialEq, Eq)]
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
    let mut scratch: Vec<Bytes32> = data.iter().map(MerkleLeaf::hash).collect();

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

        scratch.truncate(write - 1);
    }

    scratch.first().cloned().unwrap_or_default()
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
    keccak([a.min(b), a.max(b)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merkle_root_empty() {
        assert_eq!(merkle_root(&[]), [0; 32]);
    }

    #[test]
    fn merkle_root_with_one_leaf() {
        let leaf = MerkleLeaf {
            network_id: 42,
            block_number: 1337,
            block_hash: [9; 32],
        };

        assert_eq!(leaf.hash(), merkle_root(&[leaf]));
    }
}
