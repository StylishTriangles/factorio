use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};

#[derive(Deserialize)]
pub struct Model {
    pub version: String,
    pub packs: Vec<String>,
    pub items: BTreeMap<String, Item>,
    pub machines: BTreeMap<String, Machine>,
    beacon: Beacon,
    modules: Modules,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub name: String,
    pub icon: String,
    pub recipe: Option<Recipe>,
    pub base_recipe: Option<Recipe>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    time: f64,
    output: f64,
    machine: String,
    productivity: bool,
    ingredients: BTreeMap<String, f64>,
    note: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Machine {
    pub name: String,
    speed: f64,
    slots: u32,
    #[serde(default)]
    built_in_productivity: f64,
    beaconable: Option<bool>,
}

#[derive(Deserialize)]
struct Beacon {
    efficiency: f64,
    slots: u32,
    profile: Vec<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Modules {
    speed: [f64; 4],
    productivity: [f64; 4],
    productivity_speed: [f64; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub productivity_tier: u8,
    pub speed_tier: u8,
    pub beacons: u32,
    pub base_game: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            productivity_tier: 3,
            speed_tier: 3,
            beacons: 4,
            base_game: false,
        }
    }
}

pub struct Calculation {
    pub root: Node,
    pub totals: Vec<MaterialTotal>,
}

pub struct Node {
    pub id: String,
    pub path: String,
    pub rate: f64,
    pub production: Option<Production>,
    pub children: Vec<Node>,
    pub note: Option<String>,
}

pub struct Production {
    pub machine: String,
    pub speed: f64,
    pub machines: f64,
    pub productivity: f64,
}

pub struct MaterialTotal {
    pub id: String,
    pub rate: f64,
}

impl Model {
    pub fn load() -> serde_json::Result<Self> {
        serde_json::from_str(include_str!("../data/recipes.json"))
    }

    fn recipe(&self, id: &str, base_game: bool) -> Option<&Recipe> {
        let item = self.items.get(id)?;
        if base_game {
            item.base_recipe.as_ref().or(item.recipe.as_ref())
        } else {
            item.recipe.as_ref()
        }
    }

    pub fn can_expand(&self, id: &str) -> bool {
        self.items.get(id).is_some_and(|item| {
            [&item.recipe, &item.base_recipe]
                .into_iter()
                .flatten()
                .any(|recipe| !recipe.ingredients.is_empty())
        })
    }

    pub fn max_beacons(&self) -> u32 {
        self.beacon.profile.len() as u32
    }

    pub fn beacon_effectivity(&self, beacons: u32) -> f64 {
        if beacons == 0 {
            return 0.0;
        }

        let profile = &self.beacon.profile;
        let multiplier = profile
            .get(beacons as usize - 1)
            .or(profile.last())
            .copied()
            .unwrap_or(1.0);
        f64::from(beacons) * self.beacon.efficiency * multiplier
    }

    pub fn beacon_speed_bonus(&self, settings: Settings) -> f64 {
        self.beacon_effectivity(settings.beacons)
            * f64::from(self.beacon.slots)
            * self.modules.speed[usize::from(settings.speed_tier.min(3))]
    }

    pub fn calculate(
        &self,
        pack: &str,
        rate_per_second: f64,
        settings: Settings,
        expanded: &HashSet<String>,
    ) -> Option<Calculation> {
        if !rate_per_second.is_finite() || rate_per_second < 0.0 || !self.items.contains_key(pack) {
            return None;
        }

        let root = self.build_node(pack, pack.to_owned(), rate_per_second, settings, expanded)?;
        let mut totals = BTreeMap::new();
        for child in &root.children {
            child.add_leaf_rates(&mut totals);
        }
        let totals = totals
            .into_iter()
            .map(|(id, rate)| MaterialTotal {
                id: id.to_owned(),
                rate,
            })
            .collect::<Vec<_>>();
        if totals.iter().any(|material| !material.rate.is_finite()) {
            return None;
        }
        Some(Calculation { root, totals })
    }

    fn build_node(
        &self,
        id: &str,
        path: String,
        rate: f64,
        settings: Settings,
        expanded: &HashSet<String>,
    ) -> Option<Node> {
        if !rate.is_finite() {
            return None;
        }

        let mut node = Node {
            id: id.to_owned(),
            path,
            rate,
            production: None,
            children: Vec::new(),
            note: None,
        };
        if let Some(recipe) = self.recipe(id, settings.base_game) {
            let (production, crafts_per_second) = self.production(recipe, rate, settings)?;
            node.production = Some(production);
            node.note = recipe.note.clone();

            if node.path == id || expanded.contains(&node.path) {
                for (ingredient, quantity) in &recipe.ingredients {
                    let child = self.build_node(
                        ingredient,
                        format!("{}/{}", node.path, ingredient),
                        quantity * crafts_per_second,
                        settings,
                        expanded,
                    )?;
                    node.children.push(child);
                }
            }
        }
        Some(node)
    }

