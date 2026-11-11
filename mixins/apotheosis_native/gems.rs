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

// ---- salvaging, reforging and gem cutting -------------------------------------------

/// The material each rarity salvages into and reforging costs, with (material, levels) per reforge.
const MATERIALS: [(&str, &str, i32, i32); 5] = [
    ("apotheosis:common", "apotheosis:mysterious_scrap_metal", 1, 2),
    ("apotheosis:uncommon", "apotheosis:timeworn_fabric", 2, 5),
    ("apotheosis:rare", "apotheosis:luminous_crystal_shard", 2, 15),
    ("apotheosis:epic", "apotheosis:arcane_sands", 2, 30),
    ("apotheosis:mythic", "apotheosis:godforged_pearl", 3, 50),
];
/// The gem dust a gem salvages into, (min, max) by purity.
const DUST: [(i32, i32); 6] = [(1, 2), (1, 3), (2, 4), (3, 5), (4, 7), (5, 10)];
pub const GEM_DUST: &str = "apotheosis:gem_dust";

pub fn is_material(item: &str) -> bool {
    MATERIALS.iter().any(|(_, m, _, _)| *m == item)
}

/// A roll seeded from the stack itself: the anvil and the grindstone work their result out again and again, on
/// the server and in the window alike, so it must come out the same each time for the same inputs.
fn stack_rng(item: &str, custom: &Value, what: &str) -> Rng {
    Rng::new(core::hash(item) ^ core::hash(&custom.to_string()).rotate_left(21) ^ core::hash(what))
}

/// What an affix item or a gem salvages into at the grindstone (Apotheosis' salvaging recipes): its rarity's
/// material, one to four, or gem dust by purity.
pub fn salvage(item: &str, custom: &Value) -> Option<(String, i32)> {
    let a = &custom["apotheosis"];
    let mut rng = stack_rng(item, custom, "salvage");
    if gem_for_item(item).is_some() {
        let purity = a["purity"].as_str().and_then(purity_named).unwrap_or(0);
        let (min, max) = DUST[purity];
        return Some((GEM_DUST.to_owned(), min + rng.below((max - min + 1) as u64) as i32));
    }
    let rarity = a["rarity"].as_str()?;
    let (_, material, _, _) = MATERIALS.iter().find(|(r, _, _, _)| *r == rarity)?;
    Some(((*material).to_owned(), 1 + rng.below(4) as i32))
}

/// An item reforged: new affixes at the same rarity, its sockets (at least as many) and gems kept.
pub struct Reforged {
    pub name: Option<Value>,
    pub custom: Value,
    pub modifiers: Value,
    pub lore: Value,
    pub levels: i32,
    pub material_cost: i32,
}

/// Reforging at the anvil (the reforging table's recipe): an affix item and its rarity's material, enough of it.
pub fn reforge(item: &str, custom: &Value, modifiers: Option<&Value>, material: &str, count: i32) -> Option<Reforged> {
    let category = core::category(item)?;
    let a = &custom["apotheosis"];
    let rarity_id = a["rarity"].as_str()?;
    let (_, wanted, cost, levels) = MATERIALS.iter().find(|(r, _, _, _)| *r == rarity_id)?;
    if *wanted != material || count < *cost {
        return None;
    }
    let rarity = (0..5).find(|&i| core::rarity_id(i) == rarity_id)?;
    let mut rng = stack_rng(item, custom, "reforge");
    let rolled = core::roll(category, rarity, core::tier(), 0.0, &mut rng);
    let patch = core::patch(item, &rolled, None);
    let mut new = patch.custom;
    let sockets = a["sockets"].as_u64().unwrap_or(0).max(u64::from(rolled.sockets));
    if sockets > 0 {
        new["apotheosis"]["sockets"] = json!(sockets);
    }
    if let Some(gems) = a["gems"].as_array() {
        new["apotheosis"]["gems"] = Value::Array(gems.iter().take(sockets as usize).cloned().collect());
    }
    let mut list: Vec<Value> = modifiers.or(core::defaults_of(item).modifiers.as_ref()).map(core::modifier_list).unwrap_or_default();
    list.retain(|m| !m["id"].as_str().is_some_and(|id| id.starts_with("apotheosis:")));
    list.extend(core::own_modifiers(item, &new));
    let lore = lore_for(&new);
    Some(Reforged { name: patch.name, custom: new, modifiers: Value::Array(list), lore, levels: *levels, material_cost: *cost })
}

