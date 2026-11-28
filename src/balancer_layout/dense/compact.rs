use crate::balancer_layout::{EAST, Layout, compact_layout, push_belt};
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

pub(super) fn generate(outputs: usize, output_gap: usize) -> Option<Layout> {
    let pattern = match (outputs, output_gap) {
        (16, 0) => &*REDUCTION,
        (16 | 32, _) => &*EXPANSION,
        (24, _) => &*SQUARE,
        _ => return None,
    };
    let mut layout = compact_layout(pattern);
    if outputs == 16 && output_gap > 0 {
        // Each final splitter supplies one output at a native pitch of two.
        layout.output_rows = layout.output_rows.into_iter().step_by(2).collect();
        if output_gap > 1 {
            // Keep the unused splitter outlets clear of the fan-out turns.
            for &y in &layout.output_rows {
                push_belt(&mut layout.parts, layout.width, y, EAST);
            }
            layout.width += 1;
        }
    }
    Some(layout)
}
