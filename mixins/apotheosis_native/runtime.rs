// Apotheosis for Horizon: rarities and affixes on loot (the first slice of a port of Apotheosis' adventure module).
//
// Injected into horizon-world as `pub mod apotheosis`; the loot hooks there and the game's hooks in horizon-client
// call it. It follows Apotheosis' own rolling (LootController, LootRule, AttributeAffix): a chest sometimes holds an
// extra affix item, and gear any loot table drops is sometimes turned into one. The item gets a rarity, a coloured
// name built from its affixes, and its affixes as attribute modifiers the game applies itself.
//
// This slice carries the attribute affixes whose attribute the game has (`minecraft:...`); the effect, ability,
// socket and durability parts of a rarity's rules roll nothing yet. The world tier is one for the whole world.

use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{OnceLock, RwLock};

pub const ID: &str = "apotheosis_native";
pub const NAME: &str = "Apotheosis";
pub const VERSION: &str = "0.8.0";
pub const DESCRIPTION: &str = "Rarities, affixes, sockets, gems, bosses and elites, as Apotheosis rolls them.";

// ---- world tier ---------------------------------------------------------------

/// Apotheosis' world tiers, lowest first.
pub const TIERS: [&str; 5] = ["haven", "frontier", "ascent", "summit", "pinnacle"];
static TIER: AtomicU8 = AtomicU8::new(0);

/// The world's tier, an index into `TIERS`.
pub fn tier() -> usize {
    usize::from(TIER.load(Ordering::Relaxed)).min(TIERS.len() - 1)
}

pub fn set_tier(tier: usize) {
    TIER.store(tier.min(TIERS.len() - 1) as u8, Ordering::Relaxed);
}

pub fn tier_named(name: &str) -> Option<usize> {
    let name = name.to_ascii_lowercase();
    TIERS.iter().position(|t| *t == name)
}

// ---- random -------------------------------------------------------------------

/// SplitMix64: small, fast and good enough for loot.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1), as Java's `nextFloat`.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// Uniform in [0, n).
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }
}

