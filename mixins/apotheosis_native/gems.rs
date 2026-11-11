// Apotheosis for Horizon: gems and sockets (slice 0.2).
//
// Injected into horizon-world as `pub mod apotheosis_gems`, beside `apotheosis`. Gems drop from monsters a player
// kills and turn up in chests (Apotheosis' `gems` loot modifiers), each at a purity rolled by the world tier; an
// affix item has sockets from its rarity's `socket` rule, and an anvil puts a gem into a free socket, adding the
// gem's bonus for that kind of item as attribute modifiers. Only bonuses on attributes the game has
// (`minecraft:...`) are carried: a gem drops only if it has one, and goes only into an item it has one for.

use crate::apotheosis::{self as core, Rng};
use serde_json::{Map, Value, json};
use std::sync::OnceLock;

/// Apotheosis' purities, lowest first, with their colours (perfect's rainbow shown as gold).
pub const PURITIES: [&str; 6] = ["cracked", "chipped", "flawed", "normal", "flawless", "perfect"];
const PURITY_COLORS: [&str; 6] = ["#808080", "#33FF33", "#5555FF", "#BB00BB", "#ED7014", "#FFD700"];
const PURITY_NAMES: [&str; 6] = ["Cracked", "Chipped", "Flawed", "", "Flawless", "Perfect"];
const SOCKET_COLOR: &str = "#AABBCC";

pub fn purity_named(name: &str) -> Option<usize> {
    PURITIES.iter().position(|p| *p == name)
}

// ---- data ---------------------------------------------------------------------

struct Modifier {
    attribute: String,
    operation: String,
    values: Map<String, Value>,
}

struct Bonus {
    key: String,
    types: Vec<String>,
    /// The bonus's modifiers on attributes the game has; empty when it has none (or isn't an attribute bonus).
    mods: Vec<Modifier>,
    multi: bool,
}

impl Bonus {
    fn fits(&self, category: &str) -> bool {
        self.types.iter().any(|t| t == category || t == "*")
    }
    /// `GemBonus.supports`: the first modifier has a value at that purity.
    fn supports(&self, purity: usize) -> bool {
        self.mods.first().is_some_and(|m| m.values.contains_key(PURITIES[purity]))
    }
}

pub struct Gem {
    pub id: String,
    weights: Value,
    dimensions: Vec<String>,
    tiers: Vec<String>,
    min_purity: usize,
    pub unique: bool,
    bonuses: Vec<Bonus>,
}

impl Gem {
    /// The gem's path, `core/ballast`.
    pub fn path(&self) -> &str {
        self.id.split_once(':').map_or(self.id.as_str(), |(_, p)| p)
    }
    /// Its item, `apotheosis:gem_ballast` (each gem is an item of its own here, with its own sprite).
    pub fn item(&self) -> String {
        format!("apotheosis:gem_{}", self.path().rsplit('/').next().unwrap_or_default())
    }
    /// Whether any of its bonuses works here.
    fn usable(&self) -> bool {
        self.bonuses.iter().any(|b| !b.mods.is_empty())
    }
    fn bonus_for(&self, category: &str, purity: usize) -> Option<&Bonus> {
        self.bonuses.iter().find(|b| b.fits(category) && b.supports(purity))
    }
}

struct GemData {
    gems: Vec<Gem>,
    purity: Value,
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default()
}

fn modifier(m: &Value) -> Option<Modifier> {
    let attribute = m["attribute"].as_str()?;
    attribute.starts_with("minecraft:").then(|| Modifier {
        attribute: attribute.to_owned(),
        operation: m["operation"].as_str().unwrap_or("add_value").to_owned(),
        values: m["values"].as_object().cloned().unwrap_or_default(),
    })
}

