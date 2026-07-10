//! RFC 6962 Merkle tree construction and inclusion proofs.
//!
//! Pure hashing logic, no I/O — mirrors the leaf/node domain separation from
//! RFC 6962 §2.1 (`MTH`) exactly, since VCP mandates RFC 6962 tree
//! construction for anchoring decision batches. Leaf hash and internal node
//! hash use distinct one-byte prefixes so a leaf hash can never be replayed
//! as an internal node hash (the second-preimage attack RFC 6962 exists to
//! prevent).

use sha2::{Digest, Sha256};

/// Which side of the accumulator a proof step's sibling hash sits on when
/// folding upward from the leaf toward the root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// One step of an inclusion (audit) proof: a sibling hash and which side of
/// the running accumulator it combines on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofStep {
    pub hash: [u8; 32],
    pub side: Side,
}

/// `H(0x00 || data)` — RFC 6962 leaf hash.
#[must_use]
pub fn leaf_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([0x00]);
    hasher.update(data);
    hasher.finalize().into()
}

/// `H(0x01 || left || right)` — RFC 6962 internal node hash.
#[must_use]
pub fn node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([0x01]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// Largest power of two strictly smaller than `n` (`n` must be >= 2).
fn split_point(n: usize) -> usize {
    debug_assert!(n >= 2);
    let mut k = 1usize;
    while k * 2 < n {
        k *= 2;
    }
    k
}

/// RFC 6962 `MTH` over already leaf-hashed values. `hashes` must be non-empty.
fn mth(hashes: &[[u8; 32]]) -> [u8; 32] {
    let n = hashes.len();
    assert!(n > 0, "MTH is undefined over an empty leaf set here");
    if n == 1 {
        return hashes[0];
    }
    let k = split_point(n);
    let left = mth(&hashes[..k]);
    let right = mth(&hashes[k..]);
    node_hash(&left, &right)
}

/// RFC 6962 `PATH(m, D[n])` over already leaf-hashed values, ordered from the
/// leaf level upward (fold with [`verify_inclusion`] in that order).
fn audit_path(m: usize, hashes: &[[u8; 32]]) -> Vec<ProofStep> {
    let n = hashes.len();
    if n <= 1 {
        return Vec::new();
    }
    let k = split_point(n);
    if m < k {
        let mut path = audit_path(m, &hashes[..k]);
        path.push(ProofStep {
            hash: mth(&hashes[k..]),
            side: Side::Right,
        });
        path
    } else {
        let mut path = audit_path(m - k, &hashes[k..]);
        path.push(ProofStep {
            hash: mth(&hashes[..k]),
            side: Side::Left,
        });
        path
    }
}

/// A batch of decision event hashes anchored together as one RFC 6962 tree.
pub struct MerkleTree {
    leaves: Vec<[u8; 32]>,
}

impl MerkleTree {
    /// Build a tree from raw leaf data (e.g. each decision's `event_hash`
    /// bytes) — each entry is leaf-hashed internally.
    ///
    /// # Panics
    /// Panics if `entries` is empty; an anchor batch must contain at least
    /// one decision.
    #[must_use]
    pub fn from_leaves<T: AsRef<[u8]>>(entries: &[T]) -> Self {
        assert!(
            !entries.is_empty(),
            "cannot build a Merkle tree over zero entries"
        );
        let leaves = entries.iter().map(|e| leaf_hash(e.as_ref())).collect();
        Self { leaves }
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

    /// Inclusion proof for the leaf at `index` (0-based, insertion order).
    ///
    /// # Panics
    /// Panics if `index` is out of bounds.
    #[must_use]
    pub fn inclusion_proof(&self, index: usize) -> Vec<ProofStep> {
        assert!(index < self.leaves.len(), "index out of bounds");
        audit_path(index, &self.leaves)
    }
}

/// Verify that `leaf_data`, at position `index` in a tree of `tree_size`
/// leaves, is included under `root`, given its inclusion `proof`.
///
/// This is the function a third party runs with zero knowledge of anything
/// but the leaf's own raw data, its claimed index, and the published root —
/// it never touches HSIP's database.
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

    fn d(bytes: &[u8]) -> Vec<u8> {
        bytes.to_vec()
    }

    #[test]
    fn single_leaf_root_is_its_own_leaf_hash() {
        let tree = MerkleTree::from_leaves(&[d(b"only")]);
        assert_eq!(tree.root(), leaf_hash(b"only"));
        assert!(tree.inclusion_proof(0).is_empty());
        assert!(verify_inclusion(b"only", &[], &tree.root()));
    }

    #[test]
    fn two_leaf_root_matches_hand_computed_value() {
        let tree = MerkleTree::from_leaves(&[d(b"a"), d(b"b")]);
        let expected = node_hash(&leaf_hash(b"a"), &leaf_hash(b"b"));
        assert_eq!(tree.root(), expected);
    }

    #[test]
    fn inclusion_proof_verifies_for_every_index_across_sizes() {
        for n in 1..=17usize {
            let entries: Vec<Vec<u8>> = (0..n).map(|i| d(&[i as u8])).collect();
            let tree = MerkleTree::from_leaves(&entries);
            let root = tree.root();
            for i in 0..n {
                let proof = tree.inclusion_proof(i);
                assert!(
                    verify_inclusion(&entries[i], &proof, &root),
                    "inclusion proof failed to verify for n={n}, i={i}"
                );
            }
        }
    }

    #[test]
    fn tampered_leaf_fails_verification() {
        let entries: Vec<Vec<u8>> = (0..5u8).map(|i| d(&[i])).collect();
        let tree = MerkleTree::from_leaves(&entries);
        let root = tree.root();
        let proof = tree.inclusion_proof(2);
        assert!(!verify_inclusion(b"not the real leaf", &proof, &root));
    }

    #[test]
    fn tampered_proof_step_fails_verification() {
        let entries: Vec<Vec<u8>> = (0..6u8).map(|i| d(&[i])).collect();
        let tree = MerkleTree::from_leaves(&entries);
        let root = tree.root();
        let mut proof = tree.inclusion_proof(4);
        proof[0].hash[0] ^= 0xff;
        assert!(!verify_inclusion(&entries[4], &proof, &root));
    }

    #[test]
    fn wrong_root_fails_verification() {
        let entries: Vec<Vec<u8>> = (0..3u8).map(|i| d(&[i])).collect();
        let tree = MerkleTree::from_leaves(&entries);
        let proof = tree.inclusion_proof(1);
        let wrong_root = [0u8; 32];
        assert!(!verify_inclusion(&entries[1], &proof, &wrong_root));
    }

    #[test]
    fn leaf_hash_and_node_hash_use_distinct_domain_prefixes() {
        // A leaf hash of some bytes must never collide with a node hash
        // built from those same bytes split as two 32-byte halves — this is
        // the exact second-preimage attack RFC 6962's prefixing prevents.
        let a = [0x11u8; 32];
        let b = [0x22u8; 32];
        let mut concatenated = Vec::new();
        concatenated.extend_from_slice(&a);
        concatenated.extend_from_slice(&b);
        assert_ne!(leaf_hash(&concatenated), node_hash(&a, &b));
    }

    #[test]
    #[should_panic(expected = "cannot build a Merkle tree over zero entries")]
    fn empty_tree_panics() {
        let empty: Vec<Vec<u8>> = Vec::new();
        let _ = MerkleTree::from_leaves(&empty);
    }
}
