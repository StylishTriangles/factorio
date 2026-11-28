use super::{EAST, Kind, Layout, NORTH, Part, SOUTH, STATION_LANES, index_tracks};

// Output adapters for 2–10 wagons, in order. These were solved for the fewest
// columns with monotone belts, express tunnels, and straight station ports.
const PACKED: &str = include_str!("../../data/station_adapters.txt");

pub(super) fn rows(lanes: usize, first: i32) -> Vec<i32> {
    (0..lanes)
        .map(|lane| first + (lane + lane / STATION_LANES) as i32)
        .collect()
}

pub(super) fn packed_pattern(track_rows: &[i32]) -> Option<&'static str> {
    let wagons = track_rows.len() / STATION_LANES;
    if !track_rows.windows(2).all(|pair| pair[1] - pair[0] == 1) {
        return None;
    }
    PACKED.split("\n\n").nth(wagons.checked_sub(2)?)
}

pub(super) fn append(
    parts: &mut Vec<Part>,
    pattern: &str,
    width: i32,
    first_row: i32,
    left: bool,
    boundary: i32,
) {
    let mut adapter = Layout::default();
    for (y, row) in pattern.lines().enumerate() {
        for (x, tile) in row.bytes().enumerate() {
            let (kind, direction) = match tile {
                b'.' => continue,
                b'>' => (Kind::Belt, EAST),
                b'v' => (Kind::Belt, SOUTH),
                b'E' => (Kind::Input, EAST),
                b'e' => (Kind::Output, EAST),
                b'S' => (Kind::Input, SOUTH),
                b's' => (Kind::Output, SOUTH),
                _ => unreachable!("invalid station adapter tile"),
            };
            adapter.parts.push(Part {
                x: x as i32,
                y: y as i32 + first_row,
                kind,
                direction,
            });
        }
    }
    if left {
        // Reverse the paths and mirror their columns so input belts still flow east.
        index_tracks(&mut adapter);
        for part in &mut adapter.parts {
            match part.kind {
                Kind::Belt => {
                    part.direction = adapter
                        .feeds
                        .get(&(part.x, part.y))
                        .map_or(EAST, |feed| feed[0]);
                }
                Kind::Input => part.kind = Kind::Output,
                Kind::Output => part.kind = Kind::Input,
                Kind::Splitter => unreachable!("station adapters do not use splitters"),
            }
            if part.direction == SOUTH {
                part.direction = NORTH;
            }
            part.x = width - 1 - part.x;
        }
    }
    parts.extend(adapter.parts.into_iter().map(|mut part| {
        part.x += boundary;
        part
    }));
}
