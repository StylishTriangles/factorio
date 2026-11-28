use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

mod dense;

pub const MAX_LANES: usize = 64;
pub const MAX_GAP: usize = 13;

const NORTH: u8 = 0;
const EAST: u8 = 1;
const SOUTH: u8 = 2;
const WEST: u8 = 3;
const DIRECTIONS: [Tile; 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
const EXPRESS_TUNNEL_SPAN: i32 = 9;

type Tile = (i32, i32);

// Factorio-SAT books/blue_narrow.txt (blob bb6b24caf70662973450417a8796d036677e382d).
// https://github.com/R-O-C-K-E-T/Factorio-SAT/blob/main/books/blue_narrow.txt
// Compact layouts are GPL-3.0. See site/balancers/layouts.LICENSE.txt.
// The 5→3 and 8→4 catalog layouts are omitted because subsets of inputs bottleneck even with all outputs open.
static PATTERNS: LazyLock<HashMap<String, Vec<String>>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../data/balancer_layouts.json"))
        .expect("embedded balancer patterns must be valid JSON")
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Belt,
    Splitter,
    Input,
    Output,
}

#[derive(Clone, Copy, Debug)]
pub struct Part {
    pub x: i32,
    pub y: i32,
    pub kind: Kind,
    pub direction: u8,
}

#[derive(Debug, Default)]
pub struct Layout {
    pub width: i32,
    pub height: i32,
    pub parts: Vec<Part>,
    pub input_rows: Vec<i32>,
    pub output_rows: Vec<i32>,
    pub feeds: HashMap<Tile, Vec<u8>>,
    pub tunnels: Vec<(Tile, Tile)>,
}

pub fn get_layout(
    inputs: usize,
    outputs: usize,
    input_gap: usize,
    output_gap: usize,
) -> Option<Layout> {
    get_layout_variant(inputs, outputs, input_gap, output_gap, false)
}

pub(crate) fn get_layout_variant(
    inputs: usize,
    outputs: usize,
    input_gap: usize,
    output_gap: usize,
    use_compact: bool,
) -> Option<Layout> {
    if !(1..=MAX_LANES).contains(&inputs)
        || !(1..=MAX_LANES).contains(&outputs)
        || input_gap > MAX_GAP
        || output_gap > MAX_GAP
    {
        return None;
    }

    let with_ports = |mut layout: Layout| {
        route_ports(&mut layout, input_gap, output_gap);
        layout
    };
    let mut layout = if let Some(pattern) = PATTERNS.get(&format!("{inputs}x{outputs}")) {
        with_ports(compact_layout(pattern))
    } else {
        let standard = with_ports(generate_layout(inputs, outputs));
        let score = |layout: &Layout| (layout.width * layout.height, layout.parts.len());
        dense::generate(inputs, outputs, output_gap, use_compact)
            .map(with_ports)
            .filter(|candidate| score(candidate) < score(&standard))
            .unwrap_or(standard)
    };
    index_tracks(&mut layout);
    tunnel_straight_runs(&mut layout);
    Some(layout)
}

fn compact_layout(pattern: &[String]) -> Layout {
    let width = pattern[0].split_ascii_whitespace().count() as i32;
    let mut layout = Layout {
        width,
        height: pattern.len() as i32,
        ..Layout::default()
    };

    for (y, row) in pattern.iter().enumerate() {
        let y = y as i32;
        for (x, tile) in row.split_ascii_whitespace().enumerate() {
            if matches!(tile, "." | "s") {
                continue;
            }
            let x = x as i32;
            let kind = match tile.as_bytes()[0] {
                b'S' => Kind::Splitter,
                b'i' => Kind::Input,
                b'o' => Kind::Output,
                _ => Kind::Belt,
            };
            let direction = match tile.as_bytes().last() {
                Some(b'^') => NORTH,
                Some(b'>') => EAST,
                Some(b'v') => SOUTH,
                Some(b'<') => WEST,
                _ => unreachable!("embedded pattern has invalid direction"),
            };
            layout.parts.push(Part {
                x,
                y,
                kind,
                direction,
            });
            let port_rows = y..(y + 1 + i32::from(kind == Kind::Splitter && direction == EAST));
            if x == 0 {
                layout.input_rows.extend(port_rows.clone());
            }
            if x == width - 1 && direction == EAST && kind != Kind::Input {
                layout.output_rows.extend(port_rows);
            }
        }
    }
    layout
}

