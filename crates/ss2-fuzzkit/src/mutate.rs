//! A tiny deterministic mutator and random source for the stable-toolchain smoke test.
//!
//! libFuzzer mutates with coverage feedback; this does not. It is the same *kind* of damage (flipped bits, overwritten
//! numbers, cut and repeated ranges) applied to the synthetic seeds in a fixed order, so that every run of the test
//! suite feeds every parser the same few thousand inputs and a failure can be reproduced from its index.

/// A xorshift64* generator: tiny, seedable, the same on every platform.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    /// A generator for `seed` (any value; zero is replaced).
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number below `n` (`0` when `n` is `0`).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// One random byte.
    pub fn byte(&mut self) -> u8 {
        self.next_u64().to_le_bytes()[3]
    }
}

/// Numbers that sit on the edge of a size, count or offset check.
const EDGES: [u32; 12] = [
    0,
    1,
    2,
    3,
    0x7F,
    0xFF,
    0x100,
    0x7FFF_FFFF,
    0x8000_0000,
    0xFFFF_FFFF,
    0x00FF_FFFF,
    0x1_0000,
];

/// Damage `data` in place with up to `edits` random edits, keeping it within `max_len` bytes.
pub fn mutate(rng: &mut Rng, data: &mut Vec<u8>, edits: usize, max_len: usize) {
    for _ in 0..edits {
        let len = data.len();
        match rng.below(9) {
            0 if len > 0 => {
                let i = rng.below(len);
                data[i] ^= 1 << rng.below(8);
            }
            1 if len > 0 => {
                let i = rng.below(len);
                data[i] = rng.byte();
            }
            2 if len >= 4 => {
                // Overwrite a 4-byte number with an edge value.
                let i = rng.below(len - 3);
                let v = EDGES[rng.below(EDGES.len())];
                data[i..i + 4].copy_from_slice(&v.to_le_bytes());
            }
            3 if len >= 2 => {
                let i = rng.below(len - 1);
                let v = [0u16, 1, 0x7FFF, 0x8000, 0xFFFF][rng.below(5)];
                data[i..i + 2].copy_from_slice(&v.to_le_bytes());
            }
            4 if len > 0 => {
                // Cut a range.
                let a = rng.below(len);
                let b = (a + 1 + rng.below(16)).min(len);
                data.drain(a..b);
            }
            5 if len > 0 => {
                // Truncate.
                data.truncate(rng.below(len));
            }
            6 if len > 0 && len < max_len => {
                // Repeat a range.
                let a = rng.below(len);
                let b = (a + 1 + rng.below(32)).min(len);
                let piece: Vec<u8> = data[a..b].to_vec();
                let at = rng.below(len + 1);
                data.splice(at..at, piece);
            }
            7 if len < max_len => {
                let at = rng.below(len + 1);
                data.insert(at, rng.byte());
            }
            8 if len >= 8 => {
                // Swap two bytes far apart.
                let (a, b) = (rng.below(len), rng.below(len));
                data.swap(a, b);
            }
            _ => {}
        }
    }
    data.truncate(max_len);
}

