use super::{append_seed_groups, connect_row, shaft, splitter, underground};
use crate::balancer_layout::{
    EAST, Kind, Layout, NORTH, PATTERNS, Part, SOUTH, compact_layout, push_belt,
};

pub(super) fn generate() -> Option<Layout> {
    let seed = compact_layout(PATTERNS.get("8x8")?);
    let group_size = seed.height;
    let mut parts = Vec::new();
    append_seed_groups(&mut parts, &seed, 3, 0);

    // After balancing each group of eight, combine corresponding lanes from
    // the three groups. Each reducer splits the combined flow equally in two.
    let width = 4 * (group_size - 1) + 9;
    // Stagger consecutive rows into even then odd columns, leaving room for
    // the horizontal tunnels where neighboring circuits share a column.
    let splitter_x = |lane| 3 + 4 * (2 * lane % group_size + lane / (group_size / 2));
    let mut mixer = Vec::new();
    let mut unused_pins = Vec::new();
    for lane in 0..group_size {
        let x = splitter_x(lane);
        let (a, b, c) = (lane, lane + group_size, lane + 2 * group_size);
        // Balance A and B, then send one branch back to row B.
        splitter(&mut mixer, x, a);
        shaft(&mut mixer, x - 1, b, a + 1, NORTH);
        shaft(&mut mixer, x + 1, a + 1, b, SOUTH);

        // Split C and feed one branch to each of the final mergers.
        splitter(&mut mixer, x, c - 1);
        shaft(&mut mixer, x + 1, c - 1, b + 1, NORTH);
        push_belt(&mut mixer, x + 1, c, EAST);
        push_belt(&mut mixer, x + 2, c, NORTH);
        // Keep the northbound branch underground beneath both tiles of the merger.
        underground(&mut mixer, (x + 2, c - 1), (x + 2, b - 1), NORTH);
        underground(&mut mixer, (x + 2, b - 2), (x + 2, a + 2), NORTH);
        push_belt(&mut mixer, x + 2, a + 1, EAST);

        splitter(&mut mixer, x + 2, b);
        splitter(&mut mixer, x + 3, a);
        push_belt(&mut mixer, x + 1, a, EAST);
        push_belt(&mut mixer, x + 2, a, EAST);
        // A belt at any of these tiles would reopen an unused splitter pin.
        unused_pins.extend([(x - 1, c - 1), (x + 3, b + 1), (x + 4, a + 1)]);
    }

    let mut occupied = vec![false; (3 * group_size * width) as usize];
    for (x, y) in mixer
        .iter()
        .flat_map(|part| {
            [(part.x, part.y), (part.x, part.y + 1)]
                .into_iter()
                .take(if part.kind == Kind::Splitter { 2 } else { 1 })
        })
        .chain(unused_pins)
    {
        if std::mem::replace(&mut occupied[(y * width + x) as usize], true) {
            return None;
        }
    }
    for lane in 0..group_size {
        let x = splitter_x(lane);
        for (y, first, last) in [
            (lane, 0, x),
            (lane, x + 4, width),
            (lane + group_size, 0, x - 1),
            (lane + group_size, x + 3, width),
            (lane + 2 * group_size, 0, x),
        ] {
            let row = &occupied[(y * width) as usize..((y + 1) * width) as usize];
            connect_row(&mut mixer, row, first, last, y)?;
        }
    }

    // Join the mixer with the seeds on their shared straight-belt column.
    let seed_span = seed.width - 1;
    parts.extend(
        mixer
            .into_iter()
            .filter(|part| part.x != 0)
            .map(|part| Part {
                x: part.x + seed_span,
                ..part
            }),
    );
    // Let an open outlet draw from the whole bank when its neighbors back up.
    // The last tunnel exit feeds the next splitter directly at the join.
    append_seed_groups(&mut parts, &seed, 2, seed_span + width - 1);
    Some(Layout {
        width: width + 2 * seed_span,
        height: 3 * group_size,
        input_rows: (0..3 * group_size).collect(),
        output_rows: (0..2 * group_size).collect(),
        parts,
        ..Layout::default()
    })
}
