use crate::balancer_layout::{Kind, Layout};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use flate2::{Compression, write::ZlibEncoder};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Serialize)]
struct Entity {
    entity_number: usize,
    name: &'static str,
    position: Position,
    direction: u8,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    underground_end: Option<&'static str>,
}

#[derive(Serialize)]
struct Position {
    x: f64,
    y: f64,
}

#[derive(Serialize)]
struct Export {
    blueprint: Blueprint,
}

#[derive(Serialize)]
struct Blueprint {
    item: &'static str,
    label: String,
    icons: [Value; 1],
    entities: Vec<Entity>,
    version: u64,
}

pub fn blueprint_string(layout: &Layout) -> std::io::Result<String> {
    let entities: Vec<_> = layout
        .parts
        .iter()
        .enumerate()
        .map(|(index, part)| {
            let (name, underground_end) = match part.kind {
                Kind::Belt => ("express-transport-belt", None),
                Kind::Splitter => ("express-splitter", None),
                Kind::Input => ("express-underground-belt", Some("input")),
                Kind::Output => ("express-underground-belt", Some("output")),
            };
            let mut x = f64::from(part.x) + 0.5;
            let mut y = f64::from(part.y) + 0.5;
            if part.kind == Kind::Splitter {
                if part.direction % 2 == 1 {
                    y += 0.5;
                } else {
                    x += 0.5;
                }
            }
            Entity {
                entity_number: index + 1,
                name,
                position: Position { x, y },
                direction: part.direction * 4,
                underground_end,
            }
        })
        .collect();
    let export = Export {
        blueprint: Blueprint {
            item: "blueprint",
            label: format!(
                "{} → {} express belt balancer",
                layout.input_rows.len(),
                layout.output_rows.len()
            ),
            icons: [
                json!({ "index": 1, "signal": { "type": "item", "name": "express-splitter" } }),
            ],
            entities,
            version: 2_u64 << 48,
        },
    };
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    serde_json::to_writer(&mut encoder, &export)?;
    let mut encoded = String::from("0");
    STANDARD.encode_string(encoder.finish()?, &mut encoded);
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::balancer_layout::get_layout;
    use flate2::read::ZlibDecoder;

    #[test]
    fn blueprint_preserves_ports_and_express_entities() {
        let layout = get_layout(9, 6, 1, 2).unwrap();
        let code = blueprint_string(&layout).unwrap();
        let bytes = STANDARD.decode(code.strip_prefix('0').unwrap()).unwrap();
        let decoded: serde_json::Value =
            serde_json::from_reader(ZlibDecoder::new(bytes.as_slice())).unwrap();
        let entities = decoded["blueprint"]["entities"].as_array().unwrap();
        assert_eq!(entities.len(), layout.parts.len());
        assert!(entities.iter().all(|entity| {
            entity["name"]
                .as_str()
                .is_some_and(|name| name.starts_with("express-"))
        }));
        for (position, count, gap) in [(0.5, 9, 1.0), (f64::from(layout.width) - 0.5, 6, 2.0)] {
            let mut rows: Vec<_> = entities
                .iter()
                .filter(|entity| entity["position"]["x"].as_f64() == Some(position))
                .map(|entity| entity["position"]["y"].as_f64().unwrap())
                .collect();
            rows.sort_by(f64::total_cmp);
            assert_eq!(rows.len(), count);
            assert!(rows.windows(2).all(|pair| pair[1] - pair[0] == gap + 1.0));
        }
    }
}
