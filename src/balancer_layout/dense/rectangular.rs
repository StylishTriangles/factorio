use super::{append_seed_groups, connect_row, shaft, splitter, underground};
use crate::balancer_layout::{
    EAST, Kind, Layout, NORTH, PATTERNS, Part, SOUTH, compact_layout, push_belt,
};

pub(super) fn generate(outputs: i32) -> Option<Layout> {
    const GROUP: i32 = 8;
    // Column and tunnel breaks for each expanding circuit. Staggering the
    // mouths leaves room for the horizontal belts to cross.
    const EXPANSION: [(i32, i32, i32, i32); GROUP as usize] = [
        (31, 2, 10, 6),
        (3, 1, 9, 5),
        (41, 2, 10, 5),
        (16, 1, 10, 5),
        (27, 1, 5, 5),
        (9, 2, 5, 9),
        (37, 1, 5, 9),
        (23, 1, 9, 10),
    ];
    let expanding = outputs > 3 * GROUP;
    let height = (3 * GROUP).max(outputs);
    let placements = if expanding {
        EXPANSION
    } else {
        // Stagger consecutive rows into even then odd columns.
        std::array::from_fn(|lane| {
            let lane = lane as i32;
            let slot = 2 * lane % GROUP + lane / (GROUP / 2);
            (3 + 4 * slot, 1, 0, 0)
        })
    };
    let width = placements.iter().map(|&(x, ..)| x).max()? + 6;
    let seed = compact_layout(PATTERNS.get("8x8")?);
    let mut parts = Vec::new();
    append_seed_groups(&mut parts, &seed, 3, 0);

    // After balancing each group of eight, combine corresponding lanes from
    // the three input groups.
    let mut mixer = Vec::new();
    let mut unused_pins = Vec::new();
    for (lane, (x, north, south_a, south_b)) in (0..GROUP).zip(placements) {
        let (a, b, c, d) = (lane, lane + GROUP, lane + 2 * GROUP, lane + 3 * GROUP);
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
        underground(&mut mixer, (x + 2, c - 1), (x + 2, b - north), NORTH);
        underground(&mut mixer, (x + 2, b - north - 1), (x + 2, a + 2), NORTH);
        push_belt(&mut mixer, x + 2, a + 1, EAST);

        splitter(&mut mixer, x + 2, b);
        splitter(&mut mixer, x + 3, a);
        push_belt(&mut mixer, x + 1, a, EAST);
        push_belt(&mut mixer, x + 2, a, EAST);
        // A belt at any of these tiles would reopen an unused splitter pin.
        unused_pins.push((x - 1, c - 1));
        if expanding {
            // Carry the other two outlets down to the third and fourth banks.
            for (column, from, to, tunnel_break) in [
                (x + 4, a + 1, c, a + south_a),
                (x + 3, b + 1, d, b + south_b),
            ] {
                push_belt(&mut mixer, column, from, SOUTH);
                underground(
                    &mut mixer,
                    (column, from + 1),
                    (column, tunnel_break),
                    SOUTH,
                );
                underground(
                    &mut mixer,
                    (column, tunnel_break + 1),
                    (column, to - 1),
                    SOUTH,
                );
                push_belt(&mut mixer, column, to, EAST);
            }
        } else {
            unused_pins.extend([(x + 3, b + 1), (x + 4, a + 1)]);
        }
    }

    let mut occupied = vec![false; (height * width) as usize];
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
    for (lane, (x, ..)) in (0..GROUP).zip(placements) {
        let (a, b, c, d) = (lane, lane + GROUP, lane + 2 * GROUP, lane + 3 * GROUP);
        let inputs_and_outputs = [
            (a, 0, x),
            (a, x + 4, width),
            (b, 0, x - 1),
            (b, x + 3, width),
            (c, 0, x),
            (c, x + 5, width),
            (d, x + 4, width),
        ];
        for (y, first, last) in inputs_and_outputs
            .into_iter()
            .take(if expanding { 7 } else { 5 })
        {
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
    append_seed_groups(&mut parts, &seed, outputs / GROUP, seed_span + width - 1);
    Some(Layout {
        width: width + 2 * seed_span,
        height,
        input_rows: (0..3 * GROUP).collect(),
        output_rows: (0..outputs).collect(),
        parts,
        ..Layout::default()
    })
}