fn data() -> &'static GemData {
    static DATA: OnceLock<GemData> = OnceLock::new();
    DATA.get_or_init(|| {
        use crate::apotheosis_data as d;
        let gems: Value = serde_json::from_str(d::GEMS).expect("Apotheosis: embedded gems read");
        let purity: Value = serde_json::from_str(d::PURITY_WEIGHTS).expect("Apotheosis: embedded purities read");
        let gems = gems
            .as_array()
            .into_iter()
            .flatten()
            .map(|g| Gem {
                id: g["id"].as_str().unwrap_or_default().to_owned(),
                weights: g["weights"].clone(),
                dimensions: strings(g["constraints"].get("dimensions")),
                tiers: strings(g["constraints"].get("tiers")),
                min_purity: purity_named(g["min_purity"].as_str().unwrap_or("cracked")).unwrap_or(0),
                unique: g["unique"].as_bool().unwrap_or(false),
                bonuses: g["bonuses"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|b| {
                        let multi = b["type"] == "apotheosis:multi_attribute";
                        let mods = match b["type"].as_str() {
                            Some("apotheosis:attribute") => modifier(b).into_iter().collect(),
                            Some("apotheosis:multi_attribute") => b["modifiers"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(modifier)
                                .collect(),
                            _ => Vec::new(),
                        };
                        Bonus {
                            key: b["key"].as_str().unwrap_or("anything").to_owned(),
                            types: strings(b.get("types")),
                            mods,
                            multi,
                        }
                    })
                    .collect(),
            })
            .collect();
        GemData { gems, purity }
    })
}

pub fn gem_named(id: &str) -> Option<&'static Gem> {
    data().gems.iter().find(|g| g.id == id)
}

pub fn gem_for_item(item: &str) -> Option<&'static Gem> {
    data().gems.iter().find(|g| g.item() == item)
}

/// Every gem's item id and its own English name, for the content mod's lang file and tests.
pub fn gem_items() -> Vec<(String, String)> {
    data().gems.iter().map(|g| (g.item(), english(g))).collect()
}

fn english(gem: &Gem) -> String {
    let word = gem.path().rsplit('/').next().unwrap_or_default();
    let title: Vec<String> = word
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        })
        .collect();
    format!("Gem of {}", title.join(" "))
}

/// Mobs of the monster category (Apotheosis' `is_monster`), which drop gems to a player.
const MONSTERS: &[&str] = &[
    "zombie", "husk", "drowned", "zombie_villager", "skeleton", "stray", "bogged", "wither_skeleton", "creeper",
    "spider", "cave_spider", "enderman", "endermite", "silverfish", "witch", "slime", "magma_cube", "blaze", "ghast",
    "zombified_piglin", "piglin", "piglin_brute", "hoglin", "zoglin", "pillager", "vindicator", "evoker", "ravager",
    "vex", "illusioner", "phantom", "guardian", "elder_guardian", "shulker", "warden", "breeze", "creaking", "wither",
    "ender_dragon", "giant",
];

pub fn is_monster(kind: &str) -> bool {
    kind.strip_prefix("minecraft:").is_some_and(|k| MONSTERS.contains(&k))
}

// ---- rolling ------------------------------------------------------------------

/// A purity weighted by the world tier and luck (`Purity.random`); any alike when none weighs anything.
pub fn random_purity(tier: usize, luck: f32, rng: &mut Rng) -> usize {
    let weights = &data().purity[core::TIERS[tier]];
    let pool: Vec<(usize, i64)> = (0..PURITIES.len())
        .map(|i| (i, weights.get(PURITIES[i]).map_or(0, |w| core::weight(w, tier, luck))))
        .collect();
    core::pick(rng, &pool).unwrap_or_else(|| rng.below(PURITIES.len() as u64) as usize)
}

/// A gem as an item: its id, purity, name and components.
pub struct MadeGem {
    pub item: String,
    pub purity: &'static str,
    pub name: Value,
    pub components: Vec<(String, Value)>,
}