pub fn hash(text: &str) -> u64 {
    // FNV-1a, to spread the loot roll's seed over the items it touches.
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

/// A weight from Apotheosis' `TieredWeights`: one `{weight, quality}` for every tier, or one per tier name.
/// The weight is `weight + round(luck * quality)`; a tier left out weighs nothing.
pub fn weight(weights: &Value, tier: usize, luck: f32) -> i64 {
    let entry = if weights.get("weight").is_some() {
        weights
    } else {
        match weights.get(TIERS[tier]) {
            Some(entry) => entry,
            None => return 0,
        }
    };
    let base = entry.get("weight").and_then(Value::as_i64).unwrap_or(0);
    let quality = entry.get("quality").and_then(Value::as_f64).unwrap_or(0.0) as f32;
    base + (luck * quality).round() as i64
}

/// Picks one of `items` by its weight; items weighing nothing or less are left out.
pub fn pick<T: Copy>(rng: &mut Rng, items: &[(T, i64)]) -> Option<T> {
    let total: i64 = items.iter().map(|(_, w)| (*w).max(0)).sum();
    if total <= 0 {
        return None;
    }
    let mut at = rng.below(total as u64) as i64;
    for (item, w) in items {
        let w = (*w).max(0);
        if at < w {
            return Some(*item);
        }
        at -= w;
    }
    None
}

// ---- data ---------------------------------------------------------------------

pub struct Rarity {
    pub id: String,
    pub color: String,
    weights: Value,
    rules: Vec<Value>,
}

pub struct Affix {
    pub id: String,
    /// Apotheosis' affix type: `apotheosis:attribute`, `apotheosis:mob_effect`, `apotheosis:damage_reduction`...
    pub typ: String,
    /// The whole affix, for the types' own fields.
    pub raw: Value,
    kind: String,
    exclusive: Vec<String>,
    weights: Value,
    pub attribute: String,
    pub operation: String,
    values: Map<String, Value>,
    categories: Vec<String>,
}

struct Entry {
    item: String,
    weights: Value,
    rarities: Vec<String>,
    tiers: Vec<String>,
}

struct Data {
    rarities: Vec<Rarity>,
    overrides: HashMap<String, Map<String, Value>>,
    affixes: Vec<Affix>,
    entries: Vec<Entry>,
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default()
}

fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        use crate::apotheosis_data as d;
        let parse = |text: &str| serde_json::from_str::<Value>(text).expect("Apotheosis: embedded data reads");
        let rarities = match parse(d::RARITIES) {
            Value::Object(map) => map
                .into_iter()
                .map(|(id, r)| Rarity {
                    id,
                    color: r["color"].as_str().unwrap_or("#FFFFFF").to_owned(),
                    weights: r["weights"].clone(),
                    rules: r["rules"].as_array().cloned().unwrap_or_default(),
                })
                .collect(),
            _ => Vec::new(),
        };
        let overrides = match parse(d::RARITY_OVERRIDES) {
            Value::Object(map) => map
                .into_iter()
                .filter_map(|(cat, o)| Some((cat, o.as_object()?.clone())))
                .collect(),
            _ => HashMap::new(),
        };
        let affixes = parse(d::AFFIXES)
            .as_array()
            .map(|list| {
                list.iter()
                    .map(|a| Affix {
                        id: a["id"].as_str().unwrap_or_default().to_owned(),
                        typ: a["type"].as_str().unwrap_or_default().to_owned(),
                        raw: a.clone(),
                        kind: a["definition"]["affix_type"].as_str().unwrap_or_default().to_owned(),
                        exclusive: strings(a["definition"].get("exclusive_set")),
                        weights: a["definition"]["weights"].clone(),
                        attribute: a["attribute"].as_str().unwrap_or_default().to_owned(),
                        operation: a["operation"].as_str().unwrap_or("add_value").to_owned(),
                        values: a["values"].as_object().cloned().unwrap_or_default(),
                        categories: strings(a.get("categories").or(a.get("types"))),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let entries = parse(d::LOOT_ENTRIES)
            .as_array()
            .map(|list| {
                list.iter()
                    .map(|e| Entry {
                        item: e["item"].as_str().unwrap_or_default().to_owned(),
                        weights: e["weights"].clone(),
                        rarities: strings(e.get("rarities")),
                        tiers: strings(e.get("constraints").and_then(|c| c.get("tiers"))),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Data { rarities, overrides, affixes, entries }
    })
}

/// An affix by its id.
pub fn affix_named(id: &str) -> Option<&'static Affix> {
    data().affixes.iter().find(|a| a.id == id)
}

/// Whether this port carries the affix: an attribute the game has (Apothic Attributes' and NeoForge's own are
/// not in Horizon), or a type whose effect is ported.
fn supported(affix: &Affix) -> bool {
    match affix.typ.as_str() {
        "apotheosis:attribute" => {
            affix.attribute.starts_with("minecraft:") || crate::apotheosis_effects::apothic(&affix.attribute)
        }
        typ => crate::apotheosis_effects::ported(typ, &affix.raw),
    }
}

// ---- items --------------------------------------------------------------------

/// What an item's default components say about it: its attribute modifiers and the slot it is worn in.
#[derive(Clone, Default)]
pub struct Defaults {
    pub modifiers: Option<Value>,
    pub slot: Option<String>,
}

fn defaults() -> &'static RwLock<HashMap<String, Defaults>> {
    static DEFAULTS: OnceLock<RwLock<HashMap<String, Defaults>>> = OnceLock::new();
    DEFAULTS.get_or_init(Default::default)
}

/// Remembers an item's default components as they load (`native_modify_default_components`).
pub fn record_defaults(item: &str, components: &Map<String, Value>) {
    let found = Defaults {
        modifiers: components.get("minecraft:attribute_modifiers").cloned(),
        slot: components
            .get("minecraft:equippable")
            .and_then(|e| e.get("slot"))
            .and_then(Value::as_str)
            .map(str::to_owned),
    };
    if found.modifiers.is_some() || found.slot.is_some() {
        if let Ok(mut map) = defaults().write() {
            map.insert(item.to_owned(), found);
        }
    }
}

pub fn defaults_of(item: &str) -> Defaults {
    defaults().read().ok().and_then(|map| map.get(item).cloned()).unwrap_or_default()
}

/// The modifiers in an `attribute_modifiers` component, whichever form it has.
pub fn modifier_list(component: &Value) -> Vec<Value> {
    match component {
        Value::Array(list) => list.clone(),
        Value::Object(map) => map.get("modifiers").and_then(Value::as_array).cloned().unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Apotheosis' loot category of an item (`LootCategories`), by priority: bow, breaker, the armour pieces, shield
/// and trident first, then melee weapons, then shears.
pub fn category(item: &str) -> Option<&'static str> {
    let path = item.split_once(':').map_or(item, |(_, p)| p);
    let info = defaults_of(item);
    if path == "bow" || path == "crossbow" || path.ends_with("_bow") || path.ends_with("_crossbow") {
        return Some("bow");
    }
    if path.ends_with("_pickaxe") || path.ends_with("_shovel") {
        return Some("breaker");
    }
    let wearable = !(path == "carved_pumpkin" || path.ends_with("_skull") || path.ends_with("_head"));
    // The worn slot from the item's `equippable` component; by its name before the components have loaded.
    let by_name = [("_helmet", "head"), ("_chestplate", "chest"), ("_leggings", "legs"), ("_boots", "feet")]
        .into_iter()
        .find(|(end, _)| path.ends_with(end))
        .map(|(_, slot)| slot);
    if wearable {
        match info.slot.as_deref().or(by_name) {
            Some("head") => return Some("helmet"),
            Some("chest") => return Some("chestplate"),
            Some("legs") => return Some("leggings"),
            Some("feet") => return Some("boots"),
            _ => {}
        }
    }
    if path == "shield" || path.ends_with("_shield") {
        return Some("shield");
    }
    if path == "trident" {
        return Some("trident");
    }
    let hits = info.modifiers.as_ref().is_some_and(|m| {
        modifier_list(m).iter().any(|m| {
            m["type"].as_str() == Some("minecraft:attack_damage") && m["amount"].as_f64().unwrap_or(0.0) > 0.0
        })
    });
    if path.ends_with("_sword") || hits {
        return Some("melee_weapon");
    }
    if path == "shears" || path.ends_with("_shears") {
        return Some("shears");
    }
    None
}

/// The equipment slot group a category's modifiers apply in.
pub fn slot_group(category: &str) -> &'static str {
    match category {
        "bow" | "shield" => "hand",
        "helmet" => "head",
        "chestplate" => "chest",
        "leggings" => "legs",
        "boots" => "feet",
        _ => "mainhand",
    }
}

/// An affix's value at a level from its per-rarity `values` entry (Placebo's `StepFunction`): a number, or
/// `{min, max, step}` running from `min` at level 0 to `max` at level 1 in steps of `step`.
pub fn step_value(value: &Value, level: f32) -> Option<f64> {
    if let Some(n) = value.as_f64() {
        return Some(n);
    }
    let min = value.get("min")?.as_f64()?;
    let max = value.get("max").and_then(Value::as_f64).unwrap_or(min);
    let raw = min + (max - min) * f64::from(level);
    let stepped = match value.get("step").and_then(Value::as_f64) {
        Some(step) if step > 0.0 => min + ((raw - min) / step + 1e-9).floor() * step,
        _ => raw,
    };
    Some((stepped * 1e6).round() / 1e6)
}

// ---- rolling ------------------------------------------------------------------

/// A rolled item: its rarity and its affixes with their levels, in naming order.
pub struct Rolled {
    pub category: &'static str,
    pub rarity: usize,
    pub affixes: Vec<(usize, f32)>,
    /// Empty sockets the rarity's `socket` rule gave it.
    pub sockets: u32,
}

fn compatible(d: &Data, candidate: usize, current: &[(usize, f32)]) -> bool {
    let c = &d.affixes[candidate];
    current.iter().all(|&(i, _)| {
        let other = &d.affixes[i];
        i != candidate && !c.exclusive.contains(&other.id) && !other.exclusive.contains(&c.id)
    })
}

struct Roll<'a> {
    category: &'static str,
    rarity: &'a str,
    tier: usize,
    luck: f32,
    affixes: Vec<(usize, f32)>,
    sockets: u32,
}

impl Roll<'_> {
    fn rule(&mut self, d: &Data, rule: &Value, rng: &mut Rng) {
        match rule["type"].as_str().unwrap_or_default() {
            "apotheosis:affix" => {
                let kind = rule["affix_type"].as_str().unwrap_or_default();
                let wanted = format!("apotheosis:{}", self.category);
                let pool: Vec<(usize, i64)> = d
                    .affixes
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| a.kind == kind && supported(a))
                    .filter(|(_, a)| a.categories.is_empty() || a.categories.contains(&wanted))
                    .filter(|(_, a)| a.values.contains_key(self.rarity))
                    .filter(|(i, _)| compatible(d, *i, &self.affixes))
                    .map(|(i, a)| (i, weight(&a.weights, self.tier, self.luck)))
                    .filter(|(_, w)| *w > 0)
                    .collect();
                if let Some(i) = pick(rng, &pool) {
                    self.affixes.push((i, rng.next_f32()));
                }
            }
            "apotheosis:chanced" => {
                if rng.next_f32() <= rule["chance"].as_f64().unwrap_or(0.0) as f32 {
                    self.rule(d, &rule["rule"], rng);
                }
            }
            "apotheosis:select" => {
                let branch = if rng.next_f32() <= rule["chance"].as_f64().unwrap_or(0.0) as f32 { "if_true" } else { "if_false" };
                self.rule(d, &rule[branch], rng);
            }
            "apotheosis:combined" => {
                for r in rule["rules"].as_array().into_iter().flatten() {
                    self.rule(d, r, rng);
                }
            }
            // `SocketHelper.setSockets` keeps the larger count, at most 16.
            "apotheosis:socket" => {
                let min = rule["min"].as_u64().unwrap_or(0);
                let max = rule["max"].as_u64().unwrap_or(1).max(min);
                let n = (min + rng.below(max - min + 1)).min(16) as u32;
                self.sockets = self.sockets.max(n);
            }
            // Durability and component rules: later slices.
            _ => {}
        }
    }
}

/// Rolls affixes for an item of `category` at `rarity` (an index into the rarities), as `LootController
/// .createLootItem` does: the rarity's rules (or its override for the category) in order, then the affixes
/// shuffled for the name.
pub fn roll(category: &'static str, rarity: usize, tier: usize, luck: f32, rng: &mut Rng) -> Rolled {
    let d = data();
    let r = &d.rarities[rarity];
    let rules = d
        .overrides
        .get(&format!("apotheosis:{category}"))
        .and_then(|o| o.get(&r.id))
        .and_then(Value::as_array)
        .unwrap_or(&r.rules);
    let mut state = Roll { category, rarity: &r.id, tier, luck, affixes: Vec::new(), sockets: 0 };
    for rule in rules {
        state.rule(d, rule, rng);
    }
    let sockets = state.sockets;
    let mut affixes = state.affixes;
    for i in (1..affixes.len()).rev() {
        let j = rng.below(i as u64 + 1) as usize;
        affixes.swap(i, j);
    }
    Rolled { category, rarity, affixes, sockets }
}

/// A random rarity from `pool` (rarity ids; empty means all), weighted by tier and luck; when every weight is
/// nothing, any of them alike.
pub fn random_rarity(pool: &[String], tier: usize, luck: f32, rng: &mut Rng) -> usize {
    let d = data();
    let candidates: Vec<usize> = (0..d.rarities.len())
        .filter(|&i| pool.is_empty() || pool.contains(&d.rarities[i].id))
        .collect();
    let weighted: Vec<(usize, i64)> = candidates.iter().map(|&i| (i, weight(&d.rarities[i].weights, tier, luck))).collect();
    pick(rng, &weighted).unwrap_or_else(|| candidates.get(rng.below(candidates.len() as u64) as usize).copied().unwrap_or(0))
}

// ---- names --------------------------------------------------------------------

/// This port's own names for the affixes it rolls, as (affix path, English name, English suffix). Translations
/// come from the lang files (`affix.<id>` and `affix.<id>.suffix`); these are the fallbacks.
const NAMES: &[(&str, &str, &str)] = &[
    ("armor/attribute/adamantine", "Impenetrable", "of the Wall"),
    ("armor/attribute/blessed", "Vital", "of Life"),
    ("armor/attribute/elastic", "Springy", "of Strides"),
    ("armor/attribute/fireproof", "Flameproof", "of Embers"),
    ("armor/attribute/fortunate", "Starlit", "of the Stars"),
    ("armor/attribute/gravitational", "Featherweight", "of the Feather"),
    ("armor/attribute/ironforged", "Resilient", "of Iron"),
    ("armor/attribute/oxygenated", "Submersible", "of the Depths"),
    ("armor/attribute/stalwart", "Unshakable", "of the Rock"),
    ("armor/attribute/steel_touched", "Tenacious", "of Steel"),
    ("armor/attribute/windswept", "Swift", "of the Wind"),
    ("breaker/attribute/destructive", "Voracious", "of Ruin"),
    ("breaker/attribute/lengthy", "Far-Reaching", "of Reach"),
    ("breaker/attribute/submerged", "Tidal", "of the Tides"),
    ("generic/attribute/lucky", "Lucky", "of Luck"),
    ("melee/attribute/forceful", "Mighty", "of Impact"),
    ("melee/attribute/graceful", "Agile", "of Finesse"),
    ("melee/attribute/lengthy", "Long", "of the Lance"),
    ("melee/attribute/murderous", "Cruel", "of Slaughter"),
    ("melee/attribute/violent", "Fierce", "of Fury"),
    ("ranged/attribute/windswept", "Swift", "of the Wind"),
    ("shield/attribute/ironforged", "Resilient", "of Iron"),
    ("shield/attribute/stalwart", "Unshakable", "of the Rock"),
    ("shield/attribute/steel_touched", "Tenacious", "of Steel"),
    ("armor/dmg_reduction/blast_forged", "Blast-Forged", "of the Blast"),
    ("armor/dmg_reduction/blockading", "Blockading", "of the Bastion"),
    ("armor/dmg_reduction/deflective", "Deflective", "of Deflection"),
    ("armor/dmg_reduction/feathery", "Feathery", "of the Fall"),
    ("armor/dmg_reduction/grounded", "Grounded", "of the Storm"),
    ("armor/dmg_reduction/runed", "Runed", "of Runes"),
    ("armor/mob_effect/blinding", "Blinding", "of Darkness"),
    ("armor/mob_effect/bolstering", "Bolstering", "of Endurance"),
    ("armor/mob_effect/nimble", "Nimble", "of Flight"),
    ("armor/mob_effect/revitalizing", "Revitalizing", "of Renewal"),
    ("breaker/mob_effect/spelunkers", "Spelunking", "of the Caves"),
    ("breaker/mob_effect/swift", "Hasty", "of Haste"),
    ("breaker/enchantment/prosperous", "Prosperous", "of Plenty"),
    ("breaker/effect/radial", "Sweeping", "of the Quarry"),
    ("breaker/ability/supermassive", "Supermassive", "of the Mountain"),
    ("armor/attribute/spiritual", "Mending", "of Recovery"),
    ("breaker/attribute/experienced", "Learned", "of Insight"),
    ("melee/attribute/giant_slaying", "Toppling", "of the Colossus"),
    ("melee/attribute/glacial", "Frostbitten", "of Winter"),
    ("melee/attribute/infernal", "Scorching", "of the Pyre"),
    ("melee/attribute/intricate", "Precise", "of Precision"),
    ("melee/attribute/lacerating", "Rending", "of Wounds"),
    ("melee/attribute/piercing", "Puncturing", "of the Needle"),
    ("melee/attribute/vampiric", "Bloodthirsty", "of the Leech"),
    ("ranged/attribute/agile", "Quickdraw", "of the Quiver"),
    ("ranged/attribute/elven", "Keen", "of the Hunt"),
    ("weapon/attribute/shredding", "Shattering", "of Splinters"),
    ("generic/telepathic", "Telepathic", "of the Mind"),
    ("melee/cleaving", "Cleaving", "of the Cleaver"),
    ("melee/executing", "Executing", "of the Headsman"),
    ("melee/festive", "Festive", "of the Feast"),
    ("melee/thunderstruck", "Thunderstruck", "of Thunder"),
    ("melee/mob_effect/elusive", "Elusive", "of the Breeze"),
    ("melee/mob_effect/weakening", "Weakening", "of Frailty"),
    ("ranged/enchantment/prosperous", "Prosperous", "of Spoils"),
    ("ranged/mob_effect/blighted", "Blighted", "of Blight"),
    ("ranged/mob_effect/deathbound", "Deathbound", "of the Grave"),
    ("ranged/mob_effect/ensnaring", "Ensnaring", "of Snares"),
    ("ranged/mob_effect/fleeting", "Fleeting", "of the Wind's Step"),
    ("ranged/mob_effect/ivy_laced", "Ivy-Laced", "of Venom"),
    ("ranged/mob_effect/shulkers", "Levitating", "of the Shulker"),
];

/// This port's English rarity names, the fallbacks of `rarity.<id>`.
fn rarity_name(id: &str) -> &'static str {
    match id {
        "apotheosis:common" => "Common",
        "apotheosis:uncommon" => "Uncommon",
        "apotheosis:rare" => "Rare",
        "apotheosis:epic" => "Epic",
        "apotheosis:mythic" => "Mythic",
        _ => "Unknown",
    }
}

