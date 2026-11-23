//! Run seeds: where a run's seed comes from, how it is shown and typed, and how one seed becomes
//! many independent random streams (`docs/variety.md`).
//!
//! Pure rules, no window and no clock. `SeedSource` hands out one `u64` per run; `main.rs` reads
//! the system clock once at start-up and starts the source from it, so a test never depends on
//! time. `stream(seed, label, key)` gives each use of a seed its own `particles::Rng`, so two
//! labels or two keys never share a sequence and the same inputs always give the same numbers.
//! A seed is shown as `format_seed` (16 hex digits) and read back with `parse_seed`.

use crate::particles::Rng;

/// Where a source that is not told otherwise starts: "HOLLOWST" in ASCII, a fixed number so no
/// test depends on a clock.
const DEFAULT_START: u64 = 0x484F_4C4C_4F57_5354;

/// The FNV-1a 64-bit offset basis and prime.
const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

/// The most hex digits a seed has: a seed is a `u64`.
pub const MAX_SEED_DIGITS: usize = 16;

/// Hands out one seed per run: a SplitMix64 sequence from a start value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeedSource(Rng);

impl SeedSource {
    /// A source that walks the SplitMix64 sequence from `start`.
    pub fn new(start: u64) -> Self {
        Self(Rng::new(start))
    }

    /// The next seed. The sequence never ends, so this is a plain number and not an `Option`.
    // Not an `Iterator`: there is no `None` to return, and callers want the number itself.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u64 {
        self.0.next_u64()
    }
}

impl Default for SeedSource {
    /// Starts from a fixed constant, never from a clock.
    fn default() -> Self {
        Self::new(DEFAULT_START)
    }
}

/// One SplitMix64 step from `value`: the next output of a generator whose state is `value`. It
/// is a bijection, so two different inputs never give the same output.
fn mix(value: u64) -> u64 {
    Rng::new(value).next_u64()
}

/// The 64-bit FNV-1a hash of `bytes`: a fixed, dependency-free way to turn a name into a number
/// (it is how a label, and a placement's identity, become part of a stream's key).
pub fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

/// A random stream for one use of `seed`, named by `label` (what it is for, such as
/// `"enemies"`) and `key` (which one, such as a room and a group). The label is hashed with
/// FNV-1a, then the seed, the label and the key are mixed through SplitMix steps. The same three
/// values always give the same stream; a different label or key (or seed) gives a different one,
/// and for one seed and label two keys never start from the same state.
pub fn stream(seed: u64, label: &str, key: u64) -> Rng {
    let state = mix(mix(seed) ^ fnv1a(label.as_bytes())) ^ key;
    Rng::new(mix(state))
}

/// A seed as shown to the player: 16 lowercase hex digits, leading zeros kept.
pub fn format_seed(seed: u64) -> String {
    format!("{seed:016x}")
}

/// Reads a seed typed by the player: 1 to 16 hex digits in either case, with an optional `0x`
/// (or `0X`) in front. Anything else is refused with a message that says why: nothing typed, no
/// digits after the prefix, a character that is not a hex digit (spaces included), or more than
/// 16 digits.
pub fn parse_seed(text: &str) -> Result<u64, String> {
    let digits = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    if digits.is_empty() {
        return Err(if text.is_empty() {
            "the seed is empty: type 1 to 16 hex digits (0-9, a-f)".to_owned()
        } else {
            "the seed has no hex digits after 0x: type 1 to 16 (0-9, a-f)".to_owned()
        });
    }
    if let Some(bad) = digits.chars().find(|c| !c.is_ascii_hexdigit()) {
        return Err(format!("{bad:?} is not a hex digit (use 0-9 and a-f)"));
    }
    if digits.len() > MAX_SEED_DIGITS {
        return Err(format!(
            "a seed has at most {MAX_SEED_DIGITS} hex digits, this one has {}",
            digits.len()
        ));
    }
    u64::from_str_radix(digits, 16).map_err(|error| format!("not a seed: {error}"))
}

#[cfg(test)]
mod tests;