fn push_belt(parts: &mut Vec<Part>, x: i32, y: i32, direction: u8) {
    parts.push(Part {
        x,
        y,
        direction,
        kind: Kind::Belt,
    });
}

fn east_belts(parts: &mut Vec<Part>, from: i32, to: i32, y: i32) {
    for x in from..to {
        push_belt(parts, x, y, EAST);
    }
}

fn lane_row(lane: usize, bank: usize) -> i32 {
    (4 * lane + 2 * bank) as i32
}

fn lane_for(index: usize, bit: usize) -> usize {
    let low_bits = (1 << bit) - 1;
    (index & !(low_bits * 2 + 1)) | ((index & low_bits) << 1) | ((index >> bit) & 1)
}

fn splitter_column(
    parts: &mut Vec<Part>,
    x: i32,
    bank: usize,
    capacity: usize,
    input_open: impl Fn(usize) -> bool,
    output_open: impl Fn(usize) -> bool,
) -> i32 {
    for lane in (0..capacity).step_by(2) {
        let upper_input = input_open(lane);
        let lower_input = input_open(lane + 1);
        let upper_output = output_open(lane);
        let lower_output = output_open(lane + 1);
        if !(upper_input || lower_input || upper_output || lower_output) {
            continue;
        }
        let upper = lane_row(lane, bank);
        let lower = lane_row(lane + 1, bank);
        parts.push(Part {
            x: x + 1,
            y: upper,
            kind: Kind::Splitter,
            direction: EAST,
        });
        if upper_input {
            push_belt(parts, x, upper, EAST);
        }
        if lower_input {
            for y in (upper + 2..=lower).rev() {
                push_belt(parts, x, y, NORTH);
            }
            push_belt(parts, x, upper + 1, EAST);
        }
        if upper_output {
            push_belt(parts, x + 2, upper, EAST);
        }
        if lower_output {
            for y in upper + 1..lower {
                push_belt(parts, x + 2, y, SOUTH);
            }
            push_belt(parts, x + 2, lower, EAST);
        }
    }
    x + 3
}

fn connect_columns(parts: &mut Vec<Part>, x: i32, tracks: impl Iterator<Item = (i32, i32)>) -> i32 {
    // Interleaved row banks leave space to tunnel under horizontal belts.
    let mut wires: Vec<_> = tracks.map(|(from, to)| (from, to, 0)).collect();
    wires.sort_unstable_by_key(|(from, to, _)| (*from).min(*to));
    let mut bottoms = Vec::new();
    let mut bends = HashMap::new();
    for (from, to, turn_x) in &mut wires {
        let top = (*from).min(*to);
        let bottom = (*from).max(*to);
        let column = match bottoms.iter().position(|last| *last < top) {
            Some(column) => {
                bottoms[column] = bottom;
                column
            }
            None => {
                bottoms.push(bottom);
                bottoms.len() - 1
            }
        };
        *turn_x = x + column as i32;
        bends.insert(*from, (*turn_x, true));
        bends.insert(*to, (*turn_x, false));
    }

    let end = x + bottoms.len() as i32;
    for (from, to, turn_x) in wires {
        east_belts(parts, x, turn_x, from);
        east_belts(parts, turn_x + 1, end, to);
        let step = (to - from).signum();
        let heading = if step > 0 { SOUTH } else { NORTH };
        let crosses = |y| {
            y > from.min(to)
                && y < from.max(to)
                && bends.get(&y).is_some_and(
                    |&(at, source)| {
                        if source { turn_x < at } else { turn_x > at }
                    },
                )
        };
        let mut y = from;
        while y != to {
            if crosses(y + step) {
                // Two close crossings share one tunnel so their endpoints can't overlap.
                let mut exit = y + 2 * step;
                while crosses(exit) || crosses(exit + step) {
                    exit += step;
                }
                debug_assert!(exit != to && (exit - y).abs() <= EXPRESS_TUNNEL_SPAN);
                parts.push(Part {
                    x: turn_x,
                    y,
                    kind: Kind::Input,
                    direction: heading,
                });
                parts.push(Part {
                    x: turn_x,
                    y: exit,
                    kind: Kind::Output,
                    direction: heading,
                });
                y = exit;
            } else {
                push_belt(parts, turn_x, y, heading);
            }
            y += step;
        }
        push_belt(parts, turn_x, to, EAST);
    }
    end
}

