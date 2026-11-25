use crate::balancer_layout::{Layout, compact_layout};
use std::sync::LazyLock;

// Derived from the supplied 32_32_balancer (32x25) blueprint by dropping
// eight inputs, their unreachable tracks, and both straight-belt collars.
static PATTERN: LazyLock<Vec<String>> = LazyLock::new(|| {
    include_str!("../../../data/compact_24_to_32.txt")
        .lines()
        .map(str::to_owned)
        .collect()
});

pub(super) fn generate() -> Layout {
    let mut layout = compact_layout(&PATTERN);
    // Both tiles of each edge splitter are ports.
    layout.input_rows = (0..24).collect();
    layout.output_rows = (0..32).collect();
    layout
}