/// A rarity's id, by its index.
pub fn rarity_id(rarity: usize) -> String {
    data().rarities.get(rarity).map(|r| r.id.clone()).unwrap_or_default()
}

/// A rarity's colour, by its index.
pub fn rarity_color(rarity: usize) -> String {
    data().rarities.get(rarity).map(|r| r.color.clone()).unwrap_or_else(|| "#FFFFFF".into())
}

/// The rarity's line under an item's name.
pub fn rarity_line(id: &str) -> Option<Value> {
    let rarity = data().rarities.iter().find(|r| r.id == id)?;
    Some(json!({
        "translate": format!("rarity.{}", rarity.id), "fallback": rarity_name(&rarity.id),
        "color": rarity.color, "italic": false,
    }))
}

fn affix_name(affix: &Affix, prefix: bool) -> Value {
    let path = affix.id.split_once(':').map_or(affix.id.as_str(), |(_, p)| p);
    let (name, suffix) = NAMES
        .iter()
        .find(|(p, _, _)| *p == path)
        .map(|(_, n, s)| (*n, *s))
        .unwrap_or(("Enchanted", "of Power"));
    if prefix {
        json!({"translate": format!("affix.{}", affix.id), "fallback": name})
    } else {
        json!({"translate": format!("affix.{}.suffix", affix.id), "fallback": suffix})
    }
}

