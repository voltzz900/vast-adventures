// Apotheosis for Horizon: bosses and elites (slice 0.3).
//
// Injected into horizon-world as `pub mod apotheosis_bosses`. The game's side (apotheosis_client) asks it, for
// each monster the world spawns naturally, whether a boss takes its place (Apotheosis' invaders: a chance per
// dimension and world tier) or whether the mob becomes an elite, and gets back a plan: the entity, its rolled
// stats, effects, name and gear, the gear's guaranteed affix item and, now and then, a gem it drops.

use crate::apotheosis::{self as core, Rng};
use serde_json::{Map, Value, json};
use std::sync::OnceLock;

/// A boss or elite, ready for the game to summon or apply.
pub struct Plan {
    pub kind: String,
    /// The rarity's colour, for the announcement (bosses only).
    pub rarity: Option<String>,
    /// (attribute, operation, value), on the attributes Horizon loads from a mob's data.
    pub attrs: Vec<(String, String, f64)>,
    /// (effect, amplifier, ticks; -1 for ever).
    pub effects: Vec<(String, i32, i32)>,
    pub name: Value,
    /// (equipment slot, item as `{id, count, components}`).
    pub equipment: Vec<(String, Value)>,
    /// Slots whose item always drops.
    pub always_drops: Vec<String>,
}

/// The attributes a mob's data sets bases for in Horizon (`entity_nbt::mob_from_nbt`).
const LOADED: &[&str] = &["minecraft:max_health", "minecraft:movement_speed", "minecraft:attack_damage", "minecraft:attack_knockback"];

const SLOTS: [(&str, &str); 6] = [
    ("mainhands", "mainhand"),
    ("offhands", "offhand"),
    ("helmets", "head"),
    ("chestplates", "chest"),
    ("leggings", "legs"),
    ("boots", "feet"),
];

struct Data {
    invaders: Vec<Value>,
    elites: Vec<Value>,
    gear_sets: Map<String, Value>,
    gear_tags: Map<String, Value>,
    spawn_rules: Map<String, Value>,
}

fn data() -> &'static Data {
    static DATA: OnceLock<Data> = OnceLock::new();
    DATA.get_or_init(|| {
        use crate::apotheosis_data as d;
        let read = |text: &str| serde_json::from_str::<Value>(text).expect("Apotheosis: embedded bosses read");
        let list = |v: Value| v.as_array().cloned().unwrap_or_default();
        let map = |v: Value| v.as_object().cloned().unwrap_or_default();
        Data {
            invaders: list(read(d::INVADERS)),
            elites: list(read(d::ELITES)),
            gear_sets: map(read(d::GEAR_SETS)),
            gear_tags: map(read(d::GEAR_TAGS)),
            spawn_rules: map(read(d::SPAWN_RULES)),
        }
    })
}

/// The chance a monster the world spawns in `dimension` is a boss instead, at the world tier.
pub fn spawn_chance(dimension: &str, tier: usize) -> f64 {
    data().spawn_rules.get(dimension).and_then(|r| r["spawn_chances"][core::TIERS[tier]].as_f64()).unwrap_or(0.0)
}

fn allowed(constraints: &Value, dimension: &str, biome: Option<&str>, tier: usize) -> bool {
    let list = |key: &str| constraints.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
    let dims = list("dimensions");
    let tiers = list("tiers");
    // Biome-bound variants (a bogged in wet biomes...) only where the biome is known and in their tag.
    if let Some(tag) = constraints.get("biomes").filter(|b| !b.is_null()) {
        let Some(biome) = biome else { return false };
        let ok = match tag {
            Value::String(t) => biome_in(t, biome),
            Value::Array(list) => list.iter().filter_map(Value::as_str).any(|t| biome_in(t, biome)),
            _ => false,
        };
        if !ok {
            return false;
        }
    }
    (dims.is_empty() || dims.iter().any(|d| d == dimension)) && (tiers.is_empty() || tiers.iter().any(|t| t == core::TIERS[tier]))
}

