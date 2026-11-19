//! Gems and sockets. Gems drop from monsters a player kills and turn up in chests (Apotheosis' `gems` loot
//! modifiers), each at a purity rolled by the world tier; an affix item has sockets from its rarity's `socket` rule,
//! and an anvil puts a gem into a free socket, adding the gem's bonus for that kind of item. Only bonuses this port
//! carries out count: a gem drops only if it has one, and goes only into an item it has one for. Salvaging,
//! reforging, gem cutting and the seals are here too.

use crate::loot::{self as core, Rng};
use mod_api::serde_json::{Map, Value, json};
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
    typ: String,
    raw: Value,
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
    /// Whether this port applies the bonus: attributes the game has, or a ported effect.
    fn works(&self) -> bool {
        !self.mods.is_empty() || crate::effects::ported(&self.typ, &self.raw)
    }
    /// `GemBonus.supports`: the first modifier (or the bonus) has a value at that purity.
    fn supports(&self, purity: usize) -> bool {
        match self.mods.first() {
            Some(m) => m.values.contains_key(PURITIES[purity]),
            None => self.works() && self.raw["values"].get(PURITIES[purity]).is_some(),
        }
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
    /// Its item, `vastadventures:gem_ballast` (each gem is an item of its own here, with its own sprite).
    pub fn item(&self) -> String {
        format!("vastadventures:gem_{}", self.path().rsplit('/').next().unwrap_or_default())
    }
    /// Whether any of its bonuses works here.
    fn usable(&self) -> bool {
        self.bonuses.iter().any(Bonus::works)
    }
    /// The bonuses for an item's category that aren't attributes, as (type, the bonus's data).
    pub fn raw_bonuses_for(&self, category: &str) -> Vec<(&str, &Value)> {
        self.bonuses
            .iter()
            .filter(|b| b.fits(category) && b.mods.is_empty())
            .map(|b| (b.typ.as_str(), &b.raw))
            .collect()
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
    (attribute.starts_with("minecraft:") || crate::effects::apothic(attribute)).then(|| Modifier {
        attribute: attribute.to_owned(),
        operation: m["operation"].as_str().unwrap_or("add_value").to_owned(),
        values: m["values"].as_object().cloned().unwrap_or_default(),
    })
}

fn data() -> &'static GemData {
    static DATA: OnceLock<GemData> = OnceLock::new();
    DATA.get_or_init(|| {
        use crate::data as d;
        let gems: Value = mod_api::serde_json::from_str(d::GEMS).expect("Vast Adventures: embedded gems read");
        let purity: Value = mod_api::serde_json::from_str(d::PURITY_WEIGHTS).expect("Vast Adventures: embedded purities read");
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
                        let multi = b["type"] == "vastadventures:multi_attribute";
                        let mods = match b["type"].as_str() {
                            Some("vastadventures:attribute") => modifier(b).into_iter().collect(),
                            Some("vastadventures:multi_attribute") => b["modifiers"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(modifier)
                                .collect(),
                            _ => Vec::new(),
                        };
                        Bonus {
                            typ: b["type"].as_str().unwrap_or_default().to_owned(),
                            raw: b.clone(),
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

/// Every gem's item id.
pub fn gem_items() -> Vec<String> {
    data().gems.iter().map(Gem::item).collect()
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

/// A gem as an item: its id, name and components (its purity is in its `custom_data`).
pub struct MadeGem {
    pub item: String,
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
        ("minecraft:custom_data".to_owned(), json!({"vastadventures": {"gem": gem.id, "purity": PURITIES[purity]}})),
        ("minecraft:lore".to_owned(), gem_lore(gem, purity)),
    ];
    if purity == PURITIES.len() - 1 {
        components.push(("minecraft:enchantment_glint_override".to_owned(), json!(true)));
    }
    MadeGem { item: gem.item(), name: gem_name(gem, purity), components }
}

// ---- text ---------------------------------------------------------------------

fn gem_name(gem: &Gem, purity: usize) -> Value {
    let item = gem.item();
    let (ns, path) = item.split_once(':').unwrap_or(("vastadventures", &item));
    json!({
        "translate": "item.vastadventures.gem_name", "fallback": "%s %s",
        "with": [
            {"translate": format!("purity.vastadventures.{}", PURITIES[purity]), "fallback": PURITY_NAMES[purity]},
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
    if bonus.mods.is_empty() {
        let value = bonus.raw["values"].get(PURITIES[purity]).cloned().unwrap_or(Value::Null);
        return crate::effects::describe_raw(&bonus.typ, &bonus.raw, &value);
    }
    let mut parts = Vec::new();
    for (i, m) in bonus.mods.iter().enumerate() {
        let Some(v) = m.values.get(PURITIES[purity]).and_then(Value::as_f64) else { continue };
        if i > 0 {
            parts.push(json!({"text": " / "}));
        }
        if crate::effects::apothic(&m.attribute) {
            let (amount, key) = crate::effects::apothic_text(&m.attribute, v, &m.operation);
            parts.push(json!({"text": format!("{amount} ")}));
            parts.push(json!({"translate": key}));
            continue;
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
    json!({"translate": format!("gem_class.vastadventures.{key}"), "fallback": fallback})
}

/// A gem's tooltip: unique, then a line per kind of item it goes into with what it gives there.
fn gem_lore(gem: &Gem, purity: usize) -> Value {
    let mut lines = Vec::new();
    if gem.unique {
        lines.push(json!({"translate": "text.vastadventures.unique", "fallback": "Unique: one per item", "color": "#C73912", "italic": false}));
    }
    lines.push(json!({"translate": "text.vastadventures.when_socketed_in", "fallback": "When socketed in:", "color": "gold", "italic": false}));
    for bonus in gem.bonuses.iter().filter(|b| b.supports(purity)) {
        let mut extra = vec![class_name(&bonus.key), json!({"text": ": "})];
        extra.extend(bonus_effect(bonus, purity));
        lines.push(json!({"text": "• ", "color": "gold", "italic": false, "extra": extra}));
    }
    Value::Array(lines)
}

// ---- reading a stack's data ---------------------------------------------------------
//
// A stack's `custom_data` comes from the player (Creative sets any): every read here is kept to what this mod
// writes. Reading a `Value` by index never panics (a missing key or a wrong kind reads as null); writing by index
// panics unless the value is an object or null, so writes go through `as_object_mut`.

/// This mod's part of a `custom_data`, when it is an object.
fn own(custom: &Value) -> Option<&Map<String, Value>> {
    custom.get("vastadventures")?.as_object()
}

/// The sockets an item's data (its `vastadventures` object) gives it, at most `MAX_SOCKETS`.
fn socket_count(a: &Value) -> usize {
    a["sockets"].as_u64().map_or(0, |n| n.min(u64::from(core::MAX_SOCKETS))) as usize
}

/// The gems socketed in an item, from its `custom_data`: one per socket at most (gems past its sockets do nothing,
/// as in `SocketHelper.getGems`).
pub fn socketed(custom: &Value) -> impl Iterator<Item = &Value> {
    let a = &custom["vastadventures"];
    a["gems"].as_array().into_iter().flatten().take(socket_count(a))
}

/// An item's lore from its `custom_data`: the rarity, then one line per socket, the gem in it or empty.
pub fn lore_for(custom: &Value) -> Value {
    let a = &custom["vastadventures"];
    let mut lines = Vec::new();
    if let Some(line) = a["rarity"].as_str().and_then(core::rarity_line) {
        lines.push(line);
    }
    let sockets = socket_count(a);
    let gems: Vec<&Value> = socketed(custom).collect();
    for i in 0..sockets {
        let filled = gems.get(i).and_then(|g| {
            let gem = gem_named(g["gem"].as_str()?)?;
            let purity = purity_named(g["purity"].as_str()?)?;
            Some(gem_name(gem, purity))
        });
        lines.push(match filled {
            Some(name) => json!({"text": "◆ ", "color": SOCKET_COLOR, "italic": false, "extra": [name]}),
            None => json!({"text": "◇ ", "color": SOCKET_COLOR, "italic": false,
                           "extra": [{"translate": "socket.vastadventures.empty", "fallback": "Empty Socket"}]}),
        });
    }
    Value::Array(lines)
}

/// The modifiers an item's socketed gems give, from its `custom_data`: gem `i` adds its bonus for the item's kind
/// at its purity, with ids `vastadventures:gem/<path>_<slot>_<i><salt>`.
pub fn gem_modifiers(item: &str, custom: &Value) -> Vec<Value> {
    let Some(category) = core::category(item) else { return Vec::new() };
    let slot = core::slot_group(category);
    let mut out = Vec::new();
    for (index, g) in socketed(custom).enumerate() {
        let Some(gem) = g["gem"].as_str().and_then(gem_named) else { continue };
        let purity = g["purity"].as_str().and_then(purity_named).unwrap_or(0).max(gem.min_purity);
        let Some(bonus) = gem.bonus_for(category, purity) else { continue };
        for (i, m) in bonus.mods.iter().enumerate() {
            let Some(amount) = m.values.get(PURITIES[purity]).and_then(Value::as_f64) else { continue };
            let salt = if bonus.multi { i.to_string() } else { String::new() };
            out.push(json!({
                "type": m.attribute,
                "id": format!("vastadventures:gem/{}_{slot}_{index}{salt}", gem.path()),
                "amount": amount,
                "operation": m.operation,
                "slot": slot,
            }));
        }
    }
    out
}

// ---- socketing ----------------------------------------------------------------

/// An item with one more gem: its new `custom_data` and `lore`.
pub struct Socketed {
    pub custom: Value,
    pub lore: Value,
}

/// `SocketHelper.socketGem`, at the anvil: `item` (its `custom_data`) takes the gem
/// `gem_item` (its `custom_data`) into its first free socket, when it has one, the gem has a bonus that works for
/// this kind of item at its purity, and a unique gem isn't in it already.
pub fn socket(item: &str, custom: Option<&Value>, gem_item: &str, gem_custom: Option<&Value>) -> Option<Socketed> {
    let category = core::category(item)?;
    let gem = gem_for_item(gem_item)?;
    let stored = gem_custom.and_then(|c| c["vastadventures"]["purity"].as_str()).and_then(purity_named).unwrap_or(0);
    let purity = stored.max(gem.min_purity);
    let custom = custom?;
    let a = Value::Object(own(custom)?.clone());
    let sockets = socket_count(&a);
    let mut gems: Vec<Value> = socketed(custom).cloned().collect();
    if gems.len() >= sockets {
        return None;
    }
    if gem.unique && gems.iter().any(|g| g["gem"].as_str() == Some(&gem.id)) {
        return None;
    }
    gem.bonus_for(category, purity)?;
    gems.push(json!({"gem": gem.id, "purity": PURITIES[purity]}));
    let mut custom = custom.clone();
    custom.as_object_mut()?.get_mut("vastadventures")?.as_object_mut()?.insert("gems".to_owned(), Value::Array(gems));
    let lore = lore_for(&custom);
    Some(Socketed { custom, lore })
}

// ---- salvaging, reforging and gem cutting -------------------------------------------

/// The material each rarity salvages into and reforging costs, with (material, levels) per reforge.
const MATERIALS: [(&str, &str, i32, i32); 5] = [
    ("vastadventures:common", "vastadventures:rusted_scrap", 1, 2),
    ("vastadventures:uncommon", "vastadventures:faded_cloth", 2, 5),
    ("vastadventures:rare", "vastadventures:glow_shard", 2, 15),
    ("vastadventures:epic", "vastadventures:starsand", 2, 30),
    ("vastadventures:mythic", "vastadventures:titan_pearl", 3, 50),
];
/// The gem dust a gem salvages into, (min, max) by purity.
const DUST: [(i32, i32); 6] = [(1, 2), (1, 3), (2, 4), (3, 5), (4, 7), (5, 10)];
pub const GEM_DUST: &str = "vastadventures:gem_dust";

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
    let a = &custom["vastadventures"];
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
    pub lore: Value,
    pub levels: i32,
    pub material_cost: i32,
}

/// Reforging at the anvil (the reforging table's recipe): an affix item and its rarity's material, enough of it.
pub fn reforge(item: &str, custom: &Value, material: &str, count: i32) -> Option<Reforged> {
    let category = core::category(item)?;
    let a = &custom["vastadventures"];
    let rarity_id = a["rarity"].as_str()?;
    let (_, wanted, cost, levels) = MATERIALS.iter().find(|(r, _, _, _)| *r == rarity_id)?;
    if *wanted != material || count < *cost {
        return None;
    }
    let rarity = (0..5).find(|&i| core::rarity_id(i) == rarity_id)?;
    let mut rng = stack_rng(item, custom, "reforge");
    let rolled = core::roll(category, rarity, core::tier(), 0.0, &mut rng);
    let patch = core::patch(item, &rolled);
    let mut new = patch.custom;
    let sockets = socket_count(a).max(rolled.sockets as usize);
    let gems: Vec<Value> = socketed(custom).take(sockets).cloned().collect();
    let part = new.as_object_mut().and_then(|n| n.get_mut("vastadventures")).and_then(Value::as_object_mut)?;
    if sockets > 0 {
        part.insert("sockets".to_owned(), json!(sockets));
    }
    if !gems.is_empty() {
        part.insert("gems".to_owned(), Value::Array(gems));
    }
    let lore = lore_for(&new);
    Some(Reforged { name: patch.name, custom: new, lore, levels: *levels, material_cost: *cost })
}

/// Gem cutting at the anvil: a gem and gem dust (1, 3, 5, 7 or 9 by purity) make it one purity better.
pub fn cut_gem(gem_item: &str, gem_custom: &Value, right: &str, count: i32) -> Option<(MadeGem, i32, i32)> {
    let gem = gem_for_item(gem_item)?;
    if right != GEM_DUST {
        return None;
    }
    let purity = gem_custom["vastadventures"]["purity"].as_str().and_then(purity_named).unwrap_or(0).max(gem.min_purity);
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

pub const SEAL_OF_SOCKETS: &str = "vastadventures:seal_of_sockets";
pub const SEAL_OF_EXTRACTION: &str = "vastadventures:seal_of_extraction";
pub const SEAL_OF_ERASURE: &str = "vastadventures:seal_of_erasure";
/// Sockets a Sigil of Socketing adds up to (Apotheosis' `add_sockets` recipe).
const MAX_ADDED_SOCKETS: u64 = 2;

pub fn is_sigil(item: &str) -> bool {
    [SEAL_OF_SOCKETS, SEAL_OF_EXTRACTION, SEAL_OF_ERASURE].contains(&item)
}

/// An item after a sigil: its new data, and whether its affix name goes.
pub struct Sigiled {
    pub custom: Value,
    pub lore: Value,
    pub clear_name: bool,
}

/// A sigil on an item at the anvil: Socketing adds a socket (up to two), Withdrawal takes every gem out,
/// Unnaming takes the affix name off. None when it does nothing to this item, or when its data isn't an object
/// (or its `vastadventures` part, when it has one).
pub fn sigil(item: &str, custom: Option<&Value>, sigil: &str) -> Option<Sigiled> {
    core::category(item)?;
    let mut custom = match custom {
        Some(Value::Object(c)) => c.clone(),
        Some(_) => return None,
        None => Map::new(),
    };
    let a = match custom.remove("vastadventures") {
        Some(Value::Object(a)) => a,
        None => Map::new(),
        Some(_) => return None,
    };
    let mut part = a.clone();
    let a = Value::Object(a);
    let mut clear_name = false;
    match sigil {
        SEAL_OF_SOCKETS => {
            let sockets = socket_count(&a) as u64;
            if sockets >= MAX_ADDED_SOCKETS {
                return None;
            }
            part.insert("sockets".to_owned(), json!(sockets + 1));
        }
        SEAL_OF_EXTRACTION => {
            if a["gems"].as_array().is_none_or(Vec::is_empty) {
                return None;
            }
            part.remove("gems");
        }
        SEAL_OF_ERASURE => {
            if a["affixes"].as_object().is_none_or(Map::is_empty) {
                return None;
            }
            clear_name = true;
        }
        _ => return None,
    }
    custom.insert("vastadventures".to_owned(), Value::Object(part));
    let custom = Value::Object(custom);
    let lore = lore_for(&custom);
    Some(Sigiled { custom, lore, clear_name })
}

/// The gems socketed in an item, as items, for a Sigil of Withdrawal to give back.
pub fn socketed_gems(custom: &Value) -> Vec<MadeGem> {
    socketed(custom)
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
            "entries": [{"type": "minecraft:item", "name": "minecraft:stick", "modifier": [{"type": "vastadventures:random_gem"}]}],
        })
    };
    if path.starts_with("entities/") {
        pools.push(gem_pool(vec![
            json!({"type": "minecraft:killed_by_player"}),
            json!({"type": "vastadventures:is_monster"}),
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
            json!({"type": "vastadventures:has_player"}),
            json!({"type": "minecraft:random_chance", "chance": chance}),
        ]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sword_with_sockets(n: u64) -> Value {
        json!({"vastadventures": {"rarity": "vastadventures:rare", "affixes": {}, "sockets": n}})
    }

    #[test]
    fn gems_load() {
        let d = data();
        assert_eq!(d.gems.len(), 21);
        assert!(d.gems.iter().filter(|g| g.usable()).count() >= 14);
        let ballast = gem_named("vastadventures:core/ballast").unwrap();
        assert_eq!(ballast.item(), "vastadventures:gem_ballast");
        assert!(gem_for_item("vastadventures:gem_ballast").is_some());
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
            assert!(random_purity(0, 0.0, &mut rng) <= 2, "calm rolls cracked to flawed");
            assert!(random_purity(4, 0.0, &mut rng) >= 2, "apex rolls flawed or better");
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
            assert!(purity_named(gem.components[0].1["vastadventures"]["purity"].as_str().unwrap()).unwrap() >= g.min_purity);
        }
    }

    #[test]
    fn a_gem_goes_into_a_free_socket() {
        let ballast = gem_named("vastadventures:core/ballast").unwrap();
        let made = gem_stack(ballast, 3);
        let gem_custom = &made.components[0].1;
        let out = socket("minecraft:diamond_sword", Some(&sword_with_sockets(2)), &made.item, Some(gem_custom)).unwrap();
        let mods = core::own_modifiers("minecraft:diamond_sword", &out.custom);
        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0]["id"], "vastadventures:gem/core/ballast_mainhand_0");
        assert_eq!(mods[0]["amount"], 5.0);
        assert_eq!(out.custom["vastadventures"]["gems"][0]["purity"], "normal");
        assert_eq!(out.lore.as_array().unwrap().len(), 3, "rarity, the gem, one empty socket");
        // the second socket takes another, the third is refused
        let again = socket("minecraft:diamond_sword", Some(&out.custom), &made.item, Some(gem_custom)).unwrap();
        assert!(socket("minecraft:diamond_sword", Some(&again.custom), &made.item, Some(gem_custom)).is_none());
    }

    #[test]
    fn socketing_keeps_the_affixes() {
        let custom = json!({"vastadventures": {"rarity": "vastadventures:rare", "sockets": 1,
                            "affixes": {"vastadventures:melee/attribute/violent": 0.5}}});
        let ballast = gem_named("vastadventures:core/ballast").unwrap();
        let made = gem_stack(ballast, 2);
        let out = socket("minecraft:diamond_sword", Some(&custom), &made.item, Some(&made.components[0].1)).unwrap();
        let mods = core::own_modifiers("minecraft:diamond_sword", &out.custom);
        let ids: Vec<&str> = mods.iter().filter_map(|m| m["id"].as_str()).collect();
        assert!(ids.contains(&"vastadventures:affix/melee/attribute/violent"), "{ids:?}");
        assert!(ids.contains(&"vastadventures:gem/core/ballast_mainhand_0"), "{ids:?}");
        assert_eq!(core::own_modifiers("minecraft:diamond_sword", &out.custom).len(), 2);
    }

    #[test]
    fn unique_gems_and_wrong_items_are_refused() {
        let samurai = gem_named("vastadventures:core/samurai").unwrap();
        assert!(samurai.unique);
        let made = gem_stack(samurai, 3);
        let gem_custom = &made.components[0].1;
        // samurai's leggings bonus is toughness and speed
        let first = socket("minecraft:iron_leggings", Some(&sword_with_sockets(2)), &made.item, Some(gem_custom)).unwrap();
        assert!(socket("minecraft:iron_leggings", Some(&first.custom), &made.item, Some(gem_custom)).is_none());
        // no sockets, no gem
        assert!(socket("minecraft:diamond_sword", None, &made.item, Some(gem_custom)).is_none());
        // not a gem
        assert!(socket("minecraft:diamond_sword", Some(&sword_with_sockets(1)), "minecraft:diamond", None).is_none());
    }

    #[test]
    fn salvaging_gives_materials_and_dust() {
        let custom = json!({"vastadventures": {"rarity": "vastadventures:epic", "affixes": {}}});
        let (material, n) = salvage("minecraft:diamond_sword", &custom).unwrap();
        assert_eq!(material, "vastadventures:starsand");
        assert!((1..=4).contains(&n));
        assert_eq!(salvage("minecraft:diamond_sword", &custom), salvage("minecraft:diamond_sword", &custom), "same every time");
        let gem = gem_stack(gem_named("vastadventures:core/ballast").unwrap(), 5);
        let (dust, n) = salvage(&gem.item, &gem.components[0].1).unwrap();
        assert_eq!(dust, GEM_DUST);
        assert!((5..=10).contains(&n));
        assert!(salvage("minecraft:diamond_sword", &json!({})).is_none());
    }

    #[test]
    fn reforging_rerolls_and_keeps_gems() {
        let custom = json!({"vastadventures": {"rarity": "vastadventures:rare", "sockets": 2,
                            "affixes": {"vastadventures:melee/attribute/violent": 0.5},
                            "gems": [{"gem": "vastadventures:core/ballast", "purity": "flawed"}]}});
        assert!(reforge("minecraft:diamond_sword", &custom, "vastadventures:starsand", 5).is_none(), "wrong material");
        assert!(reforge("minecraft:diamond_sword", &custom, "vastadventures:glow_shard", 1).is_none(), "too few");
        let out = reforge("minecraft:diamond_sword", &custom, "vastadventures:glow_shard", 2).unwrap();
        assert_eq!(out.levels, 15);
        assert_eq!(out.custom["vastadventures"]["rarity"], "vastadventures:rare");
        assert_eq!(out.custom["vastadventures"]["gems"][0]["gem"], "vastadventures:core/ballast");
        assert!(out.custom["vastadventures"]["sockets"].as_u64().unwrap() >= 2);
        let mods = core::own_modifiers("minecraft:diamond_sword", &out.custom);
        assert!(mods.iter().any(|m| m["id"].as_str().unwrap().starts_with("vastadventures:gem/")), "{mods:?}");
        let again = reforge("minecraft:diamond_sword", &out.custom, "vastadventures:glow_shard", 2).unwrap();
        assert_ne!(again.custom, out.custom, "a reforged item rerolls differently");
    }

    #[test]
    fn gems_are_cut_with_dust() {
        let gem = gem_stack(gem_named("vastadventures:core/ballast").unwrap(), 2);
        assert!(cut_gem(&gem.item, &gem.components[0].1, GEM_DUST, 4).is_none(), "flawed needs 5 dust");
        let (better, levels, dust) = cut_gem(&gem.item, &gem.components[0].1, GEM_DUST, 5).unwrap();
        assert_eq!((better.components[0].1["vastadventures"]["purity"].as_str(), levels, dust), (Some("normal"), 9, 5));
        let perfect = gem_stack(gem_named("vastadventures:core/ballast").unwrap(), 5);
        assert!(cut_gem(&perfect.item, &perfect.components[0].1, GEM_DUST, 64).is_none());
    }

    #[test]
    fn sigils_add_sockets_take_gems_and_names() {
        let plain = sigil("minecraft:diamond_sword", None, SEAL_OF_SOCKETS).unwrap();
        assert_eq!(plain.custom["vastadventures"]["sockets"], 1);
        let two = sigil("minecraft:diamond_sword", Some(&plain.custom), SEAL_OF_SOCKETS).unwrap();
        assert!(sigil("minecraft:diamond_sword", Some(&two.custom), SEAL_OF_SOCKETS).is_none(), "two at most");
        let gemmed = json!({"vastadventures": {"rarity": "vastadventures:rare", "sockets": 2,
                            "affixes": {"vastadventures:melee/attribute/violent": 0.5},
                            "gems": [{"gem": "vastadventures:core/ballast", "purity": "flawed"}]}});
        assert_eq!(socketed_gems(&gemmed).len(), 1);
        let out = sigil("minecraft:diamond_sword", Some(&gemmed), SEAL_OF_EXTRACTION).unwrap();
        assert!(out.custom["vastadventures"].get("gems").is_none());
        assert!(!core::own_modifiers("minecraft:diamond_sword", &out.custom).iter().any(|m| m["id"].as_str().unwrap().starts_with("vastadventures:gem/")));
        assert!(sigil("minecraft:diamond_sword", Some(&out.custom), SEAL_OF_EXTRACTION).is_none(), "nothing to take");
        assert!(sigil("minecraft:diamond_sword", Some(&gemmed), SEAL_OF_ERASURE).unwrap().clear_name);
        assert!(sigil("minecraft:stick", None, SEAL_OF_SOCKETS).is_none());
    }

    #[test]
    fn socket_counts_are_kept_to_the_most_an_item_has() {
        let max = core::MAX_SOCKETS as usize;
        let huge = json!({"vastadventures": {"rarity": "vastadventures:rare", "affixes": {}, "sockets": u64::MAX}});
        assert_eq!(lore_for(&huge).as_array().unwrap().len(), 1 + max, "the rarity, then 16 sockets");
        let gem = json!({"gem": "vastadventures:core/ballast", "purity": "flawed"});
        let many = json!({"vastadventures": {"rarity": "vastadventures:rare", "affixes": {}, "sockets": 1_000_000,
                          "gems": vec![gem; 100]}});
        assert_eq!(socketed(&many).count(), max);
        assert_eq!(socketed_gems(&many).len(), max, "a Seal of Extraction gives back only socketed gems");
        assert_eq!(gem_modifiers("minecraft:diamond_sword", &many).len(), max);
        let out = reforge("minecraft:diamond_sword", &many, "vastadventures:glow_shard", 2).unwrap();
        assert_eq!(out.custom["vastadventures"]["sockets"], json!(max));
        assert_eq!(out.custom["vastadventures"]["gems"].as_array().unwrap().len(), max);
        // gems past the sockets do nothing
        let none = json!({"vastadventures": {"gems": [{"gem": "vastadventures:core/ballast", "purity": "flawed"}]}});
        assert!(gem_modifiers("minecraft:diamond_sword", &none).is_empty());
        assert!(sigil("minecraft:diamond_sword", Some(&huge), SEAL_OF_SOCKETS).is_none());
    }

    #[test]
    fn data_of_any_shape_does_not_panic() {
        let ballast = gem_stack(gem_named("vastadventures:core/ballast").unwrap(), 3);
        let shapes = [json!("x"), json!(5), json!(-1), json!(1.5), json!(true), json!([1, 2]), json!(null), json!({})];
        for shape in &shapes {
            for custom in [json!({"vastadventures": shape}), shape.clone(),
                           json!({"vastadventures": {"sockets": shape, "gems": shape, "affixes": shape, "rarity": shape}})] {
                for seal in [SEAL_OF_SOCKETS, SEAL_OF_EXTRACTION, SEAL_OF_ERASURE] {
                    let _ = sigil("minecraft:diamond_sword", Some(&custom), seal);
                }
                let _ = socket("minecraft:diamond_sword", Some(&custom), &ballast.item, Some(&custom));
                let _ = salvage("minecraft:diamond_sword", &custom);
                let _ = salvage(&ballast.item, &custom);
                let _ = reforge("minecraft:diamond_sword", &custom, "vastadventures:glow_shard", 64);
                let _ = cut_gem(&ballast.item, &custom, GEM_DUST, 64);
                let _ = lore_for(&custom);
                let _ = socketed_gems(&custom);
                let _ = core::own_modifiers("minecraft:diamond_sword", &custom);
                let _ = crate::effects::effects("minecraft:diamond_sword", &custom);
            }
        }
        // A seal on data that isn't this mod's object leaves it be; other mods' data stays with a new socket.
        assert!(sigil("minecraft:diamond_sword", Some(&json!({"vastadventures": "x"})), SEAL_OF_SOCKETS).is_none());
        assert!(sigil("minecraft:diamond_sword", Some(&json!([1])), SEAL_OF_SOCKETS).is_none());
        let other = sigil("minecraft:diamond_sword", Some(&json!({"othermod": 1})), SEAL_OF_SOCKETS).unwrap();
        assert_eq!(other.custom, json!({"othermod": 1, "vastadventures": {"sockets": 1}}));
        // An affix level out of its range counts as the range's end.
        let wild = json!({"vastadventures": {"rarity": "vastadventures:rare", "affixes": {"vastadventures:melee/attribute/violent": 1e30}}});
        let top = json!({"vastadventures": {"rarity": "vastadventures:rare", "affixes": {"vastadventures:melee/attribute/violent": 1.0}}});
        assert_eq!(core::own_modifiers("minecraft:diamond_sword", &wild), core::own_modifiers("minecraft:diamond_sword", &top));
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
