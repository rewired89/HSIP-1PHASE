//! RFC 6962 Merkle tree (domain-separated leaf/node hashing, SHA-256).
//! Pure: no I/O. Same construction the anchor service uses to batch proofs.

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofStep {
    pub hash: [u8; 32],
    pub side: Side,
}

#[must_use]
pub fn leaf_hash(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([0x00]);
    h.update(data);
    h.finalize().into()
}

#[must_use]
pub fn node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([0x01]);
    h.update(left);
    h.update(right);
    h.finalize().into()
}

fn split_point(n: usize) -> usize {
    let mut k = 1usize;
    while k * 2 < n {
        k *= 2;
    }
    k
}

fn mth(hashes: &[[u8; 32]]) -> [u8; 32] {
    if hashes.len() == 1 {
        return hashes[0];
    }
    let k = split_point(hashes.len());
    node_hash(&mth(&hashes[..k]), &mth(&hashes[k..]))
}

fn audit_path(m: usize, hashes: &[[u8; 32]]) -> Vec<ProofStep> {
    let n = hashes.len();
    if n <= 1 {
        return Vec::new();
    }
    let k = split_point(n);
    if m < k {
        let mut p = audit_path(m, &hashes[..k]);
        p.push(ProofStep {
            hash: mth(&hashes[k..]),
            side: Side::Right,
        });
        p
    } else {
        let mut p = audit_path(m - k, &hashes[k..]);
        p.push(ProofStep {
            hash: mth(&hashes[..k]),
            side: Side::Left,
        });
        p
    }
}

pub struct MerkleTree {
    leaves: Vec<[u8; 32]>,
}

impl MerkleTree {
    /// Panics on an empty entry list; callers batch only when non-empty.
    #[must_use]
    pub fn from_leaves<T: AsRef<[u8]>>(entries: &[T]) -> Self {
        assert!(
            !entries.is_empty(),
            "cannot build a Merkle tree over zero entries"
        );
        Self {
            leaves: entries.iter().map(|e| leaf_hash(e.as_ref())).collect(),
        }
    }

    #[must_use]
    pub fn root(&self) -> [u8; 32] {
        mth(&self.leaves)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    #[must_use]
    pub fn inclusion_proof(&self, index: usize) -> Vec<ProofStep> {
        assert!(index < self.leaves.len(), "index out of bounds");
        audit_path(index, &self.leaves)
    }
}

#[must_use]
pub fn verify_inclusion(leaf_data: &[u8], proof: &[ProofStep], root: &[u8; 32]) -> bool {
    let mut acc = leaf_hash(leaf_data);
    for step in proof {
        acc = match step.side {
            Side::Right => node_hash(&acc, &step.hash),
            Side::Left => node_hash(&step.hash, &acc),
        };
    }
    &acc == root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inclusion_verifies_for_every_index_across_sizes() {
        for n in 1..=33usize {
            let entries: Vec<Vec<u8>> = (0..n).map(|i| vec![i as u8]).collect();
            let tree = MerkleTree::from_leaves(&entries);
            let root = tree.root();
            for (i, e) in entries.iter().enumerate() {
                assert!(
                    verify_inclusion(e, &tree.inclusion_proof(i), &root),
                    "n={n} i={i}"
                );
            }
        }
    }

    #[test]
    fn tampering_is_detected() {
        let entries: Vec<Vec<u8>> = (0..6u8).map(|i| vec![i]).collect();
        let tree = MerkleTree::from_leaves(&entries);
        let root = tree.root();
        let mut proof = tree.inclusion_proof(4);
        assert!(!verify_inclusion(b"forged", &proof, &root));
        proof[0].hash[0] ^= 0xff;
        assert!(!verify_inclusion(&entries[4], &proof, &root));
        assert!(!verify_inclusion(
            &entries[4],
            &tree.inclusion_proof(4),
            &[0u8; 32]
        ));
    }

    #[test]
    fn leaf_and_node_domains_differ() {
        let (a, b) = ([1u8; 32], [2u8; 32]);
        let mut cat = a.to_vec();
        cat.extend_from_slice(&b);
        assert_ne!(leaf_hash(&cat), node_hash(&a, &b));
    }
}