/// Whether `biome` is in `tag` (a biome id, or one of the convention tags the bosses name: the game has none, so
/// these are this port's own lists of the vanilla biomes they mean).
pub fn biome_in(tag: &str, biome: &str) -> bool {
    let Some(tag) = tag.strip_prefix('#') else { return tag == biome };
    let biome = biome.strip_prefix("minecraft:").unwrap_or(biome);
    let list: &[&str] = match tag {
        "c:is_wet/overworld" | "c:is_wet" => &[
            "swamp", "mangrove_swamp", "jungle", "bamboo_jungle", "sparse_jungle", "beach", "lush_caves", "river",
        ],
        "c:is_dry/overworld" | "c:is_dry" => &[
            "desert", "badlands", "wooded_badlands", "eroded_badlands", "savanna", "savanna_plateau", "windswept_savanna",
        ],
        "c:is_cold/overworld" | "c:is_cold" => &[
            "taiga", "old_growth_pine_taiga", "old_growth_spruce_taiga", "snowy_taiga", "snowy_plains", "ice_spikes",
            "snowy_beach", "frozen_river", "grove", "snowy_slopes", "jagged_peaks", "frozen_peaks", "windswept_hills",
            "windswept_gravelly_hills", "windswept_forest", "stony_peaks", "frozen_ocean", "deep_frozen_ocean",
            "cold_ocean", "deep_cold_ocean",
        ],
        "c:is_snowy" => &[
            "snowy_plains", "ice_spikes", "snowy_taiga", "snowy_beach", "frozen_river", "grove", "snowy_slopes",
            "jagged_peaks", "frozen_peaks",
        ],
        _ => &[],
    };
    list.contains(&biome)
}

fn roll_value(value: &Value, rng: &mut Rng) -> f64 {
    match value {
        Value::Object(range) => {
            let min = range.get("min").and_then(Value::as_f64).unwrap_or(0.0);
            let max = range.get("max").and_then(Value::as_f64).unwrap_or(min);
            min + (max - min) * f64::from(rng.next_f32())
        }
        other => other.as_f64().unwrap_or(0.0),
    }
}

/// `BossStats`: attribute modifiers (the ones Horizon loads) rolled in their ranges, and effects for ever.
fn stats(stats: &Value, rng: &mut Rng) -> (Vec<(String, String, f64)>, Vec<(String, i32, i32)>) {
    let mut attrs = Vec::new();
    for m in stats["attribute_modifiers"].as_array().into_iter().flatten() {
        let attribute = m["attribute"].as_str().unwrap_or_default();
        let value = roll_value(&m["value"], rng);
        if LOADED.contains(&attribute) {
            attrs.push((attribute.to_owned(), m["operation"].as_str().unwrap_or("add_value").to_owned(), value));
        }
    }
    let mut effects = Vec::new();
    for e in stats["effects"].as_array().into_iter().flatten() {
        let chance = e["chance"].as_f64().unwrap_or(1.0) as f32;
        if rng.next_f32() <= chance {
            let amplifier = e["amplifier"].as_f64().unwrap_or(0.0) as i32;
            effects.push((e["effect"].as_str().unwrap_or_default().to_owned(), amplifier, -1));
        }
    }
    (attrs, effects)
}

/// A weighted entry `{weight, quality?, stack}`'s stack, as `{id, count, components}`.
fn pick_stack(entries: &Value, tier: usize, luck: f32, rng: &mut Rng) -> Option<Value> {
    let list = entries.as_array()?;
    let weighted: Vec<(usize, i64)> = list.iter().enumerate().map(|(i, e)| (i, core::weight(e, tier, luck))).collect();
    let chosen = &list[core::pick(rng, &weighted)?];
    let stack = &chosen["stack"];
    let mut out = json!({"id": stack["id"], "count": stack["count"].as_i64().unwrap_or(1)});
    if let Some(components) = stack.get("components") {
        out["components"] = components.clone();
    }
    Some(out)
}

/// The gear of a set from the tier's gear-set tag (`valid_gear_sets`): one weighted item per slot it lists.
fn gear(gear_sets: &Value, tier: usize, luck: f32, rng: &mut Rng) -> Vec<(String, Value)> {
    let d = data();
    let Some(tag) = gear_sets[core::TIERS[tier]].as_str() else { return Vec::new() };
    let ids: Vec<&str> = d.gear_tags.get(tag.trim_start_matches('#')).and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).collect();
    let sets: Vec<(usize, i64)> = ids
        .iter()
        .enumerate()
        .filter_map(|(i, id)| d.gear_sets.get(*id).map(|s| (i, core::weight(s, tier, luck))))
        .collect();
    let Some(chosen) = core::pick(rng, &sets).and_then(|i| d.gear_sets.get(ids[i])) else { return Vec::new() };
    SLOTS
        .iter()
        .filter_map(|(list, slot)| Some((slot.to_string(), pick_stack(&chosen[*list], tier, luck, rng)?)))
        .collect()
}