/// Gem cutting at the anvil: a gem and gem dust (1, 3, 5, 7 or 9 by purity) make it one purity better.
pub fn cut_gem(gem_item: &str, gem_custom: &Value, right: &str, count: i32) -> Option<(MadeGem, i32, i32)> {
    let gem = gem_for_item(gem_item)?;
    if right != GEM_DUST {
        return None;
    }
    let purity = gem_custom["apotheosis"]["purity"].as_str().and_then(purity_named).unwrap_or(0).max(gem.min_purity);
    if purity + 1 >= PURITIES.len() {
        return None;
    }
    let dust = 1 + 2 * purity as i32;
    if count < dust {
        return None;
    }
    Some((gem_stack(gem, purity + 1), 3 * (purity as i32 + 1), dust))
}

// ---- sigils -------------------------------------------------------------------------

pub const SIGIL_OF_SOCKETING: &str = "apotheosis:sigil_of_socketing";
pub const SIGIL_OF_WITHDRAWAL: &str = "apotheosis:sigil_of_withdrawal";
pub const SIGIL_OF_UNNAMING: &str = "apotheosis:sigil_of_unnaming";
/// Sockets a Sigil of Socketing adds up to (Apotheosis' `add_sockets` recipe).
const MAX_ADDED_SOCKETS: u64 = 2;

pub fn is_sigil(item: &str) -> bool {
    [SIGIL_OF_SOCKETING, SIGIL_OF_WITHDRAWAL, SIGIL_OF_UNNAMING].contains(&item)
}

/// An item after a sigil: its new data, and whether its affix name goes.
pub struct Sigiled {
    pub custom: Value,
    pub modifiers: Value,
    pub lore: Value,
    pub clear_name: bool,
}

/// A sigil on an item at the anvil: Socketing adds a socket (up to two), Withdrawal takes every gem out,
/// Unnaming takes the affix name off. None when it does nothing to this item.
pub fn sigil(item: &str, custom: Option<&Value>, modifiers: Option<&Value>, sigil: &str) -> Option<Sigiled> {
    core::category(item)?;
    let mut custom = custom.cloned().filter(|c| c.get("apotheosis").is_some()).unwrap_or_else(|| json!({"apotheosis": {}}));
    let a = custom["apotheosis"].clone();
    let mut clear_name = false;
    match sigil {
        SIGIL_OF_SOCKETING => {
            let sockets = a["sockets"].as_u64().unwrap_or(0);
            if sockets >= MAX_ADDED_SOCKETS {
                return None;
            }
            custom["apotheosis"]["sockets"] = json!(sockets + 1);
        }
        SIGIL_OF_WITHDRAWAL => {
            if a["gems"].as_array().is_none_or(Vec::is_empty) {
                return None;
            }
            custom["apotheosis"].as_object_mut()?.remove("gems");
        }
        SIGIL_OF_UNNAMING => {
            if a["affixes"].as_object().is_none_or(Map::is_empty) {
                return None;
            }
            clear_name = true;
        }
        _ => return None,
    }
    let mut list: Vec<Value> = modifiers.or(core::defaults_of(item).modifiers.as_ref()).map(core::modifier_list).unwrap_or_default();
    list.retain(|m| !m["id"].as_str().is_some_and(|id| id.starts_with("apotheosis:")));
    list.extend(core::own_modifiers(item, &custom));
    let lore = lore_for(&custom);
    Some(Sigiled { custom, modifiers: Value::Array(list), lore, clear_name })
}

/// Gems a Sigil of Withdrawal took out, waiting to be given to the player once the anvil's take is done.
static WITHDRAWN: std::sync::Mutex<Vec<MadeGem>> = std::sync::Mutex::new(Vec::new());

/// At the anvil's take, before its inputs are used up: the gems of an item withdrawn with a sigil.
pub fn queue_withdrawn(left_item: &str, left_custom: Option<&Value>, right_item: &str) {
    if right_item != SIGIL_OF_WITHDRAWAL || core::category(left_item).is_none() {
        return;
    }
    let gems = left_custom.map(socketed_gems).unwrap_or_default();
    if let Ok(mut queue) = WITHDRAWN.lock() {
        queue.extend(gems);
    }
}

/// The withdrawn gems waiting for the player, emptying the queue.
pub fn take_withdrawn() -> Vec<MadeGem> {
    WITHDRAWN.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default()
}

