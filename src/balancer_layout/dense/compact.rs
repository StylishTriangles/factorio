use crate::balancer_layout::{Layout, compact_layout};
use std::sync::LazyLock;

// Merge each pair of outlets from the 24→32 core onto one belt.
static REDUCTION: LazyLock<Vec<String>> =
    LazyLock::new(|| read_pattern(include_str!("../../../data/compact_24_to_16.txt")));

// Four 6×6 balancers feed six 4×4 balancers through a ten-column shuffle.
static SQUARE: LazyLock<Vec<String>> =
    LazyLock::new(|| read_pattern(include_str!("../../../data/compact_24_to_24.txt")));

// Derived from the supplied 32_32_balancer (32x25) blueprint by dropping
// eight inputs, their unreachable tracks, and both straight-belt collars.
static EXPANSION: LazyLock<Vec<String>> =
    LazyLock::new(|| read_pattern(include_str!("../../../data/compact_24_to_32.txt")));

fn read_pattern(text: &str) -> Vec<String> {
    text.lines().map(str::to_owned).collect()
}

pub(super) fn generate(outputs: usize) -> Option<Layout> {
    let pattern = match outputs {
        16 => &*REDUCTION,
        24 => &*SQUARE,
        32 => &*EXPANSION,
        _ => return None,
    };
    Some(compact_layout(pattern))
}