fn lane_mask(count: usize) -> u64 {
    u64::MAX >> (u64::BITS as usize - count)
}

fn has_lane(mask: u64, lane: usize) -> bool {
    mask & (1 << lane) != 0
}

fn through_splitters(mut lanes: u64, bit: usize, capacity: usize) -> u64 {
    for lane in 0..capacity {
        if has_lane(lanes, lane) {
            lanes |= 1 << (lane ^ (1 << bit));
        }
    }
    lanes
}

fn connected_lanes(stages: &[usize], capacity: usize, inputs: u64, outputs: u64) -> Vec<u64> {
    // Route only wires that an input can reach and that can reach an output.
    let mut live = vec![0; stages.len() + 1];
    live[0] = inputs;
    for (stage, &bit) in stages.iter().enumerate() {
        live[stage + 1] = through_splitters(live[stage], bit, capacity);
    }

    let mut upstream = outputs;
    live[stages.len()] &= outputs;
    for (stage, &bit) in stages.iter().enumerate().rev() {
        upstream = through_splitters(upstream, bit, capacity);
        live[stage] &= upstream;
    }
    live
}

// Splitter columns connect pairs differing in one bit. The reverse columns
// form a Beneš network when reducing the number of external belts.
fn square(
    parts: &mut Vec<Part>,
    start: i32,
    size: usize,
    open_inputs: usize,
    open_outputs: usize,
    reverse_stages: bool,
) -> (i32, usize) {
    if size == 1 {
        east_belts(parts, start, start + 3, 0);
        return (start + 2, 0);
    }

    let capacity = size.next_power_of_two();
    let feedback = capacity - size;
    let bits = capacity.ilog2() as usize;
    let mut stages: Vec<_> = (0..bits).collect();
    if reverse_stages {
        stages.extend((0..bits - 1).rev());
    }
    let feedback_lanes = lane_mask(capacity) ^ lane_mask(size);
    let live = connected_lanes(
        &stages,
        capacity,
        lane_mask(open_inputs) | feedback_lanes,
        lane_mask(open_outputs) | feedback_lanes,
    );

    let first = start + feedback as i32 + 1;
    let mut x = first;
    let mut previous_rows = [0; MAX_LANES];
    for (stage, &bit) in stages.iter().enumerate() {
        let bank = stage % 2;
        let mut order = vec![0; capacity];
        for lane in 0..capacity {
            order[lane_for(lane, bit)] = lane;
        }
        if reverse_stages {
            let used = live[stage] | live[stage + 1];
            order = order
                .as_chunks::<2>()
                .0
                .iter()
                .filter(|pair| pair.iter().any(|&lane| has_lane(used, lane)))
                .flatten()
                .copied()
                .collect();
        }
        let mut rows = [0; MAX_LANES];
        for (lane, &logical) in order.iter().enumerate() {
            rows[logical] = lane_row(lane, bank);
        }
        if stage > 0 {
            let tracks = (0..capacity)
                .filter(|&lane| has_lane(live[stage], lane))
                .map(|lane| (previous_rows[lane], rows[lane]));
            x = connect_columns(parts, x, tracks);
        }
        x = splitter_column(
            parts,
            x,
            bank,
            order.len(),
            |lane| has_lane(live[stage], order[lane]),
            |lane| has_lane(live[stage + 1], order[lane]),
        );
        previous_rows = rows;
    }

    let bank = (stages.len() - 1) % 2;
    let end = x + feedback as i32 + 1;
    for lane in 0..open_inputs {
        east_belts(parts, start, first, lane_row(lane, 0));
    }
    for lane in 0..open_outputs {
        east_belts(parts, x, end + 1, lane_row(lane, bank));
    }

    // Each surplus output wraps beneath the network to its corresponding input.
    for index in 0..feedback {
        let lane = size + index;
        let inlet = lane_row(lane, 0);
        let outlet = lane_row(lane, bank);
        let left = start + 1 + index as i32;
        let right = end - 1 - index as i32;
        let bottom = lane_row(capacity, bank) + (feedback - index) as i32;
        east_belts(parts, x, right, outlet);
        for y in outlet..bottom {
            push_belt(parts, right, y, SOUTH);
        }
        for column in (left + 1..=right).rev() {
            push_belt(parts, column, bottom, WEST);
        }
        for y in (inlet + 1..=bottom).rev() {
            push_belt(parts, left, y, NORTH);
        }
        east_belts(parts, left, first, inlet);
    }

    (end, bank)
}

