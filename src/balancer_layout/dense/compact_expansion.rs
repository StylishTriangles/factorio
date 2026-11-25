use super::{append_seed_groups, connect_row, underground};
use crate::balancer_layout::{
    EAST, EXPRESS_TUNNEL_SPAN, Layout, NORTH, PATTERNS, Part, SOUTH, compact_layout, push_belt,
};

pub(super) fn generate() -> Option<Layout> {
    const INPUTS: i32 = 24;
    const OUTPUTS: i32 = 32;
    const SHUFFLE_WIDTH: i32 = 17;
    // Each four-row group receives one lane from each input bank.
    const DESTINATIONS: [i32; INPUTS as usize] = [
        20, 24, 28, 0, 4, 8, 12, 16, 30, 2, 6, 10, 14, 18, 22, 26, 1, 5, 9, 13, 17, 21, 25, 29,
    ];
    // Stagger the vertical turns so horizontal tunnels can cross them.
    const COLUMNS: [i32; INPUTS as usize] = [
        11, 3, 7, 12, 0, 5, 13, 12, 4, 9, 14, 15, 9, 13, 5, 1, 8, 6, 10, 15, 8, 0, 2, 5,
    ];
    let routes = || (0..INPUTS).zip(DESTINATIONS).zip(COLUMNS);
    let mut shuffle = Vec::new();
    for ((from, to), x) in routes().filter(|((from, to), _)| from != to) {
        let step = (to - from).signum();
        let direction = if step > 0 { SOUTH } else { NORTH };
        push_belt(&mut shuffle, x, from, direction);
        push_belt(&mut shuffle, x, to, EAST);
        let mut entry = from + step;
        let last = to - step;
        while step * (last - entry) > 0 {
            let distance = (last - entry).abs().min(EXPRESS_TUNNEL_SPAN);
            let exit = entry + step * distance;
            underground(&mut shuffle, (x, entry), (x, exit), direction);
            entry = exit + step;
        }
        if entry == last {
            push_belt(&mut shuffle, x, entry, direction);
        }
    }

    let mut occupied = [[false; SHUFFLE_WIDTH as usize]; OUTPUTS as usize];
    for part in &shuffle {
        if std::mem::replace(&mut occupied[part.y as usize][part.x as usize], true) {
            return None;
        }
    }
    for ((from, to), x) in routes() {
        if from == to {
            connect_row(
                &mut shuffle,
                &occupied[from as usize],
                0,
                SHUFFLE_WIDTH,
                from,
            )?;
        } else {
            connect_row(&mut shuffle, &occupied[from as usize], 0, x, from)?;
            connect_row(
                &mut shuffle,
                &occupied[to as usize],
                x + 1,
                SHUFFLE_WIDTH,
                to,
            )?;
        }
    }

    let input_bank = compact_layout(PATTERNS.get("8x8")?);
    let output_group = compact_layout(PATTERNS.get("4x4")?);
    let input_join = input_bank.width - 1;
    let output_join = input_join + SHUFFLE_WIDTH - 1;
    let mut parts = Vec::new();
    append_seed_groups(&mut parts, &input_bank, INPUTS / input_bank.height, 0);
    // Underground mouths can take the place of straight belts at both joins.
    parts.retain(|part| part.x < input_join);
    parts.extend(shuffle.into_iter().map(|part| Part {
        x: part.x + input_join,
        ..part
    }));
    append_seed_groups(
        &mut parts,
        &output_group,
        OUTPUTS / output_group.height,
        output_join,
    );
    Some(Layout {
        width: output_join + output_group.width,
        height: OUTPUTS,
        parts,
        input_rows: (0..INPUTS).collect(),
        output_rows: (0..OUTPUTS).collect(),
        ..Layout::default()
    })
}
