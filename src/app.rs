use crate::model::{Calculation, Model, Node, Settings};
use std::collections::HashSet;
use std::rc::Rc;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

const BASE_SPACE: &str = "base-space-science-pack";
const SPACE: &str = "space-science-pack";

#[derive(Clone, Copy, PartialEq)]
enum RateUnit {
    Minute,
    Second,
}

impl RateUnit {
    fn factor(self) -> f64 {
        match self {
            Self::Minute => 60.0,
            Self::Second => 1.0,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Minute => "per minute",
            Self::Second => "per second",
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Self::Minute => "/ min",
            Self::Second => "/ sec",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum BeaconLayout {
    Single,
    Dual,
    Custom,
}

impl BeaconLayout {
    fn label(self) -> &'static str {
        match self {
            Self::Single => "Single lane",
            Self::Dual => "Dual lane",
            Self::Custom => "",
        }
    }
}

#[function_component]
pub fn App() -> Html {
    let model = use_state(|| Model::load().map(Rc::new));
    match &*model {
        Ok(model) => html! { <Planner model={model.clone()}/> },
        Err(_) => {
            html! { <p class="startup" role="alert">{"The recipe data could not be loaded."}</p> }
        }
    }
}

#[derive(Properties)]
struct PlannerProps {
    model: Rc<Model>,
}

impl PartialEq for PlannerProps {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}

#[function_component]
fn Planner(props: &PlannerProps) -> Html {
    let model = &props.model;
    let pack = use_state(|| "logistic-science-pack".to_owned());
    let rate = use_state_eq(|| 1.0_f64);
    let rate_draft = use_state(|| None::<(RateUnit, String)>);
    let settings = use_state(Settings::default);
    let layout = use_state_eq(|| BeaconLayout::Single);
    let custom_beacons = use_state_eq(|| "4".to_owned());
    let expanded = use_state(HashSet::<String>::new);

    let change_pack = {
        let pack = pack.clone();
        let expanded = expanded.clone();
        let settings = settings.clone();
        Callback::from(move |event: Event| {
            let selected = event.target_unchecked_into::<HtmlSelectElement>().value();
            settings.set(Settings {
                base_game: selected == BASE_SPACE,
                ..*settings
            });
            pack.set(selected);
            expanded.set(HashSet::new());
        })
    };

    let rate_field = |unit: RateUnit| {
        let output = *rate * unit.factor();
        let (editing, value) = match &*rate_draft {
            Some((active, text)) if *active == unit => (true, text.clone()),
            _ => (false, format!("{output:.2}")),
        };
        let oninput = {
            let rate = rate.clone();
            let rate_draft = rate_draft.clone();
            Callback::from(move |event: InputEvent| {
                let input = event.target_unchecked_into::<HtmlInputElement>();
                let value = input.value();
                if let Some(next) = parse_rate(&value, unit) {
                    rate.set(next);
                }
                rate_draft.set(Some((unit, value)));
            })
        };
        let onblur = {
            let rate_draft = rate_draft.clone();
            Callback::from(move |_| rate_draft.set(None))
        };
        html! {
            <label class="rate-control">
                <span class="sr-only">{format!("Packs {}", unit.label())}</span>
                <input type="text" inputmode="decimal" {value} {oninput} {onblur}
                    aria-invalid={(editing && parse_rate(rate_draft.as_ref().map_or("", |(_, value)| value), unit).is_none()).to_string()}/>
                <span class="input-unit" aria-hidden="true">{unit.symbol()}</span>
            </label>
        }
    };

    let module_field = |productivity: bool| {
        let (label, tier, icon_id) = if productivity {
            (
                "Productivity",
                settings.productivity_tier,
                "productivity-module",
            )
        } else {
            ("Speed", settings.speed_tier, "speed-module")
        };
        let icon_id = match tier {
            0 => None,
            1 => Some(icon_id.to_string()),
            _ => Some(format!("{icon_id}-{tier}")),
        };
        let onchange = {
            let settings = settings.clone();
            Callback::from(move |event: Event| {
                let tier = event
                    .target_unchecked_into::<HtmlSelectElement>()
                    .value()
                    .parse()
                    .unwrap_or(0);
                let next = if productivity {
                    Settings {
                        productivity_tier: tier,
                        ..*settings
                    }
                } else {
                    Settings {
                        speed_tier: tier,
                        ..*settings
                    }
                };
                settings.set(next);
            })
        };
        html! {
            <label class="select-control">
                <span class="sr-only">{format!("{label} module tier")}</span>
                if let Some(id) = icon_id {
                    {item_icon(model, &id, "control-icon")}
                } else {
                    <span class="control-icon no-module" aria-hidden="true"/>
                }
                <select value={tier.to_string()} {onchange}>
                    {for (0..=3).map(|number| html! {
                        <option value={number.to_string()} selected={number == tier}>
                            {if number == 0 { format!("No {} module", label.to_lowercase()) } else { format!("{label} {number}") }}
                        </option>
                    })}
                </select>
                {chevron()}
            </label>
        }
    };

    let set_layout = |mode: BeaconLayout| {
        let settings = settings.clone();
        let layout = layout.clone();
        let custom_beacons = custom_beacons.clone();
        Callback::from(move |_| {
            let beacons = match mode {
                BeaconLayout::Single => 4,
                BeaconLayout::Dual => 8,
                BeaconLayout::Custom => {
                    custom_beacons.set(settings.beacons.to_string());
                    settings.beacons
                }
            };
            settings.set(Settings {
                beacons,
                ..*settings
            });
            layout.set(mode);
        })
    };

    let max_beacons = model.max_beacons();
    let change_custom = {
        let settings = settings.clone();
        let custom_beacons = custom_beacons.clone();
        Callback::from(move |event: InputEvent| {
            let text = event.target_unchecked_into::<HtmlInputElement>().value();
            if let Ok(value) = text.parse::<u32>()
                && value <= max_beacons
            {
                settings.set(Settings {
                    beacons: value,
                    ..*settings
                });
            }
            custom_beacons.set(text);
        })
    };
    let finish_custom = {
        let settings = settings.clone();
        let custom_beacons = custom_beacons.clone();
        Callback::from(move |_| custom_beacons.set(settings.beacons.to_string()))
    };

    let toggle = {
        let expanded = expanded.clone();
        Callback::from(move |path: String| {
            let mut next = (*expanded).clone();
            if !next.remove(&path) {
                next.insert(path);
            }
            expanded.set(next);
        })
    };
    let reset = {
        let expanded = expanded.clone();
        Callback::from(move |_| expanded.set(HashSet::new()))
    };

    let pack_id = if *pack == BASE_SPACE {
        SPACE
    } else {
        pack.as_str()
    };
    let calculation = model.calculate(pack_id, *rate, *settings, &expanded);

    html! {
        <>
            <header class="site-header">
                <div class="shell header-inner">
                    <div class="brand">
                        <img src="./favicon.svg" width="36" height="36" alt=""/>
                        <strong>{"Factorio"}</strong>
                        <span>{"Science Planner"}</span>
                    </div>
                    <nav aria-label="Resources">
                        <a href="https://wiki.factorio.com/">{"Factorio Wiki"}</a>
                        <a href="https://github.com/StylishTriangles/factorio">{"GitHub"}</a>
                    </nav>
                </div>
            </header>
            <main class="shell">
                <div class="intro">
                    <h1>{"Science Planner"}</h1>
                    <p>{"Plan science output and trace every ingredient."}</p>
                </div>

                <section class="settings-panel" aria-label="Production settings">
                    <div class="settings-grid">
                        <div class="setting">
                            <h2>{"Science pack"}</h2>
                            <label class="select-control">
                                <span class="sr-only">{"Science pack"}</span>
                                {item_icon(model, pack_id, "control-icon")}
                                <select value={(*pack).clone()} onchange={change_pack}>
                                    <optgroup label="Nauvis">
                                        {for model.packs.iter().take(6).map(|id| html! {
                                            <option value={id.clone()} selected={*id == *pack}>{&model.items[id].name}</option>
                                        })}
                                    </optgroup>
                                    <optgroup label="Space Age">
                                        {for model.packs.iter().skip(6).map(|id| html! {
                                            <option value={id.clone()} selected={*id == *pack}>{&model.items[id].name}</option>
                                        })}
                                    </optgroup>
                                    <optgroup label="Base game">
                                        <option value={BASE_SPACE} selected={*pack == BASE_SPACE}>{"Space science pack (base game)"}</option>
                                    </optgroup>
                                </select>
                                {chevron()}
                            </label>
                        </div>
                        <div class="setting">
                            <h2>{"Target output"}</h2>
                            <div class="rate-pair">{rate_field(RateUnit::Minute)}{rate_field(RateUnit::Second)}</div>
                        </div>
                        <div class="setting">
                            <h2>{"Modules"}</h2>
                            <div class="module-fields">{module_field(true)}{module_field(false)}</div>
                        </div>
                        <div class="setting">
                            <h2>{"Beacons"}</h2>
                            <div class="segments" role="group" aria-label="Beacon layout">
                                {for [(BeaconLayout::Single, "4×"), (BeaconLayout::Dual, "8×"), (BeaconLayout::Custom, "Custom")].into_iter().map(|(mode, label)| html! {
                                    <button type="button" aria-pressed={(*layout == mode).to_string()} onclick={set_layout(mode)}>
                                        {label}
                                    </button>
                                })}
                            </div>
                            if *layout == BeaconLayout::Custom {
                                <label class="custom-beacons">
                                    <span>{"Beacons per machine"}</span>
                                    <input type="number" inputmode="numeric" min="0" max={max_beacons.to_string()} step="1"
                                        value={(*custom_beacons).clone()} oninput={change_custom} onblur={finish_custom}
                                        aria-invalid={custom_beacons.parse::<u32>().map_or(true, |count| count > max_beacons).to_string()}/>
                                </label>
                            } else {
                                <p class="layout-caption">{layout.label()}</p>
                            }
                        </div>
                    </div>
                    {settings_details(model, calculation.as_ref(), *settings)}
                </section>

                if let Some(result) = &calculation {
                    <section class="graph-section" aria-labelledby="graph-heading">
                        <div class="section-heading">
                            <div>
                                <h2 id="graph-heading">{"Dependency graph"}</h2>
                                <p>
                                    {"Select an ingredient to expand its recipe."}
                                    <span class="graph-mobile-hint">{" Swipe sideways to see more branches."}</span>
                                </p>
                            </div>
                            <button class="reset-button" type="button" onclick={reset} disabled={expanded.is_empty()}>{"Reset graph"}</button>
                        </div>
                        <div class="graph-viewport" tabindex="0" role="region" aria-label="Scrollable recipe graph">
                            <ul class="graph" aria-label="Recipes">
                                {graph_node(model, &result.root, &toggle, true)}
                            </ul>
                        </div>
                    </section>
                    {materials(model, result)}
                } else {
                    <p class="calculation-error" role="status">{"Try a smaller output target to calculate material rates."}</p>
                }

                <footer class="site-footer">
                    <span>{format!("Factorio {} · Normal quality", model.version)}</span>
                    <a href="https://github.com/StylishTriangles/factorio#calculation-model">{"Calculation notes"}</a>
                </footer>
            </main>
        </>
    }
}

fn parse_rate(input: &str, unit: RateUnit) -> Option<f64> {
    let input = input.trim();
    if input.is_empty() {
        return Some(0.0);
    }
    let value = input.replace(',', ".").parse::<f64>().ok()? / unit.factor();
    (value.is_finite() && value >= 0.0 && (value * 60.0).is_finite()).then_some(value.abs())
}

fn chevron() -> Html {
    html! {
        <svg class="chevron" aria-hidden="true" viewBox="0 0 16 16" width="16" height="16">
            <path d="m4.5 6 3.5 3.5L11.5 6" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
    }
}

fn item_icon(model: &Model, id: &str, class: &'static str) -> Html {
    model.items.get(id).map_or_else(Html::default, |item| {
        html! {
            <span class={classes!("item-icon", class)}>
                <img src={item.icon.clone()} alt="" referrerpolicy="no-referrer" />
            </span>
        }
    })
}

fn settings_details(model: &Model, result: Option<&Calculation>, settings: Settings) -> Html {
    let transmission = model.beacon_effectivity(settings.beacons);
    let beacon_speed = model.beacon_speed_bonus(settings) * 100.0;
    let production = result.and_then(|calculation| calculation.root.production.as_ref());
    html! {
        <div class="settings-details">
            if let Some(production) = production {
                <span class="machine-label">
                    {item_icon(model, &production.machine, "machine-icon")}
                    <span>{&model.machines[&production.machine].name}</span>
                    <strong class="machine-count" title="Fractional machine count for the selected science recipe">
                        {format!("× {:.2}", production.machines)}
                    </strong>
                </span>
                <span><strong>{format!("+{:.2}%", production.productivity * 100.0)}</strong>{" productivity"}</span>
            }
            <span title={format!("Speed bonus for a compatible machine with two modules per beacon: +{beacon_speed:.2}%")}>
                <strong>{format!("{transmission:.2}×")}</strong>{" beacon effect"}
            </span>
        </div>
    }
}

fn graph_node(model: &Model, node: &Node, toggle: &Callback<String>, root: bool) -> Html {
    let item = &model.items[&node.id];
    let open = !node.children.is_empty();
    let expandable = !root && model.can_expand(&node.id);
    let tooltip = node.production.as_ref().map(|production| {
        let machine = &model.machines[&production.machine].name;
        let note = node
            .note
            .as_deref()
            .map_or(String::new(), |note| format!("\n{note}"));
        format!(
            "{machine} × {:.2}\nCrafting speed {:.2} · Productivity +{:.2}%{note}",
            production.machines,
            production.speed,
            production.productivity * 100.0
        )
    });
    let contents = html! {
        <>
            {item_icon(model, &node.id, "node-icon")}
            <span class="node-copy">
                <span class="node-name">{&item.name}</span>
                <span class="node-rate">{format!("{:.2}", node.rate)}<small>{" / sec"}</small></span>
            </span>
            if expandable {
                <span class="node-disclosure" aria-hidden="true">{chevron()}</span>
            }
        </>
    };
    let graph_card = if expandable {
        let path = node.path.clone();
        let toggle = toggle.clone();
        let onclick = Callback::from(move |_| toggle.emit(path.clone()));
        html! {
            <button type="button" class="node-card" aria-expanded={open.to_string()}
                aria-label={format!("{} recipe for {}", if open { "Collapse" } else { "Expand" }, item.name)}
                title={tooltip} {onclick}>
                {contents}
            </button>
        }
    } else {
        html! { <div class={classes!("node-card", root.then_some("node-root"))} title={tooltip}>{contents}</div> }
    };

    html! {
        <li class="node-branch" key={node.path.clone()}>
            {graph_card}
            if open {
                <ul class="node-children">
                    {for node.children.iter().map(|child| graph_node(model, child, toggle, false))}
                </ul>
            }
        </li>
    }
}

fn materials(model: &Model, result: &Calculation) -> Html {
    html! {
        <section class="materials-section" aria-labelledby="materials-heading">
            <div class="section-heading">
                <h2 id="materials-heading">{"Material requirements"}</h2>
                <span>{"Per second"}</span>
            </div>
            <div class="table-frame">
                <table>
                    <thead><tr><th scope="col">{"Material"}</th><th scope="col">{"Required / sec"}</th></tr></thead>
                    <tbody>
                        {for result.totals.iter().map(|total| html! {
                            <tr key={total.id.clone()}>
                                <th scope="row">
                                    <span class="material-name">
                                        {item_icon(model, &total.id, "material-icon")}
                                        {&model.items[&total.id].name}
                                    </span>
                                </th>
                                <td>{format!("{:.2}", total.rate)}</td>
                            </tr>
                        })}
                    </tbody>
                </table>
            </div>
        </section>
    }
}
