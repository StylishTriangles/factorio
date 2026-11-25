use crate::balancer_layout::{Kind, Layout, Part};
use std::collections::HashMap;
use std::f64::consts::{FRAC_PI_2, PI};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    CanvasGradient, CanvasRenderingContext2d as Context, FileList, HtmlCanvasElement, HtmlElement,
    HtmlImageElement, Url,
};

type Point = (f64, f64);

const STRAIGHT_ROWS: [u8; 4] = [2, 0, 3, 1];
const CORNER_ROWS: [[u8; 4]; 4] = [[2, 5, 2, 7], [4, 0, 9, 0], [3, 8, 3, 10], [6, 1, 11, 1]];
const TUNNEL_ROWS: [[u8; 4]; 2] = [[17, 19, 13, 15], [16, 18, 12, 14]];
const DIRECTIONS: [Point; 4] = [(0., -1.), (1., 0.), (0., 1.), (-1., 0.)];

const SPLITTER_FRAMES: [(Point, Point); 4] = [
    ((160., 70.), (7., 0.)),
    ((90., 84.), (4., 13.)),
    ((164., 64.), (4., 0.)),
    ((94., 86.), (5., 12.)),
];
const SPLITTER_PATCHES: [(Point, Point); 2] = [((90., 104.), (4., -20.)), ((94., 96.), (5., -18.))];

struct Sprites {
    belt: HtmlImageElement,
    underground: HtmlImageElement,
    splitters: [HtmlImageElement; 4],
    patches: [HtmlImageElement; 2],
}

#[derive(Default)]
pub struct Art {
    sprites: Option<Sprites>,
}

impl Art {
    pub async fn load(files: FileList) -> Result<Self, String> {
        let sheets = [
            ("express-transport-belt.png", 4096, 2560),
            ("express-underground-belt-structure.png", 768, 768),
            ("express-splitter-north.png", 1280, 280),
            ("express-splitter-east.png", 720, 336),
            ("express-splitter-south.png", 1312, 256),
            ("express-splitter-west.png", 752, 344),
            ("express-splitter-east-top_patch.png", 720, 416),
            ("express-splitter-west-top_patch.png", 752, 384),
        ];
        let by_name: HashMap<_, _> = (0..files.length())
            .filter_map(|index| files.get(index))
            .filter_map(|file| {
                let name = file.name();
                name.starts_with("express-").then_some((name, file))
            })
            .collect();
        if sheets.iter().any(|(name, ..)| !by_name.contains_key(*name)) {
            return Err("Choose Factorio’s data/base/graphics/entity folder.".into());
        }

        let mut images = Vec::with_capacity(sheets.len());
        for (name, width, height) in sheets {
            let error = || format!("Could not read {name}.");
            let image = HtmlImageElement::new().map_err(|_| error())?;
            let url = Url::create_object_url_with_blob(&by_name[name]).map_err(|_| error())?;
            image.set_src(&url);
            let result = JsFuture::from(image.decode()).await;
            let _ = Url::revoke_object_url(&url);
            result.map_err(|_| error())?;
            if image.natural_width() != width || image.natural_height() != height {
                return Err("These sprite sheets do not match Factorio 2.x.".into());
            }
            images.push(image);
        }
        let [
            belt,
            underground,
            north,
            east,
            south,
            west,
            east_patch,
            west_patch,
        ] = images
            .try_into()
            .map_err(|_| "The Factorio sprite sheets could not be loaded.")?;
        Ok(Self {
            sprites: Some(Sprites {
                belt,
                underground,
                splitters: [north, east, south, west],
                patches: [east_patch, west_patch],
            }),
        })
    }
}

fn sprite(
    ctx: &Context,
    image: &HtmlImageElement,
    (column, row): (u8, u8),
    (w, h): Point,
    (x, y): Point,
) {
    // A Factorio high-resolution sprite has 64 source pixels per tile.
    let _ = ctx.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
        image,
        f64::from(column) * w,
        f64::from(row) * h,
        w,
        h,
        x - w / 128.,
        y - h / 128.,
        w / 64.,
        h / 64.,
    );
}

