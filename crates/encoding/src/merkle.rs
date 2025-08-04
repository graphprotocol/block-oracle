use super::Bytes32;
use crate::NetworkIndex;
use tiny_keccak::{Hasher, Keccak};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleLeaf {
    pub network_index: NetworkIndex,
    pub block_number: u64,
    pub block_hash: Bytes32,
}

impl MerkleLeaf {
    fn hash(&self) -> Bytes32 {
        keccak([
            &self.network_index.to_le_bytes(),
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

        scratch.truncate(write);
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
            network_index: 42,
            block_number: 1337,
            block_hash: [9; 32],
        };

        assert_eq!(leaf.hash(), merkle_root(&[leaf]));
    }

    #[test]
    fn merkle_root_with_two_leaves() {
        // This is the critical test case that would fail with the bug
        let leaves = vec![
            MerkleLeaf {
                network_index: 0,
                block_number: 100,
                block_hash: [1; 32],
            },
            MerkleLeaf {
                network_index: 1,
                block_number: 200,
                block_hash: [2; 32],
            },
        ];

        let root = merkle_root(&leaves);
        // Should not be all zeros!
        assert_ne!(root, [0; 32]);

        // Should be deterministic
        let root2 = merkle_root(&leaves);
        assert_eq!(root, root2);
    }

    #[test]
    fn merkle_root_with_multiple_leaves() {
        // Test various sizes that would trigger the bug
        for size in [2, 3, 4, 5, 7, 8, 16, 26, 32] {
            let leaves: Vec<MerkleLeaf> = (0..size)
                .map(|i| MerkleLeaf {
                    network_index: i as u64,
                    block_number: (i + 1) as u64 * 100,
                    block_hash: [(i + 1) as u8; 32],
                })
                .collect();

            let root = merkle_root(&leaves);

            // Root should not be all zeros for non-empty inputs
            assert_ne!(
                root, [0; 32],
                "Merkle root was all zeros for {} leaves",
                size
            );

            // Root should be deterministic
            let root2 = merkle_root(&leaves);
            assert_eq!(
                root, root2,
                "Merkle root was not deterministic for {} leaves",
                size
            );
        }
    }

    #[test]
    fn merkle_root_combines_in_correct_order() {
        // Test that leaf data affects the root
        let leaves1 = vec![
            MerkleLeaf {
                network_index: 0,
                block_number: 100,
                block_hash: [0xAA; 32],
            },
            MerkleLeaf {
                network_index: 1,
                block_number: 200,
                block_hash: [0xBB; 32],
            },
            MerkleLeaf {
                network_index: 2,
                block_number: 300,
                block_hash: [0xCC; 32],
            },
        ];

        let leaves2 = vec![
            MerkleLeaf {
                network_index: 0,
                block_number: 100,
                block_hash: [0xAA; 32],
            },
            MerkleLeaf {
                network_index: 1,
                block_number: 200,
                block_hash: [0xBB; 32],
            },
            MerkleLeaf {
                network_index: 2,
                block_number: 300,
                block_hash: [0xDD; 32], // Different hash
            },
        ];

        let root1 = merkle_root(&leaves1);
        let root2 = merkle_root(&leaves2);

        assert_ne!(root1, [0; 32]);
        assert_ne!(root2, [0; 32]);
        assert_ne!(
            root1, root2,
            "Different leaf data should produce different roots"
        );
    }

    #[test]
    fn merkle_root_handles_power_of_two() {
        // Powers of 2 are important edge cases
        for power in [1, 2, 3, 4, 5] {
            let size = 1 << power; // 2, 4, 8, 16, 32
            let leaves: Vec<MerkleLeaf> = (0..size)
                .map(|i| MerkleLeaf {
                    network_index: i,
                    block_number: i as u64 + 1000,
                    block_hash: {
                        let mut hash = [0; 32];
                        hash[0] = i as u8;
                        hash[1] = (i >> 8) as u8;
                        hash
                    },
                })
                .collect();

            let root = merkle_root(&leaves);
            assert_ne!(
                root, [0; 32],
                "Merkle root was all zeros for 2^{} = {} leaves",
                power, size
            );
        }
    }

    #[test]
    fn merkle_root_26_leaves_real_scenario() {
        // Test the exact scenario from the bug report - 26 networks
        let leaves: Vec<MerkleLeaf> = (0..26)
            .map(|i| MerkleLeaf {
                network_index: i,
                block_number: 23052969 + (i as u64 * 1000000), // Varying block numbers
                block_hash: {
                    let mut hash = [0; 32];
                    // Create some variety in the hashes
                    for j in 0..32 {
                        hash[j] = ((i as usize + j) % 256) as u8;
                    }
                    hash
                },
            })
            .collect();

        let root = merkle_root(&leaves);
        assert_ne!(
            root, [0; 32],
            "Merkle root was all zeros for 26 leaves (real scenario)"
        );
    }
}