fn generate_layout(inputs: usize, outputs: usize) -> Layout {
    let mut parts = Vec::new();
    let reduction = (inputs > outputs).then(|| {
        square(
            &mut parts,
            0,
            inputs.next_power_of_two(),
            inputs,
            outputs,
            true,
        )
    });
    // For a power-of-two output count, the final reverse columns already
    // balance exactly those outlets as a square butterfly.
    let (end, bank) = match reduction {
        Some(reduction) if outputs.is_power_of_two() => reduction,
        _ => square(
            &mut parts,
            reduction.map_or(0, |(end, _)| end + 1),
            outputs,
            inputs.min(outputs),
            outputs,
            false,
        ),
    };
    let height = parts
        .iter()
        .map(|part| part.y + if part.kind == Kind::Splitter { 2 } else { 1 })
        .max()
        .expect("generated layout has at least one belt");

    Layout {
        width: end + 1,
        height,
        parts,
        input_rows: (0..inputs).map(|lane| lane_row(lane, 0)).collect(),
        output_rows: (0..outputs).map(|lane| lane_row(lane, bank)).collect(),
        ..Layout::default()
    }
}

struct PortRoutes {
    rows: Vec<i32>,
    bends: Vec<i32>,
    width: i32,
}

impl PortRoutes {
    fn new(track_rows: &[i32], gap: usize) -> Self {
        let pitch = gap as i32 + 1;
        let span = (track_rows.len() as i32 - 1) * pitch;
        let top = (track_rows[0] + track_rows[track_rows.len() - 1] - span).div_euclid(2);
        let rows: Vec<_> = (0..track_rows.len())
            .map(|lane| top + lane as i32 * pitch)
            .collect();

        // Overlapping bends turn from the outside in; disjoint ones can share a column.
        let mut bends = vec![0; rows.len()];
        for lane in (0..rows.len()).rev() {
            if rows[lane] >= track_rows[lane] {
                continue;
            }
            let mut bend = 1;
            for lower in lane + 1..rows.len() {
                if rows[lower] > track_rows[lane] {
                    break;
                }
                bend = bend.max(bends[lower] + 1);
            }
            bends[lane] = bend;
        }
        for lane in 0..rows.len() {
            if rows[lane] <= track_rows[lane] {
                continue;
            }
            let mut bend = 1;
            for upper in (0..lane).rev() {
                if rows[upper] < track_rows[lane] {
                    break;
                }
                bend = bend.max(bends[upper] + 1);
            }
            bends[lane] = bend;
        }
        let bend = bends.iter().copied().max().unwrap_or_default();
        let width = if bend == 0 { 0 } else { bend + 1 };
        Self { rows, bends, width }
    }

    fn append(&self, parts: &mut Vec<Part>, track_rows: &[i32], left: bool, boundary: i32) {
        for (lane, &track_row) in track_rows.iter().enumerate() {
            let port_row = self.rows[lane];
            let bend = self.bends[lane];
            for column in 0..self.width {
                let x = if left {
                    column
                } else {
                    boundary + self.width - 1 - column
                };
                if bend == column && port_row != track_row {
                    let (from, to) = if left {
                        (port_row, track_row)
                    } else {
                        (track_row, port_row)
                    };
                    let heading = if from < to { SOUTH } else { NORTH };
                    for y in from.min(to)..=from.max(to) {
                        push_belt(parts, x, y, if y == to { EAST } else { heading });
                    }
                } else {
                    let y = if column < bend { port_row } else { track_row };
                    push_belt(parts, x, y, EAST);
                }
            }
        }
    }
}