/// A random gem for a loot roll in `dimension`: one this port can use, allowed by its constraints, weighted by
/// tier and luck, at a rolled purity no lower than the gem's own lowest.
pub fn random_gem(tier: usize, luck: f32, dimension: Option<&str>, rng: &mut Rng) -> Option<MadeGem> {
    let d = data();
    let pool: Vec<(usize, i64)> = d
        .gems
        .iter()
        .enumerate()
        .filter(|(_, g)| g.usable())
        .filter(|(_, g)| g.tiers.is_empty() || g.tiers.iter().any(|t| t == core::TIERS[tier]))
        .filter(|(_, g)| g.dimensions.is_empty() || dimension.is_some_and(|dim| g.dimensions.iter().any(|x| x == dim)))
        .map(|(i, g)| (i, core::weight(&g.weights, tier, luck)))
        .collect();
    let gem = &d.gems[core::pick(rng, &pool)?];
    let purity = random_purity(tier, luck, rng).max(gem.min_purity);
    Some(gem_stack(gem, purity))
}

pub fn gem_stack(gem: &Gem, purity: usize) -> MadeGem {
    let mut components = vec![
        ("minecraft:custom_data".to_owned(), json!({"apotheosis": {"gem": gem.id, "purity": PURITIES[purity]}})),
        ("minecraft:lore".to_owned(), gem_lore(gem, purity)),
    ];
    if purity == PURITIES.len() - 1 {
        components.push(("minecraft:enchantment_glint_override".to_owned(), json!(true)));
    }
    MadeGem { item: gem.item(), purity: PURITIES[purity], name: gem_name(gem, purity), components }
}

// ---- text ---------------------------------------------------------------------

fn gem_name(gem: &Gem, purity: usize) -> Value {
    let item = gem.item();
    let (ns, path) = item.split_once(':').unwrap_or(("apotheosis", &item));
    json!({
        "translate": "item.apotheosis.gem_name", "fallback": "%s %s",
        "with": [
            {"translate": format!("purity.apotheosis.{}", PURITIES[purity]), "fallback": PURITY_NAMES[purity]},
            {"translate": format!("item.{ns}.{path}"), "fallback": english(gem)},
        ],
        "color": PURITY_COLORS[purity], "italic": false,
    })
}