/// Makes one worn item affix loot at `rarity` (`modifyBossItem`, without the extra levels): the slot is
/// picked among those holding an item with a loot category. Returns the slot.
fn affix_one(equipment: &mut [(String, Value)], rarity: usize, tier: usize, luck: f32, rng: &mut Rng) -> Option<String> {
    let candidates: Vec<usize> = (0..equipment.len())
        .filter(|&i| equipment[i].1["id"].as_str().and_then(core::category).is_some())
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let i = candidates[rng.below(candidates.len() as u64) as usize];
    let (slot, item) = &mut equipment[i];
    let id = item["id"].as_str().unwrap_or_default().to_owned();
    let category = core::category(&id)?;
    let rolled = core::roll(category, rarity, tier, luck, rng);
    let patch = core::patch(&id, &rolled, None);
    let components = item.as_object_mut()?.entry("components").or_insert_with(|| json!({}));
    if let Some(name) = patch.name {
        components["minecraft:custom_name"] = name;
    }
    components["minecraft:lore"] = patch.lore;
    components["minecraft:attribute_modifiers"] = patch.modifiers;
    components["minecraft:custom_data"] = patch.custom;
    Some(slot.clone())
}

/// A gem in the off hand that always drops (Apotheosis' `boss_drops`: a gem 30% of the time).
fn gem_drop(plan: &mut Plan, tier: usize, luck: f32, dimension: &str, rng: &mut Rng) {
    if rng.next_f32() > 0.3 || plan.equipment.iter().any(|(slot, _)| slot == "offhand") {
        return;
    }
    let Some(gem) = crate::apotheosis_gems::random_gem(tier, luck, Some(dimension), rng) else { return };
    let mut components = Map::new();
    components.insert("minecraft:custom_name".into(), gem.name);
    for (id, value) in gem.components {
        components.insert(id, value);
    }
    plan.equipment.push(("offhand".into(), json!({"id": gem.item, "count": 1, "components": components})));
    plan.always_drops.push("offhand".into());
}

// ---- names --------------------------------------------------------------------

const SYLLABLES: &[&str] = &[
    "ka", "ro", "thar", "mor", "gul", "zan", "vor", "eth", "ul", "dra", "kor", "nis", "sha", "bel", "tor", "gri",
    "mok", "ash", "val", "ren", "xa", "lum", "or", "dus", "vex", "ira", "gor", "sel",
];
/// (key, English, Portuguese) titles; the Portuguese ones are in the content mod's lang file.
pub const TITLES: &[(&str, &str, &str)] = &[
    ("destroyer", "the Destroyer", "o Destruidor"),
    ("unyielding", "the Unyielding", "o Inabalável"),
    ("ravager", "the Ravager", "o Devastador"),
    ("bloodthirsty", "the Bloodthirsty", "o Sanguinário"),
    ("eternal", "the Eternal", "o Eterno"),
    ("cruel", "the Cruel", "o Cruel"),
    ("wicked", "the Wicked", "o Perverso"),
    ("grim", "the Grim", "o Sombrio"),
    ("butcher", "the Butcher", "o Carniceiro"),
    ("hollow", "the Hollow", "o Vazio"),
    ("stormborn", "the Stormborn", "o Filho da Tempestade"),
    ("devourer", "the Devourer", "o Devorador"),
];

/// A boss's name (`NameHelper.setEntityName`, with this port's own words): two to four syllables, and most of
/// the time a title.
pub fn boss_name(color: &str, rng: &mut Rng) -> Value {
    let mut count = 2;
    if rng.next_f32() < 0.4 {
        count += 1;
        if rng.next_f32() < 0.15 {
            count += 1;
        }
    }
    let mut root = String::new();
    for _ in 0..count {
        root.push_str(SYLLABLES[rng.below(SYLLABLES.len() as u64) as usize]);
    }
    let mut chars = root.chars();
    let root: String = chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default();
    if rng.next_f32() < 0.8 {
        let (key, en, _) = TITLES[rng.below(TITLES.len() as u64) as usize];
        json!({"translate": "apotheosis.boss_name", "fallback": "%s %s",
               "with": [{"text": root}, {"translate": format!("apotheosis.boss_title.{key}"), "fallback": en}],
               "color": color})
    } else {
        json!({"text": root, "color": color})
    }
}

