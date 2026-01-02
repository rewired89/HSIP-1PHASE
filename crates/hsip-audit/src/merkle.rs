//! Merkle Tree for Audit Log Anchoring
//!
//! Optional premium feature: anchor audit log state to blockchain

use serde::{Deserialize, Serialize};

/// Merkle tree for audit log
pub struct MerkleTree {
    leaves: Vec<[u8; 32]>,
    root: Option<[u8; 32]>,
}

impl MerkleTree {
    /// Create a new Merkle tree from leaf hashes
    pub fn new(leaves: Vec<[u8; 32]>) -> Self {
        let root = if leaves.is_empty() {
            None
        } else {
            Some(Self::compute_root(&leaves))
        };

        Self { leaves, root }
    }

    fn compute_root(leaves: &[[u8; 32]]) -> [u8; 32] {
        if leaves.is_empty() {
            return [0u8; 32];
        }

        if leaves.len() == 1 {
            return leaves[0];
        }

        let mut current_level = leaves.to_vec();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in current_level.chunks(2) {
                let hash = if chunk.len() == 2 {
                    Self::hash_pair(&chunk[0], &chunk[1])
                } else {
                    chunk[0]
                };
                next_level.push(hash);
            }

            current_level = next_level;
        }

        current_level[0]
    }

    fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(left);
        hasher.update(right);
        *hasher.finalize().as_bytes()
    }

    /// Get the Merkle root
    pub fn root(&self) -> Option<[u8; 32]> {
        self.root
    }

    /// Generate a Merkle proof for a leaf at index
    pub fn generate_proof(&self, index: usize) -> Option<MerkleProof> {
        if index >= self.leaves.len() {
            return None;
        }

        let mut proof_hashes = Vec::new();
        let mut current_level = self.leaves.clone();
        let mut current_index = index;

        while current_level.len() > 1 {
            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            if sibling_index < current_level.len() {
                proof_hashes.push(current_level[sibling_index]);
            }

            let mut next_level = Vec::new();
            for chunk in current_level.chunks(2) {
                let hash = if chunk.len() == 2 {
                    Self::hash_pair(&chunk[0], &chunk[1])
                } else {
                    chunk[0]
                };
                next_level.push(hash);
            }

            current_level = next_level;
            current_index /= 2;
        }

        Some(MerkleProof {
            leaf_hash: self.leaves[index],
            proof_hashes,
            root: self.root.unwrap(),
        })
    }
}

/// Merkle proof for a leaf
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf_hash: [u8; 32],
    pub proof_hashes: Vec<[u8; 32]>,
    pub root: [u8; 32],
}

impl MerkleProof {
    /// Verify this proof
    pub fn verify(&self) -> bool {
        let mut current_hash = self.leaf_hash;

        for proof_hash in &self.proof_hashes {
            current_hash = Self::hash_pair(&current_hash, proof_hash);
        }

        current_hash == self.root
    }

    fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(left);
        hasher.update(right);
        *hasher.finalize().as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree() {
        let leaves = vec![
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            [4u8; 32],
        ];

        let tree = MerkleTree::new(leaves);
        assert!(tree.root().is_some());
    }

    #[test]
    fn test_merkle_proof() {
        let leaves = vec![
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            [4u8; 32],
        ];

        let tree = MerkleTree::new(leaves);
        let proof = tree.generate_proof(0).unwrap();

        assert!(proof.verify());
    }

    #[test]
    fn test_merkle_proof_invalid() {
        let leaves = vec![
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            [4u8; 32],
        ];

        let tree = MerkleTree::new(leaves);
        let mut proof = tree.generate_proof(0).unwrap();

        // Tamper with the proof
        proof.leaf_hash = [99u8; 32];

        assert!(!proof.verify());
    }
}