fn number(value: f64) -> String {
    let text = format!("{:.2}", value);
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// One bonus's effect at a purity: "+3.5 Attack Damage", "+15% Speed", joined by " / ".
fn bonus_effect(bonus: &Bonus, purity: usize) -> Vec<Value> {
    let mut parts = Vec::new();
    for (i, m) in bonus.mods.iter().enumerate() {
        let Some(v) = m.values.get(PURITIES[purity]).and_then(Value::as_f64) else { continue };
        if i > 0 {
            parts.push(json!({"text": " / "}));
        }
        let amount = if m.operation == "add_value" { format!("+{} ", number(v)) } else { format!("+{}% ", number(v * 100.0)) };
        let path = m.attribute.strip_prefix("minecraft:").unwrap_or(&m.attribute);
        parts.push(json!({"text": amount}));
        parts.push(json!({"translate": format!("attribute.name.{path}")}));
    }
    parts
}

fn class_name(key: &str) -> Value {
    let fallback: String = key.replace('_', " ");
    json!({"translate": format!("gem_class.apotheosis.{key}"), "fallback": fallback})
}

/// A gem's tooltip: unique, then a line per kind of item it goes into with what it gives there.
fn gem_lore(gem: &Gem, purity: usize) -> Value {
    let mut lines = Vec::new();
    if gem.unique {
        lines.push(json!({"translate": "text.apotheosis.unique", "fallback": "Unique: one per item", "color": "#C73912", "italic": false}));
    }
    lines.push(json!({"translate": "text.apotheosis.when_socketed_in", "fallback": "When socketed in:", "color": "gold", "italic": false}));
    for bonus in gem.bonuses.iter().filter(|b| b.supports(purity)) {
        let mut extra = vec![class_name(&bonus.key), json!({"text": ": "})];
        extra.extend(bonus_effect(bonus, purity));
        lines.push(json!({"text": "• ", "color": "gold", "italic": false, "extra": extra}));
    }
    Value::Array(lines)
}

/// An item's lore from its `custom_data`: the rarity, then one line per socket, the gem in it or empty.
pub fn lore_for(custom: &Value) -> Value {
    let a = &custom["apotheosis"];
    let mut lines = Vec::new();
    if let Some(line) = a["rarity"].as_str().and_then(core::rarity_line) {
        lines.push(line);
    }
    let sockets = a["sockets"].as_u64().unwrap_or(0) as usize;
    let gems = a["gems"].as_array().cloned().unwrap_or_default();
    for i in 0..sockets {
        let filled = gems.get(i).and_then(|g| {
            let gem = gem_named(g["gem"].as_str()?)?;
            let purity = purity_named(g["purity"].as_str()?)?;
            Some(gem_name(gem, purity))
        });
        lines.push(match filled {
            Some(name) => json!({"text": "◆ ", "color": SOCKET_COLOR, "italic": false, "extra": [name]}),
            None => json!({"text": "◇ ", "color": SOCKET_COLOR, "italic": false,
                           "extra": [{"translate": "socket.apotheosis.empty", "fallback": "Empty Socket"}]}),
        });
    }
    Value::Array(lines)
}

/// The modifiers an item's socketed gems give, from its `custom_data`: gem `i` adds its bonus for the item's kind
/// at its purity, with ids `apotheosis:gem/<path>_<slot>_<i><salt>`.
pub fn gem_modifiers(item: &str, custom: &Value) -> Vec<Value> {
    let Some(category) = core::category(item) else { return Vec::new() };
    let slot = core::slot_group(category);
    let mut out = Vec::new();
    for (index, g) in custom["apotheosis"]["gems"].as_array().into_iter().flatten().enumerate() {
        let Some(gem) = g["gem"].as_str().and_then(gem_named) else { continue };
        let purity = g["purity"].as_str().and_then(purity_named).unwrap_or(0).max(gem.min_purity);
        let Some(bonus) = gem.bonus_for(category, purity) else { continue };
        for (i, m) in bonus.mods.iter().enumerate() {
            let Some(amount) = m.values.get(PURITIES[purity]).and_then(Value::as_f64) else { continue };
            let salt = if bonus.multi { i.to_string() } else { String::new() };
            out.push(json!({
                "type": m.attribute,
                "id": format!("apotheosis:gem/{}_{slot}_{index}{salt}", gem.path()),
                "amount": amount,
                "operation": m.operation,
                "slot": slot,
            }));
        }
    }
    out
}

// ---- socketing ----------------------------------------------------------------

/// An item with one more gem: its new `custom_data`, `attribute_modifiers` and `lore`.
pub struct Socketed {
    pub custom: Value,
    pub modifiers: Value,
    pub lore: Value,
}

/// `SocketHelper.socketGem`, at the anvil: `item` (its `custom_data` and `attribute_modifiers`) takes the gem
/// `gem_item` (its `custom_data`) into its first free socket, when it has one, the gem has a bonus that works for
/// this kind of item at its purity, and a unique gem isn't in it already.
pub fn socket(item: &str, custom: Option<&Value>, modifiers: Option<&Value>, gem_item: &str, gem_custom: Option<&Value>) -> Option<Socketed> {
    let category = core::category(item)?;
    let gem = gem_for_item(gem_item)?;
    let stored = gem_custom.and_then(|c| c["apotheosis"]["purity"].as_str()).and_then(purity_named).unwrap_or(0);
    let purity = stored.max(gem.min_purity);
    let mut custom = custom.cloned().unwrap_or_else(|| json!({}));
    let a = custom.get("apotheosis")?.clone();
    let sockets = a["sockets"].as_u64().unwrap_or(0) as usize;
    let mut gems = a["gems"].as_array().cloned().unwrap_or_default();
    if gems.len() >= sockets {
        return None;
    }
    if gem.unique && gems.iter().any(|g| g["gem"].as_str() == Some(&gem.id)) {
        return None;
    }
    gem.bonus_for(category, purity)?;
    gems.push(json!({"gem": gem.id, "purity": PURITIES[purity]}));
    custom["apotheosis"]["gems"] = Value::Array(gems);
    // The item kind's modifiers (or what the stack had, outside this mod's), then the affixes' and gems' again.
    let mut list: Vec<Value> = modifiers
        .or(core::defaults_of(item).modifiers.as_ref())
        .map(core::modifier_list)
        .unwrap_or_default();
    list.retain(|m| !m["id"].as_str().is_some_and(|id| id.starts_with("apotheosis:")));
    list.extend(core::own_modifiers(item, &custom));
    let lore = lore_for(&custom);
    Some(Socketed { custom, modifiers: Value::Array(list), lore })
}

// ---- loot tables --------------------------------------------------------------

/// Apotheosis' `gem_entity_drops` (0.5%) and `gem_entity_drops_from_real_players` (4%), together, for a monster a
/// player kills; and `gem_loot_injection` for chests (25% the game's, 20% other mods').
pub fn table_load(id: &str, pools: &mut Vec<Value>) {
    let (ns, path) = id.split_once(':').unwrap_or(("minecraft", id));
    let gem_pool = |terms: Vec<Value>| {
        json!({
            "rolls": 1,
            "condition": {"type": "minecraft:all_of", "terms": terms},
            "entries": [{"type": "minecraft:item", "name": "minecraft:stick", "modifier": [{"type": "apotheosis:random_gem"}]}],
        })
    };
    if path.starts_with("entities/") {
        pools.push(gem_pool(vec![
            json!({"type": "minecraft:killed_by_player"}),
            json!({"type": "apotheosis:is_monster"}),
            json!({"type": "minecraft:random_chance", "chance": 0.045}),
        ]));
    }
    let chance = if path.starts_with("chests") {
        Some(if ns == "minecraft" { 0.25 } else { 0.2 })
    } else if ns == "twilightforest" && path.starts_with("structures") {
        Some(0.2)
    } else {
        None
    };
    if let Some(chance) = chance {
        pools.push(gem_pool(vec![
            json!({"type": "apotheosis:has_player"}),
            json!({"type": "minecraft:random_chance", "chance": chance}),
        ]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sword_with_sockets(n: u64) -> Value {
        json!({"apotheosis": {"rarity": "apotheosis:rare", "affixes": {}, "sockets": n}})
    }

    #[test]
    fn gems_load() {
        let d = data();
        assert_eq!(d.gems.len(), 21);
        assert!(d.gems.iter().filter(|g| g.usable()).count() >= 14);
        let ballast = gem_named("apotheosis:core/ballast").unwrap();
        assert_eq!(ballast.item(), "apotheosis:gem_ballast");
        assert!(gem_for_item("apotheosis:gem_ballast").is_some());
        let names: Vec<String> = d.gems.iter().map(Gem::item).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "gem items are unique");
    }

    #[test]
    fn purities_follow_the_tier() {
        let mut rng = Rng::new(3);
        for _ in 0..200 {
            assert!(random_purity(0, 0.0, &mut rng) <= 2, "haven rolls cracked to flawed");
            assert!(random_purity(4, 0.0, &mut rng) >= 2, "pinnacle rolls flawed or better");
        }
    }

    #[test]
    fn random_gems_respect_dimensions_and_min_purity() {
        let mut rng = Rng::new(9);
        for _ in 0..300 {
            let gem = random_gem(4, 0.0, Some("minecraft:overworld"), &mut rng).unwrap();
            let g = gem_for_item(&gem.item).unwrap();
            assert!(g.usable());
            assert!(g.dimensions.is_empty() || g.dimensions.iter().any(|d| d == "minecraft:overworld"), "{}", g.id);
            assert!(purity_named(gem.purity).unwrap() >= g.min_purity);
        }
    }

    #[test]
    fn a_gem_goes_into_a_free_socket() {
        let ballast = gem_named("apotheosis:core/ballast").unwrap();
        let made = gem_stack(ballast, 3);
        let gem_custom = &made.components[0].1;
        let base = json!([{"type": "minecraft:attack_damage", "id": "minecraft:base_attack_damage", "amount": 6.0, "operation": "add_value", "slot": "mainhand"}]);
        let out = socket("minecraft:diamond_sword", Some(&sword_with_sockets(2)), Some(&base), &made.item, Some(gem_custom)).unwrap();
        let mods = out.modifiers.as_array().unwrap();
        assert_eq!(mods.len(), 2);
        assert_eq!(mods[1]["id"], "apotheosis:gem/core/ballast_mainhand_0");
        assert_eq!(mods[1]["amount"], 5.0);
        assert_eq!(out.custom["apotheosis"]["gems"][0]["purity"], "normal");
        assert_eq!(out.lore.as_array().unwrap().len(), 3, "rarity, the gem, one empty socket");
        // the second socket takes another, the third is refused
        let again = socket("minecraft:diamond_sword", Some(&out.custom), Some(&out.modifiers), &made.item, Some(gem_custom)).unwrap();
        assert!(socket("minecraft:diamond_sword", Some(&again.custom), Some(&again.modifiers), &made.item, Some(gem_custom)).is_none());
    }

    #[test]
    fn socketing_keeps_the_affixes_without_the_stack_modifiers() {
        // The window's copy of a stack has no `attribute_modifiers`: the affixes' come back from `custom_data`.
        let custom = json!({"apotheosis": {"rarity": "apotheosis:rare", "sockets": 1,
                            "affixes": {"apotheosis:melee/attribute/violent": 0.5}}});
        let ballast = gem_named("apotheosis:core/ballast").unwrap();
        let made = gem_stack(ballast, 2);
        let out = socket("minecraft:diamond_sword", Some(&custom), None, &made.item, Some(&made.components[0].1)).unwrap();
        let ids: Vec<&str> = out.modifiers.as_array().unwrap().iter().filter_map(|m| m["id"].as_str()).collect();
        assert!(ids.contains(&"apotheosis:affix/melee/attribute/violent"), "{ids:?}");
        assert!(ids.contains(&"apotheosis:gem/core/ballast_mainhand_0"), "{ids:?}");
        assert_eq!(core::own_modifiers("minecraft:diamond_sword", &out.custom).len(), 2);
    }

    #[test]
    fn unique_gems_and_wrong_items_are_refused() {
        let samurai = gem_named("apotheosis:core/samurai").unwrap();
        assert!(samurai.unique);
        let made = gem_stack(samurai, 3);
        let gem_custom = &made.components[0].1;
        // samurai's leggings bonus is toughness and speed
        let first = socket("minecraft:iron_leggings", Some(&sword_with_sockets(2)), None, &made.item, Some(gem_custom)).unwrap();
        assert!(socket("minecraft:iron_leggings", Some(&first.custom), None, &made.item, Some(gem_custom)).is_none());
        // no sockets, no gem
        assert!(socket("minecraft:diamond_sword", None, None, &made.item, Some(gem_custom)).is_none());
        // not a gem
        assert!(socket("minecraft:diamond_sword", Some(&sword_with_sockets(1)), None, "minecraft:diamond", None).is_none());
    }

    #[test]
    fn tables_get_gem_pools() {
        let mut pools = Vec::new();
        table_load("minecraft:entities/zombie", &mut pools);
        assert_eq!(pools.len(), 1);
        table_load("minecraft:chests/simple_dungeon", &mut pools);
        assert_eq!(pools.len(), 2);
        table_load("minecraft:blocks/stone", &mut pools);
        assert_eq!(pools.len(), 2);
        assert!(is_monster("minecraft:zombie"));
        assert!(!is_monster("minecraft:cow"));
    }
}