    fn production(
        &self,
        recipe: &Recipe,
        rate: f64,
        settings: Settings,
    ) -> Option<(Production, f64)> {
        let machine = self.machines.get(&recipe.machine)?;
        let slots = f64::from(machine.slots);
        let productivity_tier = usize::from(settings.productivity_tier.min(3));
        let speed_tier = usize::from(settings.speed_tier.min(3));
        let (module_productivity, module_speed) = if recipe.productivity && productivity_tier > 0 {
            (
                slots * self.modules.productivity[productivity_tier],
                slots * self.modules.productivity_speed[productivity_tier],
            )
        } else {
            (0.0, slots * self.modules.speed[speed_tier])
        };
        let beacon_speed = if machine.beaconable.unwrap_or(true) {
            self.beacon_speed_bonus(settings)
        } else {
            0.0
        };
        let speed = machine.speed * (1.0 + module_speed + beacon_speed).max(0.2);
        let productivity = (machine.built_in_productivity + module_productivity).min(3.0);
        let output = recipe.output * (1.0 + productivity);
        let crafts_per_second = rate / output;
        let machines = crafts_per_second * (recipe.time / speed);
        if !machines.is_finite() {
            return None;
        }
        Some((
            Production {
                machine: recipe.machine.clone(),
                speed,
                machines,
                productivity,
            },
            crafts_per_second,
        ))
    }
}

impl Node {
    fn add_leaf_rates<'a>(&'a self, totals: &mut BTreeMap<&'a str, f64>) {
        if self.children.is_empty() {
            *totals.entry(&self.id).or_default() += self.rate;
        } else {
            for child in &self.children {
                child.add_leaf_rates(totals);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
    }

    fn rate_for(calculation: &Calculation, leaf: &str) -> f64 {
        calculation
            .totals
            .iter()
            .find(|material| material.id == leaf)
            .unwrap()
            .rate
    }

    #[test]
    fn exact_beacon_profile_changes_machine_count_without_changing_ingredients() {
        let model = Model::load().unwrap();
        let expanded = HashSet::new();
        let single_lane = Settings::default();
        let dual_lane = Settings {
            beacons: 8,
            ..single_lane
        };
        close(model.beacon_effectivity(0), 0.0);
        close(model.beacon_effectivity(4), 3.0);
        close(model.beacon_effectivity(8), 4.242);
        close(model.beacon_speed_bonus(dual_lane), 4.242);
        let profile_length = model.max_beacons();
        close(
            model.beacon_effectivity(profile_length + 1),
            model.beacon_effectivity(profile_length) / f64::from(profile_length)
                * f64::from(profile_length + 1),
        );

        let single = model
            .calculate("automation-science-pack", 1.0, single_lane, &expanded)
            .unwrap();
        let dual = model
            .calculate("automation-science-pack", 1.0, dual_lane, &expanded)
            .unwrap();
        close(rate_for(&single, "copper-plate"), 1.0 / 1.4);
        close(rate_for(&dual, "copper-plate"), 1.0 / 1.4);
        let production = single.root.production.unwrap();
        close(production.productivity, 0.4);
        close(production.speed, 4.25);
        close(production.machines, 5.0 / (4.25 * 1.4));
        assert!(dual.root.production.unwrap().machines < production.machines);
    }

    #[test]
    fn module_tiers_follow_recipe_eligibility_and_building_slot_counts() {
        let model = Model::load().unwrap();
        let settings = Settings {
            productivity_tier: 1,
            speed_tier: 2,
            ..Settings::default()
        };
        let calculation = model
            .calculate("logistic-science-pack", 1.0, settings, &HashSet::new())
            .unwrap();
        let science = calculation.root.production.unwrap();
        close(science.productivity, 0.16);
        close(science.speed, 3.25);
        let inserter = calculation
            .root
            .children
            .iter()
            .find(|child| child.id == "inserter")
            .unwrap()
            .production
            .as_ref()
            .unwrap();
        close(inserter.productivity, 0.0);
        close(inserter.speed, 5.0);
    }

    #[test]
    fn expansion_aggregates_shared_leaf_rates_without_rounding() {
        let model = Model::load().unwrap();
        let settings = Settings::default();
        let expanded = HashSet::from([
            "logistic-science-pack/inserter".to_owned(),
            "logistic-science-pack/transport-belt".to_owned(),
        ]);
        let calculation = model
            .calculate("logistic-science-pack", 1.0, settings, &expanded)
            .unwrap();
        close(rate_for(&calculation, "iron-gear-wheel"), 1.5 / 1.4);
        close(rate_for(&calculation, "iron-plate"), 1.5 / 1.4);
        close(rate_for(&calculation, "electronic-circuit"), 1.0 / 1.4);

        let mut expanded = expanded;
        expanded.insert("logistic-science-pack/inserter/iron-gear-wheel".to_owned());
        let deeper = model
            .calculate("logistic-science-pack", 1.0, settings, &expanded)
            .unwrap();
        close(rate_for(&deeper, "iron-gear-wheel"), 0.5 / 1.4);
        close(
            rate_for(&deeper, "iron-plate"),
            1.5 / 1.4 + 2.0 / (1.4 * 1.4),
        );
    }

    #[test]
    fn all_pack_recipes_resolve_to_known_items_without_cycles() {
        fn visit<'a>(
            model: &'a Model,
            id: &'a str,
            base_game: bool,
            active: &mut HashSet<&'a str>,
            visited: &mut HashSet<&'a str>,
        ) {
            if visited.contains(id) {
                return;
            }
            assert!(model.items.contains_key(id), "missing item {id}");
            assert!(active.insert(id), "cyclic recipe for {id}");
            if let Some(recipe) = model.recipe(id, base_game) {
                assert!(model.machines.contains_key(&recipe.machine));
                assert!(recipe.output > 0.0 && recipe.time > 0.0);
                for (ingredient, quantity) in &recipe.ingredients {
                    assert!(*quantity > 0.0);
                    visit(model, ingredient, base_game, active, visited);
                }
            }
            active.remove(id);
            visited.insert(id);
        }

        let model = Model::load().unwrap();
        let mut visited = HashSet::new();
        let mut active = HashSet::new();
        for pack in &model.packs {
            assert!(model.can_expand(pack), "missing recipe for {pack}");
            visit(&model, pack, false, &mut active, &mut visited);
        }
        visited.clear();
        visit(
            &model,
            "space-science-pack",
            true,
            &mut active,
            &mut visited,
        );
        assert!(visited.contains("satellite"));
    }
}
