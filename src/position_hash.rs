//! Hashing for maps keyed by positions the compiler computed itself.
//!
//! The standard library's default hasher (SipHash) resists collisions an
//! adversary chooses. Byte offsets into the file being compiled are not
//! chosen that way, and SipHash costs more than the lookups it serves on
//! the projection's hot maps, so those maps use the multiply-rotate hash
//! rustc uses for its own integer keys.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

const MULTIPLIER: u64 = 0x51_7c_c1_b7_27_22_0a_95;

#[derive(Clone, Copy, Default)]
pub(crate) struct PositionHasher(u64);

impl PositionHasher {
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(MULTIPLIER);
    }
}

impl Hasher for PositionHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.add(u64::from(byte));
        }
    }

    fn write_u32(&mut self, value: u32) {
        self.add(u64::from(value));
    }

    fn write_u64(&mut self, value: u64) {
        self.add(value);
    }

    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

pub(crate) type PositionMap<K, V> = HashMap<K, V, BuildHasherDefault<PositionHasher>>;
pub(crate) type PositionSet<K> = HashSet<K, BuildHasherDefault<PositionHasher>>;