/// `len` random bytes.
pub fn random_bytes(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.byte()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_generator_is_deterministic_and_seeds_differ() {
        let (mut a, mut b, mut c) = (Rng::new(7), Rng::new(7), Rng::new(8));
        let xs: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        assert_eq!(xs, (0..8).map(|_| b.next_u64()).collect::<Vec<_>>());
        assert_ne!(xs, (0..8).map(|_| c.next_u64()).collect::<Vec<_>>());
        assert!(xs.windows(2).all(|w| w[0] != w[1]));
        assert_eq!(Rng::new(1).next_u64(), 0x47e4ce4b896cdd1d);
        let mut pinned = Rng::new(0x0123_4567_89AB_CDEF);
        assert_eq!(
            (pinned.next_u64(), pinned.next_u64()),
            (0x7c9482472cb6708c, 0xd5705692bf1f28de)
        );
        assert_ne!(
            Rng::new(0).next_u64(),
            0,
            "seed 0 is replaced, not stuck at 0"
        );
    }

    #[test]
    fn below_stays_below_and_covers_the_range() {
        let mut rng = Rng::new(3);
        assert_eq!(rng.below(0), 0);
        assert_eq!(rng.below(1), 0);
        let mut seen = [false; 10];
        for _ in 0..500 {
            let v = rng.below(10);
            assert!(v < 10);
            seen[v] = true;
        }
        assert!(seen.iter().all(|s| *s), "every value of 0..10 occurs");
    }

    #[test]
    fn bytes_cover_all_bit_positions() {
        let mut rng = Rng::new(5);
        let all = random_bytes(&mut rng, 4000);
        assert_eq!(all.len(), 4000);
        assert!(
            (0..8).all(|bit| all.iter().any(|b| b >> bit & 1 == 1)
                && all.iter().any(|b| b >> bit & 1 == 0))
        );
        assert!(random_bytes(&mut rng, 0).is_empty());
    }

    #[test]
    fn mutation_respects_the_length_cap_and_changes_things() {
        let seed: Vec<u8> = (0..64).collect();
        let mut rng = Rng::new(11);
        let (mut changed, mut longer, mut shorter) = (0, 0, 0);
        for _ in 0..2000 {
            let mut d = seed.clone();
            mutate(&mut rng, &mut d, 3, 80);
            assert!(d.len() <= 80);
            changed += usize::from(d != seed);
            longer += usize::from(d.len() > seed.len());
            shorter += usize::from(d.len() < seed.len());
        }
        assert!(
            changed > 1500,
            "most mutations change the input ({changed})"
        );
        assert!(
            longer > 100 && shorter > 100,
            "both growth and shrinking happen ({longer}, {shorter})"
        );
        // The cap is applied even when the input is already over it.
        let mut big = vec![1u8; 500];
        mutate(&mut rng, &mut big, 0, 100);
        assert_eq!(big.len(), 100);
        // Empty and tiny inputs are handled.
        for len in 0..4 {
            let mut d = vec![9u8; len];
            for _ in 0..200 {
                mutate(&mut rng, &mut d, 2, 8);
                assert!(d.len() <= 8);
            }
        }
    }

    #[test]
    fn each_kind_of_edit_happens() {
        let seed: Vec<u8> = vec![0x55; 64];
        let mut rng = Rng::new(21);
        let (mut one_bit, mut edge32, mut edge16, mut cut, mut grown) = (0, 0, 0, 0, 0);
        for _ in 0..6000 {
            let mut d = seed.clone();
            mutate(&mut rng, &mut d, 1, 200);
            if d.len() == seed.len() {
                let diff: Vec<usize> = (0..d.len()).filter(|&i| d[i] != seed[i]).collect();
                if diff.len() == 1 && (d[diff[0]] ^ seed[diff[0]]).count_ones() == 1 {
                    one_bit += 1;
                }
                if d.windows(4).any(|w| {
                    w == 0xFFFF_FFFFu32.to_le_bytes()
                        || w == 0x8000_0000u32.to_le_bytes()
                        || w == [0, 0, 0, 0]
                }) {
                    edge32 += 1;
                }
                if d.windows(2).any(|w| w == [0xFF, 0xFF] || w == [0xFF, 0x7F]) {
                    edge16 += 1;
                }
            }
            cut += usize::from(d.len() < seed.len());
            grown += usize::from(d.len() > seed.len());
        }
        assert!(one_bit > 100, "bit flips ({one_bit})");
        assert!(edge32 > 100, "four-byte edge values ({edge32})");
        assert!(edge16 > 50, "two-byte edge values ({edge16})");
        assert!(cut > 200, "cuts and truncations ({cut})");
        assert!(grown > 200, "repeats and inserts ({grown})");
    }

    #[test]
    fn a_swap_exchanges_two_bytes() {
        let seed: Vec<u8> = (0..32).collect();
        let mut rng = Rng::new(31);
        let mut swaps = 0;
        for _ in 0..6000 {
            let mut d = seed.clone();
            mutate(&mut rng, &mut d, 1, 32);
            let mut sorted = d.clone();
            sorted.sort_unstable();
            if d.len() == 32 && sorted == seed && d != seed {
                swaps += 1;
            }
        }
        assert!(swaps > 100, "a permutation of the same bytes ({swaps})");
    }

    /// Run `n` single-edit mutations of `seed` and hand each result to `f`.
    fn each(seed: &[u8], n: usize, max_len: usize, rng_seed: u64, mut f: impl FnMut(&[u8])) {
        let mut rng = Rng::new(rng_seed);
        for _ in 0..n {
            let mut d = seed.to_vec();
            mutate(&mut rng, &mut d, 1, max_len);
            f(&d);
        }
    }

    #[test]
    fn a_bit_flip_flips_every_bit_position_both_ways() {
        for fill in [0x00u8, 0xFF] {
            let seed = vec![fill; 40];
            let mut positions = [0usize; 8];
            let mut plain = 0;
            each(&seed, 8000, 40, 41, |d| {
                if d.len() == seed.len() {
                    let diff: Vec<usize> = (0..d.len()).filter(|&i| d[i] != fill).collect();
                    if let [i] = diff[..] {
                        let x = d[i] ^ fill;
                        if x.count_ones() == 1 {
                            positions[x.trailing_zeros() as usize] += 1;
                            plain += 1;
                        }
                    }
                }
            });
            assert!(
                positions.iter().all(|&c| c > 30),
                "fill {fill:#x}: bit positions {positions:?}"
            );
            assert!(plain > 600, "fill {fill:#x}: {plain}");
        }
    }

    #[test]
    fn numbers_are_written_anywhere_up_to_the_end() {
        let seed = vec![0x55u8; 12];
        let (mut first, mut last4, mut last2) = (false, false, false);
        each(&seed, 20000, 12, 43, |d| {
            if d.len() == 12 {
                first |= d[..4] == [0, 0, 0, 0] || d[..4] == [0xFF; 4];
                last4 |= d[8..] == [0, 0, 0, 0] || d[8..] == [0xFF; 4];
                last2 |= d[10..] == [0xFF, 0xFF] || d[10..] == [0, 0];
            }
        });
        assert!(first && last4 && last2);
    }

    #[test]
    fn cuts_repeats_and_inserts_have_the_documented_sizes() {
        let seed: Vec<u8> = (0..100).collect();
        let (mut cut, mut grow) = (
            std::collections::BTreeSet::new(),
            std::collections::BTreeSet::new(),
        );
        let (mut appended, mut prepended, mut cut_tail) = (false, false, false);
        each(&seed, 30000, 400, 47, |d| {
            if d.len() < seed.len()
                && seed.starts_with(&d[..d.len().min(1)])
                && !d.iter().eq(seed.iter().take(d.len()))
            {
                cut.insert(seed.len() - d.len());
            }
            if d.len() > seed.len() {
                grow.insert(d.len() - seed.len());
                appended |= d.starts_with(&seed);
                prepended |= d.ends_with(&seed);
            }
            cut_tail |= d.len() < seed.len() && d.iter().eq(seed.iter().take(d.len()));
        });
        assert!((1..=16).all(|n| cut.contains(&n)), "cut sizes {cut:?}");
        assert!(
            grow.contains(&1) && grow.contains(&32) && grow.iter().all(|&n| n <= 32),
            "growth {grow:?}"
        );
        assert!(appended && prepended && cut_tail);
    }

    #[test]
    fn nothing_grows_past_the_cap_and_a_full_input_still_changes() {
        let seed = vec![7u8; 16];
        each(&seed, 3000, 16, 53, |d| assert!(d.len() <= 16));
        let mut shrunk = false;
        each(&seed, 3000, 16, 59, |d| shrunk |= d.len() < 16);
        assert!(shrunk);
    }
}
