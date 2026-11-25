use super::{
    EAST, EXPRESS_TUNNEL_SPAN, Kind, Layout, NORTH, PATTERNS, Part, SOUTH, compact_layout,
    east_belts, push_belt,
};

mod rectangular;
mod square_24;

pub(super) fn generate(inputs: usize, outputs: usize) -> Option<Layout> {
    if inputs == 24 && matches!(outputs, 16 | 32) {
        return rectangular::generate(outputs as i32);
    }
    if inputs != outputs || inputs == 0 || !inputs.is_multiple_of(2) {
        return None;
    }
    let mut layout = if inputs == 24 {
        square_24::generate().or_else(|| square(inputs))?
    } else {
        square(inputs)?
    };
    // Mix adjacent outlet pairs so alternating blocked outlets still leave an
    // exit for every route through the paired halves.
    for y in (0..layout.height).step_by(2) {
        splitter(&mut layout.parts, layout.width, y);
    }
    for y in 0..layout.height {
        push_belt(&mut layout.parts, layout.width + 1, y, EAST);
    }
    layout.width += 2;
    Some(layout)
}

fn square(size: usize) -> Option<Layout> {
    if let Some(pattern) = PATTERNS.get(&format!("{size}x{size}")) {
        let layout = compact_layout(pattern);
        let rows = 0..size as i32;
        return (layout.height == size as i32
            && layout.input_rows.iter().copied().eq(rows.clone())
            && layout.output_rows.iter().copied().eq(rows))
        .then_some(layout);
    }
    if !size.is_multiple_of(2) {
        return None;
    }

    // Two balanced halves can be fully mixed by pairing each outlet of one with
    // a different outlet of the other. The pairing need not permute either half.
    let half = square(size / 2)?;
    let offset = half.width - 1;
    let (width, mixer) = mix_halves(half.height)?;
    let mut parts = Vec::with_capacity(2 * half.parts.len() + mixer.len());
    parts.extend(half.parts.iter().copied());
    parts.extend(half.parts.into_iter().map(|mut part| {
        part.y += half.height;
        part
    }));
    // Both sections use a straight belt as the shared boundary tile.
    parts.extend(
        mixer
            .into_iter()
            .filter(|part| part.x != 0)
            .map(|mut part| {
                part.x += offset;
                part
            }),
    );

    Some(Layout {
        width: width + offset,
        height: size as i32,
        parts,
        input_rows: (0..size as i32).collect(),
        output_rows: (0..size as i32).collect(),
        ..Layout::default()
    })
}

fn mix_halves(half: i32) -> Option<(i32, Vec<Part>)> {
    let width = 3 * half + 4;
    let mut parts = Vec::new();
    // Splitter columns go through the even slots, then the odd slots. That leaves
    // room on adjacent rows for a whole-belt tunnel at each crossing.
    let even_slots = (half + 1) / 2;
    let splitter_x = |lane| {
        let slot = if lane < even_slots {
            2 * lane
        } else {
            2 * (lane - even_slots) + 1
        };
        3 + 3 * slot
    };
    for lane in 0..half {
        let x = splitter_x(lane);
        parts.push(Part {
            x,
            y: lane,
            kind: Kind::Splitter,
            direction: EAST,
        });

        let upper = lane + 1;
        let lower = lane + half;
        push_belt(&mut parts, x - 1, lower, NORTH);
        push_belt(&mut parts, x - 1, upper, EAST);
        push_belt(&mut parts, x + 1, upper, SOUTH);
        push_belt(&mut parts, x + 1, lower, EAST);

        let limit = lower - 1;
        let mut first = upper + 1;
        while first <= limit {
            let last = (first + EXPRESS_TUNNEL_SPAN - 1).min(limit);
            if first == last {
                push_belt(&mut parts, x - 1, first, NORTH);
                push_belt(&mut parts, x + 1, first, SOUTH);
            } else {
                underground(&mut parts, (x - 1, last), (x - 1, first), NORTH);
                underground(&mut parts, (x + 1, first), (x + 1, last), SOUTH);
            }
            first = last + 1;
        }
    }

    let mut occupied = vec![false; (2 * half * width) as usize];
    for part in &parts {
        let tile = (part.y * width + part.x) as usize;
        if std::mem::replace(&mut occupied[tile], true)
            || (part.kind == Kind::Splitter
                && std::mem::replace(&mut occupied[tile + width as usize], true))
        {
            return None;
        }
    }
    for lane in 0..half {
        let x = splitter_x(lane);
        for (y, first, last) in [
            (lane, 0, x),
            (lane, x + 1, width),
            (lane + half, 0, x - 1),
            (lane + half, x + 2, width),
        ] {
            let row = &occupied[(y * width) as usize..((y + 1) * width) as usize];
            connect_row(&mut parts, row, first, last, y)?;
        }
    }
    Some((width, parts))
}

fn connect_row(
    parts: &mut Vec<Part>,
    occupied: &[bool],
    first: i32,
    last: i32,
    y: i32,
) -> Option<()> {
    let mut blocks = (first..last).filter(|&x| occupied[x as usize]).peekable();
    let mut next = first;
    while let Some(block) = blocks.next() {
        let entry = block - 1;
        let mut exit = block + 1;
        // Nearby obstacles must share a tunnel so oncoming belts remain straight
        // at all underground mouths.
        while let Some(block) = blocks.next_if(|&block| block <= exit + 1) {
            exit = block + 1;
        }
        if entry < next || exit >= last || exit - entry > EXPRESS_TUNNEL_SPAN {
            return None;
        }
        east_belts(parts, next, entry, y);
        underground(parts, (entry, y), (exit, y), EAST);
        next = exit + 1;
    }
    east_belts(parts, next, last, y);
    Some(())
}

fn underground(parts: &mut Vec<Part>, entry: (i32, i32), exit: (i32, i32), direction: u8) {
    for ((x, y), kind) in [(entry, Kind::Input), (exit, Kind::Output)] {
        parts.push(Part {
            x,
            y,
            kind,
            direction,
        });
    }
}

fn append_seed_groups(parts: &mut Vec<Part>, seed: &Layout, groups: i32, x: i32) {
    for group in 0..groups {
        parts.extend(
            seed.parts
                .iter()
                .filter(|part| x == 0 || part.x != 0)
                .map(|part| Part {
                    x: part.x + x,
                    y: part.y + group * seed.height,
                    ..*part
                }),
        );
    }
}

fn splitter(parts: &mut Vec<Part>, x: i32, y: i32) {
    parts.push(Part {
        x,
        y,
        direction: EAST,
        kind: Kind::Splitter,
    });
}

fn shaft(parts: &mut Vec<Part>, x: i32, from: i32, to: i32, direction: u8) {
    let step = if direction == NORTH { -1 } else { 1 };
    push_belt(parts, x, from, direction);
    underground(parts, (x, from + step), (x, to - step), direction);
    push_belt(parts, x, to, EAST);
}