/// The gems socketed in an item, as items, for a Sigil of Withdrawal to give back.
pub fn socketed_gems(custom: &Value) -> Vec<MadeGem> {
    custom["apotheosis"]["gems"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|g| {
            let gem = gem_named(g["gem"].as_str()?)?;
            Some(gem_stack(gem, g["purity"].as_str().and_then(purity_named).unwrap_or(0)))
        })
        .collect()
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
    fn salvaging_gives_materials_and_dust() {
        let custom = json!({"apotheosis": {"rarity": "apotheosis:epic", "affixes": {}}});
        let (material, n) = salvage("minecraft:diamond_sword", &custom).unwrap();
        assert_eq!(material, "apotheosis:arcane_sands");
        assert!((1..=4).contains(&n));
        assert_eq!(salvage("minecraft:diamond_sword", &custom), salvage("minecraft:diamond_sword", &custom), "same every time");
        let gem = gem_stack(gem_named("apotheosis:core/ballast").unwrap(), 5);
        let (dust, n) = salvage(&gem.item, &gem.components[0].1).unwrap();
        assert_eq!(dust, GEM_DUST);
        assert!((5..=10).contains(&n));
        assert!(salvage("minecraft:diamond_sword", &json!({})).is_none());
    }

    #[test]
    fn reforging_rerolls_and_keeps_gems() {
        let custom = json!({"apotheosis": {"rarity": "apotheosis:rare", "sockets": 2,
                            "affixes": {"apotheosis:melee/attribute/violent": 0.5},
                            "gems": [{"gem": "apotheosis:core/ballast", "purity": "flawed"}]}});
        assert!(reforge("minecraft:diamond_sword", &custom, None, "apotheosis:arcane_sands", 5).is_none(), "wrong material");
        assert!(reforge("minecraft:diamond_sword", &custom, None, "apotheosis:luminous_crystal_shard", 1).is_none(), "too few");
        let out = reforge("minecraft:diamond_sword", &custom, None, "apotheosis:luminous_crystal_shard", 2).unwrap();
        assert_eq!(out.levels, 15);
        assert_eq!(out.custom["apotheosis"]["rarity"], "apotheosis:rare");
        assert_eq!(out.custom["apotheosis"]["gems"][0]["gem"], "apotheosis:core/ballast");
        assert!(out.custom["apotheosis"]["sockets"].as_u64().unwrap() >= 2);
        let ids: Vec<&str> = out.modifiers.as_array().unwrap().iter().filter_map(|m| m["id"].as_str()).collect();
        assert!(ids.iter().any(|id| id.starts_with("apotheosis:gem/")), "{ids:?}");
        let again = reforge("minecraft:diamond_sword", &out.custom, None, "apotheosis:luminous_crystal_shard", 2).unwrap();
        assert_ne!(again.custom, out.custom, "a reforged item rerolls differently");
    }

    #[test]
    fn gems_are_cut_with_dust() {
        let gem = gem_stack(gem_named("apotheosis:core/ballast").unwrap(), 2);
        assert!(cut_gem(&gem.item, &gem.components[0].1, GEM_DUST, 4).is_none(), "flawed needs 5 dust");
        let (better, levels, dust) = cut_gem(&gem.item, &gem.components[0].1, GEM_DUST, 5).unwrap();
        assert_eq!((better.purity, levels, dust), ("normal", 9, 5));
        let perfect = gem_stack(gem_named("apotheosis:core/ballast").unwrap(), 5);
        assert!(cut_gem(&perfect.item, &perfect.components[0].1, GEM_DUST, 64).is_none());
    }

    #[test]
    fn sigils_add_sockets_take_gems_and_names() {
        let plain = sigil("minecraft:diamond_sword", None, None, SIGIL_OF_SOCKETING).unwrap();
        assert_eq!(plain.custom["apotheosis"]["sockets"], 1);
        let two = sigil("minecraft:diamond_sword", Some(&plain.custom), None, SIGIL_OF_SOCKETING).unwrap();
        assert!(sigil("minecraft:diamond_sword", Some(&two.custom), None, SIGIL_OF_SOCKETING).is_none(), "two at most");
        let gemmed = json!({"apotheosis": {"rarity": "apotheosis:rare", "sockets": 2,
                            "affixes": {"apotheosis:melee/attribute/violent": 0.5},
                            "gems": [{"gem": "apotheosis:core/ballast", "purity": "flawed"}]}});
        assert_eq!(socketed_gems(&gemmed).len(), 1);
        let out = sigil("minecraft:diamond_sword", Some(&gemmed), None, SIGIL_OF_WITHDRAWAL).unwrap();
        assert!(out.custom["apotheosis"].get("gems").is_none());
        assert!(!out.modifiers.as_array().unwrap().iter().any(|m| m["id"].as_str().unwrap().starts_with("apotheosis:gem/")));
        assert!(sigil("minecraft:diamond_sword", Some(&out.custom), None, SIGIL_OF_WITHDRAWAL).is_none(), "nothing to take");
        assert!(sigil("minecraft:diamond_sword", Some(&gemmed), None, SIGIL_OF_UNNAMING).unwrap().clear_name);
        assert!(sigil("minecraft:stick", None, None, SIGIL_OF_SOCKETING).is_none());
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