fn route_ports(layout: &mut Layout, input_gap: usize, output_gap: usize) {
    let inputs = PortRoutes::new(&layout.input_rows, input_gap);
    let outputs = PortRoutes::new(&layout.output_rows, output_gap);

    for part in &mut layout.parts {
        part.x += inputs.width;
    }
    inputs.append(&mut layout.parts, &layout.input_rows, true, 0);
    outputs.append(
        &mut layout.parts,
        &layout.output_rows,
        false,
        layout.width + inputs.width,
    );

    let top = inputs.rows[0].min(outputs.rows[0]).min(0);
    if top < 0 {
        for part in &mut layout.parts {
            part.y -= top;
        }
    }
    layout.width += inputs.width + outputs.width;
    layout.height = layout
        .height
        .max(inputs.rows[inputs.rows.len() - 1] + 1)
        .max(outputs.rows[outputs.rows.len() - 1] + 1)
        - top;
    layout.input_rows = inputs.rows.into_iter().map(|row| row - top).collect();
    layout.output_rows = outputs.rows.into_iter().map(|row| row - top).collect();
}

fn tunnel_straight_runs(layout: &mut Layout) {
    // Existing underground belts can block new pairs on the same axis.
    let mut underground = HashSet::new();
    for &((x, y), (end_x, end_y)) in &layout.tunnels {
        let axis = u8::from(y == end_y);
        for tile_x in x.min(end_x)..=x.max(end_x) {
            for tile_y in y.min(end_y)..=y.max(end_y) {
                underground.insert((tile_x, tile_y, axis));
            }
        }
    }

    let straight: HashMap<_, _> = layout
        .parts
        .iter()
        .enumerate()
        .filter_map(|(index, part)| {
            let tile = (part.x, part.y);
            (part.kind == Kind::Belt
                && (1..layout.width - 1).contains(&part.x)
                && layout.feeds.get(&tile).map(Vec::as_slice) == Some(&[part.direction])
                && !underground.contains(&(part.x, part.y, part.direction % 2)))
            .then_some((tile, (index, part.direction)))
        })
        .collect();

    let mut run = Vec::new();
    let mut keep = vec![true; layout.parts.len()];
    for (&(x, y), &(_, direction)) in &straight {
        let (dx, dy) = DIRECTIONS[direction as usize];
        if straight
            .get(&(x - dx, y - dy))
            .is_some_and(|&(_, previous)| previous == direction)
        {
            continue;
        }
        run.clear();
        let (mut x, mut y) = (x, y);
        while let Some(&(index, heading)) = straight.get(&(x, y))
            && heading == direction
        {
            run.push(index);
            x += dx;
            y += dy;
        }
        for section in run.chunks(EXPRESS_TUNNEL_SPAN as usize + 1) {
            if let [entry, middle @ .., exit] = section
                && middle.len() >= 2
            {
                layout.parts[*entry].kind = Kind::Input;
                layout.parts[*exit].kind = Kind::Output;
                for &index in middle {
                    keep[index] = false;
                }
            }
        }
    }
    let mut keep = keep.into_iter();
    layout.parts.retain(|_| keep.next().unwrap());
    index_tracks(layout);
}