fn game_belt(ctx: &Context, sprites: &Sprites, center: Point, row: u8) {
    sprite(ctx, &sprites.belt, (0, row), (128., 128.), center);
}

fn game_splitter(ctx: &Context, sprites: &Sprites, (x, y): Point, direction: usize) {
    let (dx, dy) = if direction % 2 == 1 {
        (0., 0.5)
    } else {
        (0.5, 0.)
    };
    let row = STRAIGHT_ROWS[direction];
    game_belt(ctx, sprites, (x - dx, y - dy), row);
    game_belt(ctx, sprites, (x + dx, y + dy), row);
    let (size, (ox, oy)) = SPLITTER_FRAMES[direction];
    sprite(
        ctx,
        &sprites.splitters[direction],
        (0, 0),
        size,
        (x + ox / 32., y + oy / 32.),
    );
    if direction % 2 == 1 {
        let patch = direction / 2;
        let (size, (ox, oy)) = SPLITTER_PATCHES[patch];
        sprite(
            ctx,
            &sprites.patches[patch],
            (0, 0),
            size,
            (x + ox / 32., y + oy / 32.),
        );
    }
}

fn metal(ctx: &Context, height: f64, edges: &str, face: &str) -> CanvasGradient {
    let gradient = ctx.create_linear_gradient(0., -height, 0., height);
    for (stop, color) in [(0., edges), (0.3, face), (0.7, face), (1., edges)] {
        let _ = gradient.add_color_stop(stop, color);
    }
    gradient
}

fn stroke(ctx: &Context, width: f64, color: &str) {
    ctx.set_line_width(width);
    ctx.set_stroke_style_str(color);
    ctx.stroke();
}

fn draw_track(ctx: &Context, side: f64, from: f64, to: f64, treads: bool) {
    ctx.begin_path();
    if side == 0. {
        ctx.move_to(from - 0.5, 0.);
        ctx.line_to(to - 0.5, 0.);
    } else {
        let angle = |t| PI + side * t * FRAC_PI_2;
        let _ = ctx.arc_with_anticlockwise(0.5, side * 0.5, 0.5, angle(from), angle(to), side < 0.);
    }
    ctx.set_line_cap("butt");
    for (width, color) in [
        (0.91, "#101419"),
        (0.83, "#6e747a"),
        (0.77, "#263b4b"),
        (0.69, "#4e8ba6"),
    ] {
        stroke(ctx, width, color);
    }
    ctx.set_line_width(0.57);
    ctx.set_stroke_style_canvas_gradient(&metal(ctx, 0.3, "#4f565b", "#31363b"));
    ctx.stroke();
    stroke(ctx, 0.04, "#24282b");

    if !treads {
        return;
    }
    for index in 0..10 {
        let t = (f64::from(index) + 0.5) / 10.;
        if !(from..=to).contains(&t) {
            continue;
        }
        ctx.save();
        if side == 0. {
            let _ = ctx.translate(t - 0.5, 0.);
        } else {
            let angle = PI + side * t * FRAC_PI_2;
            let _ = ctx.translate(0.5 + angle.cos() * 0.5, side * 0.5 + angle.sin() * 0.5);
            let _ = ctx.rotate(-side * (1. - t) * FRAC_PI_2);
        }
        for y in [-0.25, 0.025] {
            ctx.set_fill_style_str("#20272a");
            ctx.fill_rect(-0.019, y, 0.042, 0.225);
            ctx.set_fill_style_str("#728087");
            ctx.fill_rect(0.014, y, 0.016, 0.225);
        }
        ctx.restore();
    }
}