/// The game's own name of an item, as a translatable component.
fn item_name(item: &str) -> Value {
    let (ns, path) = item.split_once(':').unwrap_or(("minecraft", item));
    json!({"translate": format!("item.{ns}.{path}")})
}

// ---- the stack ----------------------------------------------------------------

/// What a roll writes on the stack: its custom name, its lore (the rarity), its attribute modifiers (the item's
/// own ones and the affixes'), and the roll itself in `custom_data`.
pub struct Patch {
    pub name: Option<Value>,
    pub lore: Value,
    pub modifiers: Value,
    pub custom: Value,
}

/// The modifiers an item's affixes give, from its `custom_data` (affix levels and rarity), in its category's slot.
/// Worked out again wherever needed: a stack's `attribute_modifiers` doesn't reach the game's window.
pub fn affix_modifiers(item: &str, custom: &Value) -> Vec<Value> {
    let Some(category) = category(item) else { return Vec::new() };
    let a = &custom["apotheosis"];
    let Some(rarity) = a["rarity"].as_str() else { return Vec::new() };
    let slot = slot_group(category);
    let d = data();
    let mut out = Vec::new();
    for (id, level) in a["affixes"].as_object().into_iter().flatten() {
        let Some(affix) = d.affixes.iter().find(|x| &x.id == id && x.typ == "apotheosis:attribute") else { continue };
        let level = level.as_f64().unwrap_or(0.0) as f32;
        let Some(amount) = affix.values.get(rarity).and_then(|v| step_value(v, level)) else { continue };
        let path = affix.id.split_once(':').map_or(affix.id.as_str(), |(_, p)| p);
        out.push(json!({
            "type": affix.attribute,
            "id": format!("apotheosis:affix/{path}"),
            "amount": amount,
            "operation": affix.operation,
            "slot": slot,
        }));
    }
    out
}