fn index_tracks(layout: &mut Layout) {
    layout.feeds.clear();
    layout.tunnels.clear();
    let mut exits = HashMap::new();
    for part in &layout.parts {
        let (dx, dy) = DIRECTIONS[part.direction as usize];
        match part.kind {
            Kind::Input => continue,
            Kind::Output => {
                exits.insert((part.x, part.y, part.direction), (part.x, part.y));
            }
            _ => {}
        }
        let lanes = if part.kind == Kind::Splitter { 2 } else { 1 };
        for lane in 0..lanes {
            let tile = (part.x + dx + dy.abs() * lane, part.y + dy + dx.abs() * lane);
            layout.feeds.entry(tile).or_default().push(part.direction);
        }
    }
    for part in layout.parts.iter().filter(|part| part.kind == Kind::Input) {
        let (dx, dy) = DIRECTIONS[part.direction as usize];
        if let Some(&exit) = (1..=EXPRESS_TUNNEL_SPAN).find_map(|distance| {
            exits.get(&(
                part.x + dx * distance,
                part.y + dy * distance,
                part.direction,
            ))
        }) {
            layout.tunnels.push(((part.x, part.y), exit));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_geometry(layout: &Layout, inputs: usize, outputs: usize, gaps: (usize, usize)) {
        let mut occupied = HashSet::new();
        for part in &layout.parts {
            let (dx, dy) = DIRECTIONS[part.direction as usize];
            let mut claim = |x, y| {
                assert!((0..layout.width).contains(&x) && (0..layout.height).contains(&y));
                assert!(occupied.insert((x, y)), "overlapping parts at {x},{y}");
            };
            claim(part.x, part.y);
            if part.kind == Kind::Splitter {
                claim(part.x + dy.abs(), part.y + dx.abs());
            }
        }
        for (x, rows, count, gap) in [
            (0, &layout.input_rows, inputs, gaps.0),
            (layout.width - 1, &layout.output_rows, outputs, gaps.1),
        ] {
            assert_eq!(rows.len(), count);
            for pair in rows.windows(2) {
                assert_eq!(pair[1] - pair[0], gap as i32 + 1);
            }
            let mut edge_rows: Vec<_> = layout
                .parts
                .iter()
                .filter(|part| part.x == x && (x == 0 || part.direction == EAST))
                .flat_map(|part| {
                    assert_eq!(part.direction, EAST);
                    let ports: Vec<_> = match part.kind {
                        Kind::Belt => vec![part.y],
                        Kind::Output if x != 0 => vec![part.y],
                        Kind::Splitter => [part.y, part.y + 1]
                            .into_iter()
                            .filter(|y| rows.binary_search(y).is_ok())
                            .collect(),
                        _ => panic!("invalid port at {x},{}", part.y),
                    };
                    assert!(!ports.is_empty(), "missing port at {x},{}", part.y);
                    ports
                })
                .collect();
            edge_rows.sort_unstable();
            assert_eq!(&edge_rows, rows);
        }

        let tunnels: HashSet<_> = layout.tunnels.iter().map(|&(_, output)| output).collect();
        assert_eq!(tunnels.len(), layout.tunnels.len());
        assert_eq!(
            layout.tunnels.len(),
            layout
                .parts
                .iter()
                .filter(|part| part.kind == Kind::Input)
                .count()
        );
        assert_eq!(
            layout.tunnels.len(),
            layout
                .parts
                .iter()
                .filter(|part| part.kind == Kind::Output)
                .count()
        );
    }

    #[test]
    fn generated_and_compact_layouts_have_exact_port_spacing() {
        for (inputs, outputs) in [
            (1, 1),
            (1, 8),
            (4, 4),
            (3, 5),
            (12, 12),
            (16, 16),
            (32, 32),
            (9, 6),
            (64, 33),
            (33, 64),
        ] {
            for gaps in [(0, 0), (1, 2), (MAX_GAP, MAX_GAP)] {
                let layout = get_layout(inputs, outputs, gaps.0, gaps.1).unwrap();
                check_geometry(&layout, inputs, outputs, gaps);
            }
        }
        let adjacent = get_layout(12, 12, 0, 0).unwrap();
        assert!(adjacent.height < 80, "oversized 12×12: {}", adjacent.height);
        assert!(adjacent.width < 70, "overly wide 12×12: {}", adjacent.width);
        let compact = get_layout(4, 4, 0, 0).unwrap();
        assert_eq!((compact.width, compact.height), (10, 4));
    }

    #[test]
    fn port_routes_do_not_add_sideloads_to_generated_layouts() {
        for gaps in [(0, 0), (1, 2), (MAX_GAP, 1)] {
            let layout = get_layout(13, 13, gaps.0, gaps.1).unwrap();
            assert!(layout.feeds.values().all(|feeds| feeds.len() == 1));

            let occupied: HashMap<_, _> = layout
                .parts
                .iter()
                .flat_map(|part| {
                    let mut tiles = vec![((part.x, part.y), part)];
                    if part.kind == Kind::Splitter {
                        tiles.push(((part.x, part.y + 1), part));
                    }
                    tiles
                })
                .collect();
            for &((x, y), (end_x, end_y)) in &layout.tunnels {
                let direction = occupied[&(x, y)].direction;
                let (dx, dy) = DIRECTIONS[direction as usize];
                let length = (x - end_x).abs() + (y - end_y).abs();
                assert!((2..=EXPRESS_TUNNEL_SPAN).contains(&length));
                assert_eq!((x + dx * length, y + dy * length), (end_x, end_y));
                for distance in 1..length {
                    if let Some(part) = occupied.get(&(x + dx * distance, y + dy * distance)) {
                        assert!(part.direction != direction || part.kind == Kind::Belt);
                    }
                }
            }
        }
    }

    #[test]
    fn straight_runs_use_tunnels_without_disturbing_side_feeds_or_existing_pairs() {
        let mut layout = Layout {
            width: 22,
            height: 7,
            ..Layout::default()
        };
        east_belts(&mut layout.parts, 0, 4, 0);
        east_belts(&mut layout.parts, 0, 5, 1);
        east_belts(&mut layout.parts, 0, 22, 2);
        east_belts(&mut layout.parts, 0, 7, 3);
        push_belt(&mut layout.parts, 3, 4, NORTH);
        east_belts(&mut layout.parts, 1, 9, 6);
        layout.parts.extend([
            Part {
                x: 9,
                y: 6,
                kind: Kind::Input,
                direction: WEST,
            },
            Part {
                x: 0,
                y: 6,
                kind: Kind::Output,
                direction: WEST,
            },
        ]);
        index_tracks(&mut layout);
        tunnel_straight_runs(&mut layout);

        assert_eq!(
            layout.tunnels.iter().copied().collect::<HashSet<_>>(),
            HashSet::from([
                ((1, 1), (4, 1)),
                ((1, 2), (10, 2)),
                ((11, 2), (20, 2)),
                ((9, 6), (0, 6)),
            ])
        );
        assert_eq!(layout.feeds[&(3, 3)].len(), 2);
        assert!(
            layout
                .parts
                .iter()
                .any(|part| { (part.x, part.y, part.kind) == (21, 2, Kind::Belt) })
        );
    }

    #[test]
    fn reduction_only_places_tracks_between_real_ports() {
        for gaps in [(0, 0), (1, 2)] {
            let layout = get_layout(12, 4, gaps.0, gaps.1).unwrap();
            check_geometry(&layout, 12, 4, gaps);
            assert!(layout.width <= 56 && layout.height <= 63 && layout.parts.len() <= 1_256);

            check_tracks(&layout);
        }
    }

    #[test]
    fn dense_squares_keep_belts_adjacent_and_use_less_space() {
        for (size, max_width) in [(12, 38), (14, 41), (16, 41), (32, 92), (64, 191)] {
            let layout = get_layout(size, size, 0, 0).unwrap();
            assert_eq!(layout.height, size as i32);
            assert!(layout.width <= max_width);
            check_geometry(&layout, size, size, (0, 0));
            check_tracks(&layout);
        }

        for gaps in [(1, 2), (3, 3), (MAX_GAP, MAX_GAP)] {
            let layout = get_layout(16, 16, gaps.0, gaps.1).unwrap();
            check_geometry(&layout, 16, 16, gaps);
            let mut standard = generate_layout(16, 16);
            route_ports(&mut standard, gaps.0, gaps.1);
            assert!(layout.width * layout.height <= standard.width * standard.height);
        }
    }

    #[test]
    fn dense_rectangles_keep_adjacent_ports_and_live_tracks() {
        for (outputs, max_width, height, max_parts) in [(16, 59, 24, 1_080), (32, 68, 32, 1_687)] {
            let layout = get_layout(24, outputs, 0, 0).unwrap();
            assert_eq!(layout.height, height);
            assert!(layout.width <= max_width && layout.parts.len() <= max_parts);
            check_tracks(&layout);

            for input_gap in 0..=MAX_GAP {
                for output_gap in 0..=MAX_GAP {
                    let layout = get_layout(24, outputs, input_gap, output_gap).unwrap();
                    check_geometry(&layout, 24, outputs, (input_gap, output_gap));
                }
            }
        }
    }

    #[test]
    fn dense_24_to_24_packs_the_same_number_of_belts_into_less_space() {
        let layout = get_layout(24, 24, 0, 0).unwrap();
        assert_eq!(layout.height, 24);
        assert!(layout.width <= 61 && layout.parts.len() <= 1_299);
        check_tracks(&layout);

        for input_gap in 0..=MAX_GAP {
            for output_gap in 0..=MAX_GAP {
                let layout = get_layout(24, 24, input_gap, output_gap).unwrap();
                check_geometry(&layout, 24, 24, (input_gap, output_gap));
            }
        }
    }

    #[test]
    fn compact_variants_stay_smaller_with_the_same_ports() {
        for (outputs, width, height, parts) in
            [(16, 29, 32, 631), (24, 31, 24, 626), (32, 25, 32, 575)]
        {
            let layout = get_layout_variant(24, outputs, 0, 0, true).unwrap();
            assert_eq!((layout.width, layout.height), (width, height));
            assert!(layout.parts.len() <= parts);
            assert!(
                layout
                    .parts
                    .iter()
                    .filter(|part| part.x == 0 || (outputs != 16 && part.x == layout.width - 1))
                    .all(|part| part.kind == Kind::Splitter)
            );
            check_tracks(&layout);

            for input_gap in 0..=MAX_GAP {
                for output_gap in 0..=MAX_GAP {
                    let layout =
                        get_layout_variant(24, outputs, input_gap, output_gap, true).unwrap();
                    check_geometry(&layout, 24, outputs, (input_gap, output_gap));
                    let standard = get_layout(24, outputs, input_gap, output_gap).unwrap();
                    assert!(layout.width * layout.height <= standard.width * standard.height);
                }
            }
        }
    }

    #[test]
    fn compact_reduction_routes_spaced_outputs_from_alternating_splitter_ports() {
        for output_gap in 1..=MAX_GAP {
            let layout = get_layout_variant(24, 16, 0, output_gap, true).unwrap();
            let width = match output_gap {
                1 => 25,
                2 | 3 => 34,
                _ => 35,
            };
            assert_eq!(layout.width, width);
            if output_gap == 1 {
                assert_eq!(layout.height, 32);
                assert!(layout.output_rows.iter().copied().eq((0..32).step_by(2)));
            } else {
                let outlets: Vec<_> = layout.parts.iter().filter(|part| part.x == 25).collect();
                assert_eq!(outlets.len(), 16);
                assert!(outlets.iter().all(|part| part.direction == EAST));
                assert!(outlets.windows(2).all(|pair| pair[1].y - pair[0].y == 2));
            }
            check_geometry(&layout, 24, 16, (0, output_gap));
            check_tracks(&layout);
        }
    }

    fn check_tracks(layout: &Layout) {
        let mut tiles = HashMap::new();
        for (id, part) in layout.parts.iter().enumerate() {
            tiles.insert((part.x, part.y), id);
            if part.kind == Kind::Splitter {
                let (dx, dy) = DIRECTIONS[part.direction as usize];
                tiles.insert((part.x + dy.abs(), part.y + dx.abs()), id);
            }
        }
        let tunnel_ends: HashMap<_, _> = layout.tunnels.iter().copied().collect();
        let mut downstream = vec![Vec::new(); layout.parts.len()];
        let mut upstream = vec![Vec::new(); layout.parts.len()];
        for (id, part) in layout.parts.iter().enumerate() {
            let (dx, dy) = DIRECTIONS[part.direction as usize];
            let mut connect = |tile: Tile| {
                if let Some(&target) = tiles.get(&tile) {
                    downstream[id].push(target);
                    upstream[target].push(id);
                }
            };
            if part.kind == Kind::Input {
                connect(tunnel_ends[&(part.x, part.y)]);
            } else {
                connect((part.x + dx, part.y + dy));
                if part.kind == Kind::Splitter {
                    connect((part.x + dx + dy.abs(), part.y + dy + dx.abs()));
                }
            }
        }

        let inputs = layout.input_rows.iter().map(|&row| tiles[&(0, row)]);
        let outputs: Vec<_> = layout
            .output_rows
            .iter()
            .map(|&row| tiles[&(layout.width - 1, row)])
            .collect();
        let mut supplied = HashSet::new();
        for input in inputs {
            let seen = reachable(&downstream, vec![input]);
            assert!(outputs.iter().all(|output| seen.contains(output)));
            supplied.extend(seen);
        }
        assert_eq!(supplied.len(), layout.parts.len());
        assert_eq!(reachable(&upstream, outputs).len(), layout.parts.len());
    }

    fn reachable(edges: &[Vec<usize>], mut to_visit: Vec<usize>) -> HashSet<usize> {
        let mut reached = HashSet::new();
        while let Some(id) = to_visit.pop() {
            if reached.insert(id) {
                to_visit.extend(&edges[id]);
            }
        }
        reached
    }

    #[test]
    fn rejects_counts_and_spacing_outside_their_ranges() {
        assert!(get_layout(0, 1, 0, 0).is_none());
        assert!(get_layout(1, MAX_LANES + 1, 0, 0).is_none());
        assert!(get_layout(1, 1, MAX_GAP + 1, 0).is_none());
        assert!(get_layout(1, 1, 0, MAX_GAP + 1).is_none());
    }
}