fn direction_mark(ctx: &Context, x: f64, y: f64, size: f64) {
    ctx.begin_path();
    ctx.move_to(x - size, y - size);
    ctx.line_to(x + size, y);
    ctx.line_to(x - size, y + size);
    ctx.line_to(x - size * 0.42, y);
    ctx.close_path();
    ctx.set_fill_style_str("#79d4f5");
    ctx.fill();
}

fn housing(ctx: &Context, x: f64, y: f64, w: f64, h: f64, radius: f64) {
    ctx.begin_path();
    let _ = ctx.round_rect_with_f64(x, y, w, h, radius);
    ctx.fill();
    stroke(ctx, 0.025, "#10161a");
}

fn screw(ctx: &Context, x: f64, y: f64) {
    ctx.set_fill_style_str("#182026");
    ctx.fill_rect(x - 0.037, y - 0.037, 0.074, 0.074);
    ctx.set_fill_style_str("#bac1bc");
    ctx.fill_rect(x - 0.024, y - 0.024, 0.048, 0.022);
}

fn turn_direction(incoming: &[u8], direction: u8) -> Option<u8> {
    match incoming {
        [feed] if feed % 2 != direction % 2 => Some(*feed),
        _ => None,
    }
}

fn draw_belt(
    ctx: &Context,
    center: Point,
    direction: u8,
    incoming: &[u8],
    art: &Art,
    treads: bool,
) {
    let turn = turn_direction(incoming, direction);
    if let Some(sprites) = &art.sprites {
        let row = turn.map_or(STRAIGHT_ROWS[direction as usize], |feed| {
            CORNER_ROWS[feed as usize][direction as usize]
        });
        game_belt(ctx, sprites, center, row);
        return;
    }
    ctx.save();
    let _ = ctx.translate(center.0, center.1);
    let _ = ctx.rotate((f64::from(direction) - 1.) * FRAC_PI_2);
    let side = turn.map_or(
        0.,
        |feed| {
            if feed == (direction + 1) % 4 { -1. } else { 1. }
        },
    );
    draw_track(ctx, side, 0., 1., treads);
    direction_mark(ctx, 0.25, 0., 0.094);
    ctx.restore();
}

fn draw_splitter(ctx: &Context, (x, y): Point, direction: u8, art: &Art, treads: bool) {
    let center = if direction % 2 == 1 {
        (x + 0.5, y + 1.)
    } else {
        (x + 1., y + 0.5)
    };
    if let Some(sprites) = &art.sprites {
        game_splitter(ctx, sprites, center, direction as usize);
        return;
    }
    ctx.save();
    let _ = ctx.translate(center.0, center.1);
    let _ = ctx.rotate((f64::from(direction) - 1.) * FRAC_PI_2);
    for lane in [-0.5, 0.5] {
        ctx.save();
        let _ = ctx.translate(0., lane);
        draw_track(ctx, 0., 0., 1., treads);
        ctx.restore();
    }
    ctx.set_fill_style_canvas_gradient(&metal(ctx, 0.94, "#222f33", "#596870"));
    housing(ctx, -0.26, -0.94, 0.52, 1.88, 0.085);
    ctx.set_fill_style_str("#293740");
    housing(ctx, -0.19, -0.85, 0.38, 1.7, 0.058);
    for pos in [-0.83, 0.83] {
        screw(ctx, -0.16, pos);
        screw(ctx, 0.16, pos);
    }
    for pos in [-0.54, 0.54] {
        ctx.set_fill_style_str("#2d627d");
        housing(ctx, -0.11, pos - 0.27, 0.44, 0.54, 0.06);
        ctx.set_fill_style_canvas_gradient(&metal(ctx, 0.74, "#3c6680", "#5b95ad"));
        housing(ctx, -0.075, pos - 0.2, 0.375, 0.4, 0.05);
        direction_mark(ctx, 0.19, pos, 0.09);
    }
    ctx.set_fill_style_str("#18282f");
    housing(ctx, -0.28, -0.16, 0.58, 0.32, 0.07);
    screw(ctx, -0.18, 0.);
    screw(ctx, 0.18, 0.);
    ctx.set_fill_style_str("#c29150");
    ctx.fill_rect(-0.055, -0.035, 0.11, 0.07);
    ctx.restore();
}