/// Every modifier this mod gives an item, affixes' and gems', from its `custom_data`.
pub fn own_modifiers(item: &str, custom: &Value) -> Vec<Value> {
    let mut out = affix_modifiers(item, custom);
    out.extend(crate::apotheosis_gems::gem_modifiers(item, custom));
    out
}

/// The item kind's own modifiers and then this mod's, as the stack's `attribute_modifiers`.
pub fn full_modifiers(item: &str, custom: &Value) -> Value {
    let mut list = defaults_of(item).modifiers.as_ref().map(modifier_list).unwrap_or_default();
    list.retain(|m| !m["id"].as_str().is_some_and(|id| id.starts_with("apotheosis:")));
    list.extend(own_modifiers(item, custom));
    Value::Array(list)
}

pub fn patch(item: &str, rolled: &Rolled, current_modifiers: Option<&Value>) -> Patch {
    let d = data();
    let rarity = &d.rarities[rolled.rarity];
    let slot = slot_group(rolled.category);
    let mut modifiers = current_modifiers
        .or(defaults_of(item).modifiers.as_ref())
        .map(modifier_list)
        .unwrap_or_default();
    let mut levels = Map::new();
    for &(i, level) in &rolled.affixes {
        let affix = &d.affixes[i];
        levels.insert(affix.id.clone(), json!(level));
        if affix.typ != "apotheosis:attribute" {
            continue;
        }
        let Some(amount) = affix.values.get(&rarity.id).and_then(|v| step_value(v, level)) else { continue };
        let path = affix.id.split_once(':').map_or(affix.id.as_str(), |(_, p)| p);
        modifiers.push(json!({
            "type": affix.attribute,
            "id": format!("apotheosis:affix/{path}"),
            "amount": amount,
            "operation": affix.operation,
            "slot": slot,
        }));
    }
    let name = match rolled.affixes.as_slice() {
        [] => None,
        [(only, _)] => Some(json!({
            "translate": "misc.apotheosis.affix_name.two", "fallback": "%s %s",
            "with": [affix_name(&d.affixes[*only], true), item_name(item)],
            "color": rarity.color, "italic": false,
        })),
        [(first, _), (second, _), ..] => Some(json!({
            "translate": "misc.apotheosis.affix_name.three", "fallback": "%s %s %s",
            "with": [affix_name(&d.affixes[*first], true), item_name(item), affix_name(&d.affixes[*second], false)],
            "color": rarity.color, "italic": false,
        })),
    };
    let mut custom = json!({"apotheosis": {"rarity": rarity.id, "affixes": Value::Object(levels)}});
    if rolled.sockets > 0 {
        custom["apotheosis"]["sockets"] = json!(rolled.sockets);
    }
    let lore = crate::apotheosis_gems::lore_for(&custom);
    Patch { name, lore, modifiers: Value::Array(modifiers), custom }
}

