use super::{append_seed_groups, connect_row, shaft, splitter};
use crate::balancer_layout::{
    EAST, Kind, Layout, NORTH, PATTERNS, Part, SOUTH, compact_layout, push_belt,
};

const GROUP: i32 = 6;

pub(super) fn generate() -> Option<Layout> {
    const HEIGHT: i32 = 4 * GROUP;
    // Stagger the six circuits so their tunnels and surface crossings can
    // share columns. Each one combines the corresponding lanes of four banks.
    const COLUMNS: [i32; GROUP as usize] = [10, 3, 17, 38, 24, 31];
    let width = COLUMNS.into_iter().max().unwrap() + 7;
    let seed = compact_layout(PATTERNS.get("6x6")?);
    let mut parts = Vec::new();
    append_seed_groups(&mut parts, &seed, 4, 0);

    let mut mixer = Vec::new();
    for (lane, x) in (0..GROUP).zip(COLUMNS) {
        let start = mixer.len();
        let (a, b, c, d) = (lane, lane + GROUP, lane + 2 * GROUP, lane + 3 * GROUP);
        // The first two pairs are vertically disjoint and use the same columns.
        for (top, bottom) in [(a, b), (c, d)] {
            splitter(&mut mixer, x, top);
            shaft(&mut mixer, x - 1, bottom, top + 1, NORTH);
            shaft(&mut mixer, x + 1, top + 1, bottom, SOUTH);
        }

        // The second layer mixes A with C and B with D. The C return passes
        // beneath the B/D splitter, overlapping the two layers in the layout.
        splitter(&mut mixer, x + 3, a);
        splitter(&mut mixer, x + 4, b);
        push_belt(&mut mixer, x + 1, c, EAST);
        push_belt(&mut mixer, x + 2, d, EAST);
        shaft(&mut mixer, x + 2, c, a + 1, NORTH);
        shaft(&mut mixer, x + 3, d, b + 1, NORTH);
        shaft(&mut mixer, x + 4, a + 1, c, SOUTH);
        shaft(&mut mixer, x + 5, b + 1, d, SOUTH);
        // Mirroring alternate circuits separates nearby tunnel mouths while
        // preserving which four rows the circuit combines.
        if lane % 2 == 0 {
            for part in &mut mixer[start..] {
                part.y = mixer_row(lane, part.y) - i32::from(part.kind == Kind::Splitter);
                if matches!(part.direction, NORTH | SOUTH) {
                    part.direction = SOUTH - part.direction;
                }
            }
        }
    }

    let mut occupied = vec![false; (HEIGHT * width) as usize];
    for part in &mixer {
        let tile = (part.y * width + part.x) as usize;
        if std::mem::replace(&mut occupied[tile], true)
            || (part.kind == Kind::Splitter
                && std::mem::replace(&mut occupied[tile + width as usize], true))
        {
            return None;
        }
    }
    for (lane, x) in (0..GROUP).zip(COLUMNS) {
        let (a, b, c, d) = (lane, lane + GROUP, lane + 2 * GROUP, lane + 3 * GROUP);
        for (y, first, last) in [
            (a, 0, x),
            (a, x + 1, x + 3),
            (a, x + 4, width),
            (b, 0, x - 1),
            (b, x + 2, x + 4),
            (b, x + 5, width),
            (c, 0, x),
            (c, x + 5, width),
            (d, 0, x - 1),
            (d, x + 6, width),
        ] {
            let y = mixer_row(lane, y);
            let row = &occupied[(y * width) as usize..((y + 1) * width) as usize];
            connect_row(&mut mixer, row, first, last, y)?;
        }
    }

    // Keep the existing seed's belts on the shared input column of the mixer.
    let offset = seed.width - 1;
    parts.extend(
        mixer
            .into_iter()
            .filter(|part| part.x != 0)
            .map(|part| Part {
                x: part.x + offset,
                ..part
            }),
    );
    Some(Layout {
        width: width + offset,
        height: HEIGHT,
        input_rows: (0..HEIGHT).collect(),
        output_rows: (0..HEIGHT).collect(),
        parts,
        ..Layout::default()
    })
}

fn mixer_row(lane: i32, y: i32) -> i32 {
    if lane % 2 == 0 {
        2 * lane + 3 * GROUP - y
    } else {
        y
    }
}