fn draw_tunnel_end(
    ctx: &Context,
    center: Point,
    direction: u8,
    input: bool,
    art: &Art,
    treads: bool,
) {
    if let Some(sprites) = &art.sprites {
        let row = TUNNEL_ROWS[usize::from(!input)][direction as usize];
        game_belt(ctx, sprites, center, row);
        let column = if input {
            direction
        } else {
            (direction + 2) % 4
        };
        sprite(
            ctx,
            &sprites.underground,
            (column, u8::from(input)),
            (192., 192.),
            center,
        );
        return;
    }
    ctx.save();
    let _ = ctx.translate(center.0, center.1);
    let _ = ctx.rotate((f64::from(direction) - 1.) * FRAC_PI_2);
    let (from, to) = if input { (0., 0.5) } else { (0.5, 1.) };
    draw_track(ctx, 0., from, to, treads);
    ctx.set_fill_style_str("#202d35");
    housing(ctx, -0.24, -0.44, 0.48, 0.88, 0.075);
    ctx.set_fill_style_canvas_gradient(&metal(ctx, 0.37, "#28495b", "#507b91"));
    housing(ctx, -0.19, -0.37, 0.38, 0.74, 0.048);
    direction_mark(ctx, 0.025, 0., 0.128);
    for x in [-0.16, 0.16] {
        screw(ctx, x, -0.35);
        screw(ctx, x, 0.35);
    }
    ctx.restore();
}

fn draw_tunnels(ctx: &Context, tunnels: impl IntoIterator<Item = (Point, Point)>) {
    ctx.save();
    let dash = js_sys::Array::of2(&JsValue::from_f64(0.11), &JsValue::from_f64(0.1));
    let _ = ctx.set_line_dash(dash.as_ref());
    ctx.begin_path();
    for ((x1, y1), (x2, y2)) in tunnels {
        ctx.move_to(x1, y1);
        ctx.line_to(x2, y2);
    }
    ctx.set_line_cap("round");
    stroke(ctx, 0.081, "#142529");
    stroke(ctx, 0.036, "#6cdce9");
    ctx.restore();
}

fn context(canvas: &HtmlCanvasElement) -> Option<Context> {
    canvas.get_context("2d").ok()??.dyn_into().ok()
}

pub fn draw_legend(
    belt: &HtmlCanvasElement,
    splitter: &HtmlCanvasElement,
    underground: &HtmlCanvasElement,
    art: &Art,
) {
    let ready = |canvas: &HtmlCanvasElement, sx, sy, x, y| {
        let ctx = context(canvas)?;
        let _ = ctx.reset_transform();
        ctx.clear_rect(0., 0., canvas.width().into(), canvas.height().into());
        let _ = ctx.set_transform(sx, 0., 0., sy, x, y);
        Some(ctx)
    };
    if let Some(ctx) = ready(belt, 48., 43., 10., 5.) {
        draw_belt(&ctx, (0.5, 0.5), 1, &[], art, true);
    }
    if let Some(ctx) = ready(splitter, 46., 26., 8., 6.) {
        draw_splitter(&ctx, (0., 0.), 1, art, true);
    }
    if let Some(ctx) = ready(underground, 29., 43., 8., 5.) {
        draw_tunnels(&ctx, [((0.5, 0.5), (3.1, 0.5))]);
        draw_tunnel_end(&ctx, (0.5, 0.5), 1, true, art, true);
        draw_tunnel_end(&ctx, (3.1, 0.5), 1, false, art, true);
    }
}