// ---- loot hooks ---------------------------------------------------------------

/// The chance a chest table gets an affix item (Apotheosis' `affix_loot_injection`), or None.
fn injection_chance(id: &str) -> Option<f64> {
    let (ns, path) = id.split_once(':').unwrap_or(("minecraft", id));
    if path.starts_with("chests") {
        return Some(if ns == "minecraft" { 0.35 } else { 0.3 });
    }
    if ns == "twilightforest" && path.starts_with("structures") {
        return Some(0.3);
    }
    None
}

/// `native_loot_table_load`: gear from any table but a block's may turn into affix loot (`affix_conversion`, 35%
/// an item), and chests may hold one more item, an affix item (`affix_loot_injection`).
pub fn table_load(id: &str, table: &mut Value) {
    static FIRST: std::sync::Once = std::sync::Once::new();
    FIRST.call_once(|| log::info!("Apotheosis: adding affix loot to the loot tables (first: {id})"));
    let path = id.split_once(':').map_or(id, |(_, p)| p);
    let Some(table) = table.as_object_mut() else { return };
    let pools = table.entry("pools").or_insert_with(|| Value::Array(Vec::new()));
    let Some(pools) = pools.as_array_mut() else { return };
    // Minecraft 26.3's loot format: a pool's functions are its `modifier` (one or a list, each naming its `type`),
    // and its conditions one `condition`.
    if !path.contains("blocks") {
        for pool in pools.iter_mut() {
            let Some(pool) = pool.as_object_mut() else { continue };
            let conversion = json!({"type": "apotheosis:affix_conversion", "chance": 0.35});
            let modifier = match pool.remove("modifier") {
                Some(Value::Array(mut list)) => {
                    list.push(conversion);
                    Value::Array(list)
                }
                Some(one @ Value::Object(_)) => Value::Array(vec![one, conversion]),
                _ => Value::Array(vec![conversion]),
            };
            pool.insert("modifier".to_owned(), modifier);
        }
    }
    if let Some(chance) = injection_chance(id) {
        pools.push(json!({
            "rolls": 1,
            "condition": {
                "type": "minecraft:all_of",
                "terms": [
                    {"type": "apotheosis:has_player"},
                    {"type": "minecraft:random_chance", "chance": chance},
                ],
            },
            "entries": [{
                "type": "minecraft:item",
                "name": "minecraft:stick",
                "modifier": [{"type": "apotheosis:affix_loot"}],
            }],
        }));
    }
    crate::apotheosis_gems::table_load(id, pools);
}

/// `native_loot_condition` for this mod's kinds: None for a kind that isn't its own.
pub fn loot_condition(kind: &str, has_player: bool, entity: Option<&str>) -> Option<bool> {
    match kind {
        "apotheosis:has_player" => Some(has_player),
        "apotheosis:is_monster" => Some(entity.is_some_and(crate::apotheosis_gems::is_monster)),
        _ => None,
    }
}

/// Logs the first roll of each of this mod's loot kinds and what its context named, to see in the log that the
/// hooks run and what the engines tell them.
pub fn note_context(kind: &str, player: bool, entity: Option<&str>) {
    static SEEN: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    if let Ok(mut seen) = SEEN.lock() {
        if !seen.iter().any(|k| k == kind) {
            seen.push(kind.to_owned());
            log::info!("Apotheosis: first {kind} roll (player: {player}, entity: {})", entity.unwrap_or("none"));
        }
    }
}

/// The stack a loot function changes, as the loot engines hand it over.
pub struct Stack<'a> {
    pub item: &'a mut String,
    pub count: &'a mut i32,
    pub names: &'a mut Vec<(String, Value)>,
    pub components: &'a mut Vec<(String, Value)>,
}

fn has_affixes(components: &[(String, Value)]) -> bool {
    components
        .iter()
        .any(|(id, v)| id == "minecraft:custom_data" && v.get("apotheosis").is_some())
}