// ---- bosses and elites --------------------------------------------------------

/// A boss for a monster spawn in `dimension`: an invader allowed there, weighted by tier and luck, at a rarity
/// from those its stats list, with its tier's gear, one affix item that always drops, and sometimes a gem.
pub fn plan_boss(dimension: &str, biome: Option<&str>, tier: usize, luck: f32, rng: &mut Rng) -> Option<Plan> {
    let d = data();
    let pool: Vec<(usize, i64)> = d
        .invaders
        .iter()
        .enumerate()
        .filter(|(_, b)| allowed(&b["constraints"], dimension, biome, tier))
        .map(|(i, b)| (i, core::weight(&b["weights"], tier, luck)))
        .collect();
    let boss = &d.invaders[core::pick(rng, &pool)?];
    let rarities: Vec<String> = boss["stats"].as_object()?.keys().cloned().collect();
    let rarity = core::random_rarity(&rarities, tier, luck, rng);
    let rarity_id = core::rarity_id(rarity);
    let (attrs, mut effects) = stats(&boss["stats"][rarity_id.as_str()], rng);
    effects.push(("minecraft:glowing".into(), 0, 3600));
    let color = core::rarity_color(rarity);
    let mut equipment = gear(&boss["gear_sets"], tier, luck, rng);
    let always_drops = affix_one(&mut equipment, rarity, tier, luck, rng).into_iter().collect();
    let mut plan = Plan {
        kind: boss["entity"].as_str()?.to_owned(),
        rarity: Some(rarity_id),
        attrs,
        effects,
        name: boss_name(&color, rng),
        equipment,
        always_drops,
    };
    gem_drop(&mut plan, tier, luck, dimension, rng);
    Some(plan)
}

/// An elite for a mob of `kind` spawning in `dimension`: one that turns that kind, allowed there, weighted, then
/// its `success_chance`. It keeps the mob, adding its stats, name and gear, and maybe one affix item.
pub fn plan_elite(kind: &str, dimension: &str, tier: usize, luck: f32, rng: &mut Rng) -> Option<Plan> {
    let d = data();
    let pool: Vec<(usize, i64)> = d
        .elites
        .iter()
        .enumerate()
        .filter(|(_, e)| e["entities"].as_array().is_some_and(|l| l.iter().any(|x| x == kind)))
        .filter(|(_, e)| allowed(&e["constraints"], dimension, None, tier))
        .map(|(i, e)| (i, core::weight(&e["weights"], tier, luck)))
        .collect();
    let elite = &d.elites[core::pick(rng, &pool)?];
    if rng.next_f32() > elite["success_chance"].as_f64().unwrap_or(0.0) as f32 {
        return None;
    }
    let (attrs, effects) = stats(&elite["stats"], rng);
    let mut equipment = gear(&elite["gear_sets"], tier, luck, rng);
    let mut always_drops = Vec::new();
    if rng.next_f32() <= elite["affix_chance"].as_f64().unwrap_or(-1.0) as f32 {
        let pool: Vec<String> = elite["rarities"].as_array().into_iter().flatten().filter_map(|r| r.as_str().map(str::to_owned)).collect();
        let rarity = core::random_rarity(&pool, tier, luck, rng);
        always_drops.extend(affix_one(&mut equipment, rarity, tier, luck, rng));
    }
    let id = elite["id"].as_str().unwrap_or_default();
    let path = id.rsplit('/').next().unwrap_or_default();
    let name = json!({"translate": format!("apotheosis.elite.{path}"), "fallback": elite["name"].as_str().unwrap_or("Elite"), "color": "#FFAA00"});
    Some(Plan { kind: kind.to_owned(), rarity: None, attrs, effects, name, equipment, always_drops })
}