fn draw_grid(ctx: &Context, w: f64, h: f64, x: f64, y: f64, tile: f64) {
    let step = tile * (12. / tile).ceil();
    ctx.begin_path();
    let mut col = x.rem_euclid(step);
    while col <= w {
        ctx.move_to(col, 0.);
        ctx.line_to(col, h);
        col += step;
    }
    let mut row = y.rem_euclid(step);
    while row <= h {
        ctx.move_to(0., row);
        ctx.line_to(w, row);
        row += step;
    }
    stroke(ctx, 1., "#73695539");
}

fn draw_ports(ctx: &Context, rows: &[i32], x: f64, title: &str, tile: f64) {
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    let pitch = rows
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .min()
        .unwrap_or(1);
    let font = (tile * 0.32).clamp(10., 20.);
    let every = ((font + 2.) / (tile * f64::from(pitch))).ceil().max(1.) as usize;
    ctx.set_font(&format!("700 {}px ui-monospace, monospace", font / tile));
    for (index, &row) in rows.iter().enumerate() {
        let y = f64::from(row) + 0.5;
        if index % every == 0 {
            ctx.set_fill_style_str("#ddd4c5");
            let _ = ctx.fill_text(&(index + 1).to_string(), x, y);
        } else {
            ctx.set_fill_style_str("#736955");
            ctx.fill_rect(x - 2. / tile, y - 0.5 / tile, 4. / tile, 1. / tile);
        }
    }
    ctx.set_fill_style_str("#c0b4a0");
    let title_size = if tile < 25. { 10. } else { 13. };
    ctx.set_font(&format!(
        "700 {}px ui-monospace, monospace",
        title_size / tile
    ));
    if let Some(first) = rows.first() {
        let _ = ctx.fill_text(title, x, f64::from(*first) - 13. / tile);
    }
}

fn draw_overview(ctx: &Context, layout: &Layout, visible: &impl Fn(&Part) -> bool) {
    ctx.save();
    ctx.begin_path();
    ctx.set_line_cap("butt");
    ctx.set_line_join("round");
    for part in layout
        .parts
        .iter()
        .filter(|part| part.kind == Kind::Belt && visible(part))
    {
        let (x, y) = center(part.x, part.y);
        let (dx, dy) = DIRECTIONS[part.direction as usize];
        let incoming = layout
            .feeds
            .get(&(part.x, part.y))
            .map_or(&[][..], Vec::as_slice);
        let turn = turn_direction(incoming, part.direction);
        let (ix, iy) = turn.map_or((dx, dy), |feed| DIRECTIONS[feed as usize]);
        ctx.move_to(x - ix * 0.51, y - iy * 0.51);
        if turn.is_some() {
            ctx.quadratic_curve_to(x, y, x + dx * 0.51, y + dy * 0.51);
        } else {
            ctx.line_to(x + dx * 0.51, y + dy * 0.51);
        }
    }
    stroke(ctx, 0.74, "#718998");
    for part in layout
        .parts
        .iter()
        .filter(|part| part.kind != Kind::Belt && visible(part))
    {
        let splitter = part.kind == Kind::Splitter;
        let horizontal = part.direction % 2 == 1;
        ctx.set_fill_style_str(if splitter { "#76bad5" } else { "#4889a6" });
        ctx.fill_rect(
            f64::from(part.x) + 0.12,
            f64::from(part.y) + 0.12,
            if splitter && !horizontal { 1.76 } else { 0.76 },
            if splitter && horizontal { 1.76 } else { 0.76 },
        );
    }
    ctx.restore();
}

fn center(x: i32, y: i32) -> Point {
    (f64::from(x) + 0.5, f64::from(y) + 0.5)
}

