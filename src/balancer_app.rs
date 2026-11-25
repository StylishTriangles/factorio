use crate::app::{SiteHeader, chevron};
use crate::balancer_blueprint::blueprint_string;
use crate::balancer_canvas::{Art, draw_layout, draw_legend};
use crate::balancer_layout::{MAX_GAP as MAX_SPACING, MAX_LANES, get_layout_variant};
use crate::graph_viewport::GraphViewport;
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    HtmlCanvasElement, HtmlElement, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement,
    ResizeObserver,
};
use yew::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Settings {
    inputs: usize,
    outputs: usize,
    input_spacing: usize,
    output_spacing: usize,
    compact: bool,
}

const DEFAULT: Settings = Settings {
    inputs: 4,
    outputs: 4,
    input_spacing: 0,
    output_spacing: 0,
    compact: true,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum CopyFeedback {
    Ready,
    Copied,
    Manual,
}

#[derive(Default)]
struct Graphics {
    art: Art,
    revision: u64,
    game: bool,
}

#[function_component]
pub(crate) fn Balancers() -> Html {
    let counts = use_state_eq(|| DEFAULT);
    let generated = use_memo(*counts, |settings| {
        get_layout_variant(
            settings.inputs,
            settings.outputs,
            settings.input_spacing,
            settings.output_spacing,
            settings.compact,
        )
        .and_then(|layout| {
            blueprint_string(&layout)
                .ok()
                .map(|blueprint| (layout, blueprint))
        })
    });
    let zoom = use_state_eq(|| 0_u32);
    let feedback = use_state_eq(|| CopyFeedback::Ready);
    let copy_revision = use_mut_ref(|| 0_u64);
    let graphics = use_state(Graphics::default);
    let graphics_loading = use_state_eq(|| false);
    let graphics_status = use_state_eq(String::new);

    let inputs = use_node_ref();
    let outputs = use_node_ref();
    let input_spacing = use_node_ref();
    let output_spacing = use_node_ref();
    let compact = use_node_ref();
    let manual = use_node_ref();
    let files = use_node_ref();
    let choose = use_node_ref();
    let board = use_node_ref();
    let area = use_node_ref();
    let canvas = use_node_ref();
    let belt = use_node_ref();
    let splitter = use_node_ref();
    let underground = use_node_ref();

    let regenerate = {
        let counts = counts.clone();
        let feedback = feedback.clone();
        let copy_revision = copy_revision.clone();
        let inputs = inputs.clone();
        let outputs = outputs.clone();
        let input_spacing = input_spacing.clone();
        let output_spacing = output_spacing.clone();
        let compact = compact.clone();
        Callback::from(move |()| {
            let read = |field: &NodeRef, min: usize, max: usize| {
                let value = field.cast::<HtmlInputElement>()?.value_as_number();
                (value.fract() == 0.0 && (min as f64..=max as f64).contains(&value))
                    .then_some(value as usize)
            };
            let next = Settings {
                inputs: read(&inputs, 1, MAX_LANES).unwrap_or(counts.inputs),
                outputs: read(&outputs, 1, MAX_LANES).unwrap_or(counts.outputs),
                input_spacing: read(&input_spacing, 0, MAX_SPACING).unwrap_or(counts.input_spacing),
                output_spacing: read(&output_spacing, 0, MAX_SPACING)
                    .unwrap_or(counts.output_spacing),
                compact: compact
                    .cast::<HtmlInputElement>()
                    .map_or(counts.compact, |input| input.checked()),
            };
            if next != *counts {
                *copy_revision.borrow_mut() += 1;
                feedback.set(CopyFeedback::Ready);
                counts.set(next);
            }
        })
    };
    let onblur = regenerate.reform(|_: FocusEvent| ());
    let on_spacing = regenerate.reform(|_: InputEvent| ());
    let on_layout = regenerate.reform(|_: Event| ());
    let onsubmit = Callback::from(move |event: SubmitEvent| {
        event.prevent_default();
        regenerate.emit(());
    });
    let copy = {
        let generated = generated.clone();
        let feedback = feedback.clone();
        let copy_revision = copy_revision.clone();
        Callback::from(move |_| {
            let Some((_, code)) = generated.as_ref() else {
                return;
            };
            let clipboard = web_sys::window()
                .map(|window| window.navigator().clipboard())
                .filter(|clipboard| !clipboard.is_undefined() && !clipboard.is_null());
            let Some(clipboard) = clipboard else {
                feedback.set(CopyFeedback::Manual);
                return;
            };
            let promise = clipboard.write_text(code);
            let revision = {
                let mut current = copy_revision.borrow_mut();
                *current += 1;
                *current
            };
            let feedback = feedback.clone();
            let copy_revision = copy_revision.clone();
            spawn_local(async move {
                let copied = JsFuture::from(promise).await.is_ok();
                if *copy_revision.borrow() == revision {
                    feedback.set(if copied {
                        CopyFeedback::Copied
                    } else {
                        CopyFeedback::Manual
                    });
                }
            });
        })
    };

    let choose_graphics = {
        let files = files.clone();
        Callback::from(move |_| {
            if let Some(input) = files.cast::<HtmlInputElement>() {
                input.click();
            }
        })
    };
    let load_graphics = {
        let graphics = graphics.clone();
        let loading = graphics_loading.clone();
        let status = graphics_status.clone();
        Callback::from(move |event: Event| {
            let input = event.target_unchecked_into::<HtmlInputElement>();
            let Some(files) = input.files().filter(|files| files.length() > 0) else {
                return;
            };
            loading.set(true);
            status.set("Loading game graphics…".into());
            let graphics = graphics.clone();
            let loading = loading.clone();
            let status = status.clone();
            spawn_local(async move {
                match Art::load(files).await {
                    Ok(art) => {
                        graphics.set(Graphics {
                            art,
                            revision: graphics.revision + 1,
                            game: true,
                        });
                        status.set("Using game graphics from this device.".into());
                    }
                    Err(error) => status.set(error),
                }
                input.set_value("");
                loading.set(false);
            });
        })
    };
    let reset_graphics = {
        let graphics = graphics.clone();
        let status = graphics_status.clone();
        let choose = choose.clone();
        Callback::from(move |_| {
            graphics.set(Graphics {
                revision: graphics.revision + 1,
                ..Graphics::default()
            });
            status.set("Default graphics restored.".into());
            if let Some(button) = choose.cast::<HtmlElement>() {
                let _ = button.focus();
            }
        })
    };

    let repaint = {
        let (board, area, canvas) = (board.clone(), area.clone(), canvas.clone());
        let (generated, graphics, zoom) = (generated.clone(), graphics.clone(), *zoom);
        Callback::from(move |()| {
            if let (Some((layout, _)), Some(board), Some(area), Some(canvas)) = (
                generated.as_ref(),
                board.cast::<HtmlElement>(),
                area.cast::<HtmlElement>(),
                canvas.cast::<HtmlCanvasElement>(),
            ) {
                draw_layout(layout, zoom, &board, &area, &canvas, &graphics.art);
            }
        })
    };
    {
        let board = board.clone();
        use_effect_with((*counts, *zoom), move |_| {
            if let Some(board) = board.cast::<HtmlElement>() {
                board.set_scroll_left(0);
                board.set_scroll_top(0);
            }
        });
    }
    {
        let repaint = repaint.clone();
        let (board, belt, splitter, underground) = (
            board.clone(),
            belt.clone(),
            splitter.clone(),
            underground.clone(),
        );
        let graphics = graphics.clone();
        use_effect_with((*counts, *zoom, graphics.revision), move |_| {
            if let (Some(belt), Some(splitter), Some(underground)) = (
                belt.cast::<HtmlCanvasElement>(),
                splitter.cast::<HtmlCanvasElement>(),
                underground.cast::<HtmlCanvasElement>(),
            ) {
                draw_legend(&belt, &splitter, &underground, &graphics.art);
            }
            repaint.emit(());
            let listener = Closure::<dyn FnMut()>::new(move || repaint.emit(()));
            let observer = ResizeObserver::new(listener.as_ref().unchecked_ref()).ok();
            if let (Some(observer), Some(board)) = (&observer, board.cast::<HtmlElement>()) {
                observer.observe(&board);
            }
            move || {
                if let Some(observer) = observer {
                    observer.disconnect();
                }
                drop(listener);
            }
        });
    }
    {
        let manual = manual.clone();
        use_effect_with(*feedback, move |feedback| {
            if *feedback == CopyFeedback::Manual
                && let Some(text) = manual.cast::<HtmlTextAreaElement>()
            {
                let _ = text.focus();
                text.select();
            }
        });
    }

    let change_zoom = {
        let zoom = zoom.clone();
        Callback::from(move |event: Event| {
            let selection = event.target_unchecked_into::<HtmlSelectElement>();
            zoom.set(selection.value().parse().unwrap_or(0));
        })
    };
    let zooms = [
        (0, "Fit to view"),
        (24, "24 px / tile"),
        (48, "48 px / tile"),
        (72, "72 px / tile"),
    ];
    let description = format!(
        "{} input and {} output express belt layout",
        counts.inputs, counts.outputs
    );
    let copied = *feedback == CopyFeedback::Copied;
    let copying_manually = *feedback == CopyFeedback::Manual;
    let onscroll = Callback::from(move |_: Event| repaint.emit(()));
    let footer_link = "https://github.com/R-O-C-K-E-T/Factorio-SAT/blob/main/books/blue_narrow.txt";

    html! {
        <>
            <SiteHeader balancers={true}/>
            <main class="shell">
                <div class="intro">
                    <h1>{"Belt Balancers"}</h1>
                    <p>{format!("Build and preview express belt balancers with 1–{MAX_LANES} belts per side.")}</p>
                </div>
                <form class="settings-panel" aria-label="Balancer settings" {onsubmit}>
                    <div class="settings-grid">
                        <div class="setting">
                            <h2>{"Inputs"}</h2>
                            <div class="rate-pair">
                                <NumberField node={inputs} label="Inputs" initial={DEFAULT.inputs} onblur={onblur.clone()}/>
                                {spacing_slider(input_spacing, "Input spacing", counts.input_spacing, on_spacing.clone())}
                            </div>
                        </div>
                        <div class="setting">
                            <h2>{"Outputs"}</h2>
                            <div class="rate-pair">
                                <NumberField node={outputs} label="Outputs" initial={DEFAULT.outputs} {onblur}/>
                                {spacing_slider(output_spacing, "Output spacing", counts.output_spacing, on_spacing)}
                            </div>
                        </div>
                        <div class="setting">
                            <h2>{"Blueprint"}</h2>
                            <div class="belt-actions">
                                <button class="reset-button belt-generate" type="submit">{"Generate"}</button>
                                <button class="reset-button" type="button" onclick={copy} disabled={generated.is_none()}>
                                    {if copied { "Copied!" } else { "Copy blueprint" }}
                                </button>
                            </div>
                        </div>
                        <div class="setting">
                            <h2>{"Zoom"}</h2>
                            <label class="select-control belt-zoom">
                                <span class="sr-only">{"Zoom"}</span>
                                <select value={zoom.to_string()} onchange={change_zoom}>
                                    {for zooms.into_iter().map(|(scale, name)| html! {
                                        <option value={scale.to_string()} selected={*zoom == scale}>{name}</option>
                                    })}
                                </select>
                                {chevron()}
                            </label>
                        </div>
                    </div>
                    <div class="settings-details">
                        <span>{"Spacing in empty tiles between belts. "}<strong>{"0"}</strong>{" puts belts side by side."}</span>
                        if (counts.inputs, counts.outputs) == (24, 32) {
                            <span>
                                <label class="belt-compact">
                                    <input ref={compact} type="checkbox" checked={counts.compact}
                                        aria-describedby={counts.compact.then_some("compact-throughput")} onchange={on_layout}/>
                                    {"Compact layout"}
                                </label>
                            </span>
                            if counts.compact {
                                <span id="compact-throughput">{"Switch off for better flow when outputs back up."}</span>
                            }
                        }
                    </div>
                </form>

                if let Some((layout, blueprint)) = generated.as_ref() {
                    <section class="graph-section" aria-labelledby="belt-heading">
                        <div class="section-heading">
                            <div>
                                <h2 id="belt-heading">{"Belt layout"}</h2>
                                <p id="belt-instructions">{"Drag to pan. Use Zoom to inspect individual tiles."}</p>
                            </div>
                            <span id="footprint" class="belt-footprint" aria-live="polite">
                                {format!("{} → {} · {} × {} tiles", counts.inputs, counts.outputs, layout.width, layout.height)}
                            </span>
                        </div>
                        <GraphViewport viewport={board} class="belt-viewport" label="Scrollable belt layout"
                            instructions="belt-instructions" center_on_mount={false} {onscroll}>
                            <div class="belt-diagram" ref={area}>
                                <canvas id="belt-diagram" ref={canvas} role="img" aria-label={description}/>
                            </div>
                        </GraphViewport>
                        <ul class="belt-legend" aria-label="Diagram legend">
                            <li><canvas ref={belt} width="68" height="52" aria-hidden="true"/>{"Belt"}</li>
                            <li><canvas ref={splitter} width="64" height="64" aria-hidden="true"/>{"Splitter"}</li>
                            <li><canvas ref={underground} width="120" height="52" aria-hidden="true"/>{"Underground"}</li>
                        </ul>
                        <p class="sr-only" role="status">{if copied { "Blueprint copied to the clipboard." } else { "" }}</p>
                        if copying_manually {
                            <label class="belt-manual">
                                {"Copy this blueprint string manually"}
                                <textarea ref={manual} readonly={true} spellcheck="false" rows="3" value={blueprint.clone()}/>
                            </label>
                        }
                    </section>
                } else {
                    <p class="calculation-error" role="alert">{"Choose valid belt counts and spacing to generate a blueprint."}</p>
                }

                <details class="belt-graphics">
                    <summary>{"Game graphics"}</summary>
                    <p>{"Choose "}<code>{"data/base/graphics/entity"}</code>{" from your Factorio 2.x installation. The browser reads the graphics locally."}</p>
                    <input id="game-files" ref={files} type="file" webkitdirectory={true} hidden={true} onchange={load_graphics}/>
                    <div class="belt-actions">
                        <button class="reset-button" ref={choose} type="button" disabled={*graphics_loading} onclick={choose_graphics}>{"Choose folder"}</button>
                        if graphics.game {
                            <button class="reset-button" type="button" disabled={*graphics_loading} onclick={reset_graphics}>{"Reset graphics"}</button>
                        }
                    </div>
                    <p role="status">{graphics_status.as_str()}</p>
                </details>
                <footer class="site-footer">
                    <span>{"In Factorio, open "}<strong>{"Import string"}</strong>{" and paste the blueprint."}</span>
                    <a href={footer_link}>{"Compact layouts from Factorio-SAT"}</a>
                </footer>
            </main>
        </>
    }
}

#[derive(Properties, PartialEq)]
struct NumberFieldProps {
    node: NodeRef,
    label: AttrValue,
    initial: usize,
    onblur: Callback<FocusEvent>,
}

#[function_component]
fn NumberField(props: &NumberFieldProps) -> Html {
    let value = use_state_eq(|| props.initial.to_string());
    let oninput = {
        let value = value.clone();
        Callback::from(move |event: InputEvent| {
            value.set(event.target_unchecked_into::<HtmlInputElement>().value());
        })
    };
    html! {
        <label class="rate-control belt-number">
            <span class="sr-only">{props.label.clone()}</span>
            <input ref={props.node.clone()} type="number" inputmode="numeric"
                min="1" max={MAX_LANES.to_string()} step="1" value={(*value).clone()}
                {oninput} onblur={props.onblur.clone()} required={true}/>
            <span class="input-unit" aria-hidden="true">{"belts"}</span>
        </label>
    }
}

fn spacing_slider(
    node: NodeRef,
    label: &'static str,
    value: usize,
    oninput: Callback<InputEvent>,
) -> Html {
    let unit = if value == 1 { "tile" } else { "tiles" };
    html! {
        <label class="belt-spacing">
            <span aria-hidden="true">{"Spacing"}</span>
            <strong aria-hidden="true">{value}</strong>
            <input ref={node} type="range" min="0" max={MAX_SPACING.to_string()} step="1"
                value={value.to_string()} aria-label={label} aria-valuetext={format!("{value} empty {unit}")} {oninput}/>
        </label>
    }
}