fn apply(stack: &mut Stack<'_>, patch: Patch) {
    if let Some(name) = patch.name {
        // The loot engines key a name by its component id, as `set_name` does.
        stack.names.retain(|(kind, _)| kind != "minecraft:custom_name");
        stack.names.push(("minecraft:custom_name".to_owned(), name));
    }
    for (id, value) in [
        ("minecraft:lore", patch.lore),
        ("minecraft:attribute_modifiers", patch.modifiers),
        ("minecraft:custom_data", patch.custom),
    ] {
        stack.components.retain(|(c, _)| c != id);
        stack.components.push((id.to_owned(), value));
    }
}

fn current_modifiers(components: &[(String, Value)]) -> Option<&Value> {
    components.iter().rev().find(|(id, _)| id == "minecraft:attribute_modifiers").map(|(_, v)| v)
}

/// `native_loot_function` for this mod's kinds. False for a kind that isn't its own.
pub fn loot_function(
    kind: &str,
    args: &Value,
    has_player: bool,
    luck: f32,
    seed: i64,
    dimension: Option<&str>,
    stack: &mut Stack<'_>,
) -> bool {
    let mut rng = Rng::new((seed as u64) ^ hash(stack.item) ^ hash(kind));
    let tier = tier();
    match kind {
        // A chest's extra item: an entry from the affix loot list, rolled.
        "apotheosis:affix_loot" => {
            let d = data();
            let pool: Vec<(usize, i64)> = d
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| e.tiers.is_empty() || e.tiers.iter().any(|t| t == TIERS[tier]))
                .map(|(i, e)| (i, weight(&e.weights, tier, luck)))
                .collect();
            let entry = if has_player { pick(&mut rng, &pool).map(|i| &d.entries[i]) } else { None };
            let rolled = entry.and_then(|e| {
                let category = category(&e.item)?;
                let rarity = random_rarity(&e.rarities, tier, luck, &mut rng);
                Some((e, roll(category, rarity, tier, luck, &mut rng)))
            });
            match rolled {
                Some((entry, rolled)) => {
                    *stack.item = entry.item.clone();
                    *stack.count = 1;
                    stack.components.retain(|(id, _)| id != "minecraft:attribute_modifiers");
                    log::info!("Apotheosis: chest loot {} at {} ({} affixes)", entry.item, d.rarities[rolled.rarity].id, rolled.affixes.len());
                    apply(stack, patch(&entry.item, &rolled, None));
                }
                None => *stack.count = 0,
            }
            true
        }
        // Gear a table dropped: now and then an affix item of a random rarity.
        "apotheosis:affix_conversion" => {
            if !has_player || has_affixes(stack.components) {
                return true;
            }
            let Some(category) = category(stack.item) else { return true };
            let chance = args["chance"].as_f64().unwrap_or(0.35) as f32;
            if rng.next_f32() > chance {
                return true;
            }
            let rarity = random_rarity(&[], tier, luck, &mut rng);
            let rolled = roll(category, rarity, tier, luck, &mut rng);
            let item = stack.item.clone();
            log::info!("Apotheosis: {} turned {} ({} affixes)", item, data().rarities[rolled.rarity].id, rolled.affixes.len());
            let p = patch(&item, &rolled, current_modifiers(stack.components));
            apply(stack, p);
            true
        }
        // A gem from a monster or a chest.
        "apotheosis:random_gem" => {
            let made = if has_player { crate::apotheosis_gems::random_gem(tier, luck, dimension, &mut rng) } else { None };
            match made {
                Some(gem) => {
                    log::info!("Apotheosis: gem {} at {}", gem.item, gem.purity);
                    *stack.item = gem.item;
                    *stack.count = 1;
                    stack.names.retain(|(k, _)| k != "minecraft:custom_name");
                    stack.names.push(("minecraft:custom_name".to_owned(), gem.name));
                    for (id, value) in gem.components {
                        stack.components.retain(|(c, _)| *c != id);
                        stack.components.push((id, value));
                    }
                }
                None => *stack.count = 0,
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sword_defaults() {
        let mut components = Map::new();
        components.insert(
            "minecraft:attribute_modifiers".into(),
            json!([{"type": "minecraft:attack_damage", "id": "minecraft:base_attack_damage", "amount": 6.0, "operation": "add_value", "slot": "mainhand"}]),
        );
        record_defaults("minecraft:diamond_sword", &components);
        let mut helmet = Map::new();
        helmet.insert("minecraft:equippable".into(), json!({"slot": "head"}));
        record_defaults("minecraft:iron_helmet", &helmet);
    }

    #[test]
    fn data_loads() {
        let d = data();
        assert_eq!(d.rarities.len(), 5);
        assert_eq!(d.rarities[0].id, "apotheosis:common");
        assert!(d.affixes.iter().filter(|a| supported(a)).count() >= 20);
        assert_eq!(d.entries.len(), 50);
        for a in d.affixes.iter().filter(|a| supported(a)) {
            let path = a.id.split_once(':').unwrap().1;
            assert!(NAMES.iter().any(|(p, _, _)| *p == path), "no name for {}", a.id);
        }
    }

    #[test]
    fn weights_follow_tiers_and_luck() {
        assert_eq!(weight(&json!({"weight": 25, "quality": 0.1}), 3, 10.0), 26);
        assert_eq!(weight(&json!({"pinnacle": {"weight": 15}}), 0, 0.0), 0);
        assert_eq!(weight(&json!({"pinnacle": {"weight": 15, "quality": 0.75}}), 4, 4.0), 18);
    }

    #[test]
    fn step_values() {
        assert_eq!(step_value(&json!(0.5), 0.9), Some(0.5));
        assert_eq!(step_value(&json!({"min": 2.0, "max": 5.0, "step": 0.25}), 0.0), Some(2.0));
        assert_eq!(step_value(&json!({"min": 2.0, "max": 5.0, "step": 0.25}), 0.5), Some(3.5));
        assert_eq!(step_value(&json!({"min": 2.0, "max": 5.0, "step": 0.25}), 0.99), Some(4.75));
        assert_eq!(step_value(&json!({"min": 0.1, "max": 0.2}), 0.5), Some(0.15));
    }

    #[test]
    fn categories() {
        sword_defaults();
        assert_eq!(category("minecraft:diamond_sword"), Some("melee_weapon"));
        assert_eq!(category("minecraft:iron_pickaxe"), Some("breaker"));
        assert_eq!(category("minecraft:bow"), Some("bow"));
        assert_eq!(category("minecraft:iron_helmet"), Some("helmet"));
        assert_eq!(category("minecraft:shears"), Some("shears"));
        assert_eq!(category("minecraft:stick"), None);
    }

    #[test]
    fn a_rare_sword_rolls_named_affixes() {
        sword_defaults();
        let rare = data().rarities.iter().position(|r| r.id == "apotheosis:rare").unwrap();
        for seed in 0..200 {
            let mut rng = Rng::new(seed);
            let rolled = roll("melee_weapon", rare, 2, 0.0, &mut rng);
            assert!(!rolled.affixes.is_empty(), "seed {seed}");
            let mut ids: Vec<usize> = rolled.affixes.iter().map(|a| a.0).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), rolled.affixes.len(), "no affix twice");
            for &(i, _) in &rolled.affixes {
                assert!(supported(&data().affixes[i]));
            }
            let p = patch("minecraft:diamond_sword", &rolled, None);
            let mods = p.modifiers.as_array().unwrap();
            assert_eq!(mods[0]["id"], "minecraft:base_attack_damage", "keeps the sword's own damage");
            let attributes = rolled.affixes.iter().filter(|(i, _)| data().affixes[*i].typ == "apotheosis:attribute").count();
            assert_eq!(mods.len(), 1 + attributes);
            assert_eq!(p.name.unwrap()["color"], "#5555FF");
        }
    }

    #[test]
    fn tables_get_conversion_and_chests_injection() {
        let mut chest = json!({"pools": [{"rolls": 1, "entries": []}]});
        table_load("minecraft:chests/simple_dungeon", &mut chest);
        assert_eq!(chest["pools"].as_array().unwrap().len(), 3, "its own pool, the affix item and the gem");
        assert_eq!(chest["pools"][0]["modifier"][0]["type"], "apotheosis:affix_conversion");
        assert_eq!(chest["pools"][1]["entries"][0]["modifier"][0]["type"], "apotheosis:affix_loot");
        let mut fishing = json!({"pools": [{"rolls": 1, "entries": [], "modifier": {"type": "minecraft:set_count", "count": 1}}]});
        table_load("minecraft:gameplay/fishing/treasure", &mut fishing);
        assert_eq!(fishing["pools"][0]["modifier"][0]["type"], "minecraft:set_count");
        assert_eq!(fishing["pools"][0]["modifier"][1]["type"], "apotheosis:affix_conversion");
        let mut block = json!({"pools": [{"rolls": 1, "entries": []}]});
        table_load("minecraft:blocks/stone", &mut block);
        assert!(block["pools"][0].get("modifier").is_none());
        assert_eq!(block["pools"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn chest_item_without_a_player_is_dropped() {
        let (mut item, mut count, mut names, mut components) = ("minecraft:stick".to_owned(), 1, Vec::new(), Vec::new());
        let mut stack = Stack { item: &mut item, count: &mut count, names: &mut names, components: &mut components };
        assert!(loot_function("apotheosis:affix_loot", &json!({}), false, 0.0, 7, None, &mut stack));
        assert_eq!(count, 0);
    }

    #[test]
    fn chest_item_with_a_player_is_rolled() {
        sword_defaults();
        let mut rolled = 0;
        for seed in 0..100 {
            let (mut item, mut count, mut names, mut components) = ("minecraft:stick".to_owned(), 1, Vec::new(), Vec::new());
            let mut stack = Stack { item: &mut item, count: &mut count, names: &mut names, components: &mut components };
            loot_function("apotheosis:affix_loot", &json!({}), true, 0.0, seed, None, &mut stack);
            if count == 1 {
                assert_ne!(item, "minecraft:stick");
                assert!(has_affixes(&components));
                rolled += 1;
            }
        }
        assert!(rolled > 90);
    }

    #[test]
    fn tiers_by_name() {
        assert_eq!(tier_named("Summit"), Some(3));
        assert_eq!(tier_named("nope"), None);
        set_tier(2);
        assert_eq!(tier(), 2);
        set_tier(0);
    }
}