pub fn draw_layout(
    layout: &Layout,
    zoom: u32,
    board: &HtmlElement,
    area: &HtmlElement,
    canvas: &HtmlCanvasElement,
    art: &Art,
) {
    let (w, h) = (
        f64::from(board.client_width()),
        f64::from(board.client_height()),
    );
    if w < 1. || h < 1. {
        return;
    }

    let (width, height) = (f64::from(layout.width), f64::from(layout.height));
    let gutter = if w < 500. { 40. } else { 66. };
    let tile = if zoom == 0 {
        ((w - gutter * 2.) / width)
            .min((h - 70.) / height)
            .clamp(0.25, 84.)
    } else {
        f64::from(zoom)
    };
    let (world_width, world_height) = if zoom == 0 {
        (w, h)
    } else {
        (
            w.max((width * tile + gutter * 2.).ceil()),
            h.max((height * tile + 70.).ceil()),
        )
    };
    let _ = area
        .style()
        .set_property("width", &format!("{world_width}px"));
    let _ = area
        .style()
        .set_property("height", &format!("{world_height}px"));
    let _ = canvas.style().set_property("width", &format!("{w}px"));
    let _ = canvas.style().set_property("height", &format!("{h}px"));

    let dpr = web_sys::window().map_or(1., |window| window.device_pixel_ratio());
    let (pixels_x, pixels_y) = ((w * dpr).round() as u32, (h * dpr).round() as u32);
    if canvas.width() != pixels_x {
        canvas.set_width(pixels_x);
    }
    if canvas.height() != pixels_y {
        canvas.set_height(pixels_y);
    }
    let Some(ctx) = context(canvas) else { return };
    let x = (world_width - width * tile) / 2. - f64::from(board.scroll_left());
    let y = (world_height - height * tile) / 2. - f64::from(board.scroll_top()) + 6.;
    let _ = ctx.set_transform(dpr, 0., 0., dpr, 0., 0.);
    ctx.clear_rect(0., 0., w, h);
    draw_grid(&ctx, w, h, x, y, tile);
    let _ = ctx.translate(x, y);
    let _ = ctx.scale(tile, tile);

    let left = -x / tile - 2.;
    let top = -y / tile - 2.;
    let right = (w - x) / tile + 2.;
    let bottom = (h - y) / tile + 2.;
    let visible = |part: &Part| {
        (left..=right).contains(&f64::from(part.x)) && (top..=bottom).contains(&f64::from(part.y))
    };
    if tile < 5. {
        draw_overview(&ctx, layout, &visible);
    } else {
        let treads = tile * dpr >= 24.;
        let visible_parts = layout.parts.iter().filter(|part| visible(part));
        for part in visible_parts.clone().filter(|part| part.kind == Kind::Belt) {
            draw_belt(
                &ctx,
                center(part.x, part.y),
                part.direction,
                layout
                    .feeds
                    .get(&(part.x, part.y))
                    .map_or(&[], Vec::as_slice),
                art,
                treads,
            );
        }
        for part in visible_parts
            .clone()
            .filter(|part| part.kind == Kind::Splitter)
        {
            draw_splitter(
                &ctx,
                (f64::from(part.x), f64::from(part.y)),
                part.direction,
                art,
                treads,
            );
        }
        draw_tunnels(
            &ctx,
            layout.tunnels.iter().filter_map(|&((x1, y1), (x2, y2))| {
                (f64::from(x1.min(x2)) <= right
                    && f64::from(x1.max(x2)) >= left
                    && f64::from(y1.min(y2)) <= bottom
                    && f64::from(y1.max(y2)) >= top)
                    .then_some((center(x1, y1), center(x2, y2)))
            }),
        );
        for part in visible_parts {
            if matches!(part.kind, Kind::Input | Kind::Output) {
                draw_tunnel_end(
                    &ctx,
                    center(part.x, part.y),
                    part.direction,
                    part.kind == Kind::Input,
                    art,
                    treads,
                );
            }
        }
    }

    let gap = if w < 500. { 16. } else { 24. } / tile;
    draw_ports(&ctx, &layout.input_rows, -gap, "IN", tile);
    draw_ports(&ctx, &layout.output_rows, width + gap, "OUT", tile);
}