/// An attribute's value from its base and the plan's modifiers on it, as the game works them out: the base plus
/// every `add_value`, times one plus every `add_multiplied_base`, times one plus each `add_multiplied_total`.
pub fn final_value(base: f64, attribute: &str, attrs: &[(String, String, f64)]) -> f64 {
    let on = || attrs.iter().filter(|(a, _, _)| a == attribute);
    let added: f64 = on().filter(|(_, op, _)| op == "add_value").map(|(_, _, v)| v).sum();
    let base_mult: f64 = on().filter(|(_, op, _)| op == "add_multiplied_base").map(|(_, _, v)| v).sum();
    let mut value = (base + added) * (1.0 + base_mult);
    for (_, _, v) in on().filter(|(_, op, _)| op == "add_multiplied_total") {
        value *= 1.0 + v;
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_loads() {
        let d = data();
        assert_eq!(d.invaders.len(), 23);
        assert_eq!(d.elites.len(), 4);
        assert!(d.gear_sets.len() >= 30);
        assert_eq!(spawn_chance("minecraft:overworld", 0), 0.0);
        assert!(spawn_chance("minecraft:the_nether", 4) > 0.03);
    }

    #[test]
    fn bosses_have_gear_a_drop_and_a_name() {
        let mut gem_seen = false;
        for seed in 0..200 {
            let mut rng = Rng::new(seed);
            let plan = plan_boss("minecraft:overworld", Some("minecraft:plains"), 3, 0.0, &mut rng).expect("an overworld boss");
            assert!(!plan.equipment.is_empty(), "{} has gear", plan.kind);
            assert!(plan.attrs.iter().all(|(a, _, _)| LOADED.contains(&a.as_str())));
            assert!(plan.effects.iter().any(|(e, _, _)| e == "minecraft:glowing"));
            if let Some(slot) = plan.always_drops.first() {
                let item = &plan.equipment.iter().find(|(s, _)| s == slot).unwrap().1;
                assert!(item["components"]["minecraft:custom_data"]["apotheosis"].is_object() || item["id"].as_str().unwrap().starts_with("apotheosis:gem_"));
            }
            gem_seen |= plan.equipment.iter().any(|(s, i)| s == "offhand" && i["id"].as_str().unwrap().starts_with("apotheosis:gem_"));
            assert!(plan.name.is_object());
        }
        assert!(gem_seen, "some bosses hold a gem");
    }

    #[test]
    fn nether_bosses_stay_in_the_nether() {
        let mut rng = Rng::new(1);
        for _ in 0..100 {
            let plan = plan_boss("minecraft:the_nether", None, 4, 0.0, &mut rng).unwrap();
            assert!(["minecraft:blaze", "minecraft:piglin", "minecraft:piglin_brute", "minecraft:wither_skeleton",
                     "minecraft:zoglin", "minecraft:zombified_piglin"].contains(&plan.kind.as_str()), "{}", plan.kind);
        }
    }

    #[test]
    fn elites_are_rare_and_only_their_mobs() {
        let mut rng = Rng::new(5);
        let made = (0..20_000).filter(|_| plan_elite("minecraft:skeleton", "minecraft:overworld", 2, 0.0, &mut rng).is_some()).count();
        assert!(made > 20 && made < 500, "{made}");
        assert!(plan_elite("minecraft:cow", "minecraft:overworld", 2, 0.0, &mut rng).is_none());
    }

    #[test]
    fn attributes_combine_as_the_game_does() {
        let attrs = vec![
            ("minecraft:max_health".to_owned(), "add_value".to_owned(), 40.0),
            ("minecraft:max_health".to_owned(), "add_multiplied_base".to_owned(), 0.5),
            ("minecraft:max_health".to_owned(), "add_multiplied_total".to_owned(), 0.1),
        ];
        assert!((final_value(20.0, "minecraft:max_health", &attrs) - 99.0).abs() < 1e-9);
        assert_eq!(final_value(3.0, "minecraft:attack_damage", &attrs), 3.0);
    }

    #[test]
    fn biome_bound_bosses() {
        let mut rng = Rng::new(9);
        let kinds = |biome: Option<&str>, rng: &mut Rng| -> std::collections::HashSet<String> {
            (0..3000).filter_map(|_| plan_boss("minecraft:overworld", biome, 4, 0.0, rng)).map(|p| p.kind).collect()
        };
        let snowy = kinds(Some("minecraft:snowy_plains"), &mut rng);
        assert!(snowy.contains("minecraft:wolf"), "{snowy:?}");
        let plains = kinds(Some("minecraft:plains"), &mut rng);
        assert!(!plains.contains("minecraft:wolf"), "{plains:?}");
        assert!(biome_in("#c:is_dry/overworld", "minecraft:desert"));
        assert!(!biome_in("#c:is_dry/overworld", "minecraft:swamp"));
    }

}
