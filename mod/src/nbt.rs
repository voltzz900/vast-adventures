//! The bridge between the mod's JSON (the pure logic's `custom_data`, lore and names) and the game's values: NBT
//! tags, text components, item stacks, and item ids for the hooks that hand over only an `ItemKey`.

use mod_api::serde_json::{Map, Number, Value};
use mod_api::text::TextContent;
use mod_api::{Component, Compound, Host, ItemKey, ItemStack, Tag, Text};
use std::collections::HashMap;
use std::sync::RwLock;

/// An NBT compound key.
pub fn key<K: for<'a> From<&'a str>>(s: &str) -> K {
    K::from(s)
}

/// JSON as NBT, as the game reads a component's JSON: booleans are bytes, whole numbers ints (longs past an int),
/// other numbers doubles; `null` inside a list or object is left out.
pub fn json_to_tag(value: &Value) -> Tag {
    match value {
        Value::Null => Tag::End,
        Value::Bool(b) => Tag::Byte(i8::from(*b)),
        Value::Number(n) => match n.as_i64() {
            Some(i) => i32::try_from(i).map_or(Tag::Long(i), Tag::Int),
            None => Tag::Double(n.as_f64().unwrap_or(0.0).to_bits()),
        },
        Value::String(s) => Tag::String(key(s)),
        Value::Array(list) => Tag::List(list.iter().filter(|v| !v.is_null()).map(json_to_tag).collect()),
        Value::Object(map) => Tag::Compound(json_to_compound(map)),
    }
}

pub fn json_to_compound(map: &Map<String, Value>) -> Compound {
    map.iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (key(k), json_to_tag(v))).collect()
}

/// NBT as JSON: numbers stay numbers (bytes too), strings strings.
pub fn tag_to_json(tag: &Tag) -> Value {
    let float = |f: f64| Number::from_f64(f).map_or(Value::Null, Value::Number);
    match tag {
        Tag::End => Value::Null,
        Tag::Byte(v) => Value::from(*v),
        Tag::Short(v) => Value::from(*v),
        Tag::Int(v) => Value::from(*v),
        Tag::Long(v) => Value::from(*v),
        Tag::Float(bits) => float(f64::from(f32::from_bits(*bits))),
        Tag::Double(bits) => float(f64::from_bits(*bits)),
        Tag::String(s) => Value::String(s.to_string_lossy()),
        Tag::List(list) => Value::Array(list.iter().map(tag_to_json).collect()),
        Tag::Compound(c) => Value::Object(c.iter().map(|(k, v)| (k.to_string_lossy(), tag_to_json(v))).collect()),
        Tag::ByteArray(v) => Value::Array(v.iter().map(|b| Value::from(*b)).collect()),
        Tag::IntArray(v) => Value::Array(v.iter().map(|i| Value::from(*i)).collect()),
        Tag::LongArray(v) => Value::Array(v.iter().map(|i| Value::from(*i)).collect()),
    }
}

/// A JSON text component (the keys the pure logic writes: `text`, `translate`, `fallback`, `with`, `color`, `italic`,
/// `extra`) as a game text.
pub fn text(value: &Value) -> Text {
    let mut out = match value {
        Value::String(s) => return Text::literal(s.clone()),
        Value::Object(o) => match o.get("translate").and_then(Value::as_str) {
            Some(key) => Text::new(TextContent::Translatable {
                key: key.to_owned(),
                fallback: o.get("fallback").and_then(Value::as_str).map(str::to_owned),
                arguments: o.get("with").and_then(Value::as_array).into_iter().flatten().map(text).collect(),
            }),
            None => Text::literal(o.get("text").and_then(Value::as_str).unwrap_or_default()),
        },
        other => return Text::literal(other.to_string()),
    };
    out.style.color = value.get("color").and_then(Value::as_str).map(str::to_owned);
    out.style.italic = value.get("italic").and_then(Value::as_bool);
    for child in value.get("extra").and_then(Value::as_array).into_iter().flatten() {
        out = out.append(text(child));
    }
    out
}

/// A component by its id, its value as raw NBT.
pub struct Raw<'a>(pub &'a str);

impl Component for Raw<'_> {
    type Value = Tag;
    fn id(&self) -> &str {
        self.0
    }
    fn decode(&self, value: &Tag) -> Option<Tag> {
        Some(value.clone())
    }
    fn encode(&self, value: Tag) -> Tag {
        value
    }
}

/// A stack's component as JSON.
pub fn component(stack: &ItemStack, id: &str) -> Option<Value> {
    stack.get(&Raw(id)).map(|t| tag_to_json(&t))
}

/// A stack's `custom_data`, when it carries this mod's data.
pub fn custom_data(stack: &ItemStack) -> Option<Value> {
    component(stack, "minecraft:custom_data").filter(|c| c.get("vastadventures").is_some())
}

/// Sets component `id` on `stack` to `value` (JSON); `null` takes it off.
pub fn set(stack: &mut ItemStack, id: &str, value: &Value) {
    if value.is_null() {
        stack.remove(&Raw(id));
    } else {
        let _ = stack.set(&Raw(id), json_to_tag(value));
    }
}

// ---- item ids ---------------------------------------------------------------------------

#[derive(Default)]
struct Items {
    ids: HashMap<ItemKey, String>,
    keys: HashMap<String, ItemKey>,
}

static ITEMS: RwLock<Option<Items>> = RwLock::new(None);

/// Learns every item's id, once a host can tell them (`client_setup`, `server_about_to_start`, then each level tick
/// until it has): the loot, attribute and enchantment hooks have only the key.
pub fn learn_items(host: &dyn Host) {
    if ITEMS.read().is_ok_and(|i| i.is_some()) {
        return;
    }
    let keys = host.item_keys();
    if keys.is_empty() {
        return;
    }
    let mut items = Items::default();
    for k in keys {
        let id = k.id(host);
        if !id.is_empty() {
            items.keys.insert(id.clone(), k);
            items.ids.insert(k, id);
        }
    }
    if let Ok(mut slot) = ITEMS.write() {
        *slot = Some(items);
    }
}

/// An item's id, by its key (empty before the ids are known).
pub fn item_id(k: ItemKey) -> String {
    ITEMS.read().ok().and_then(|i| i.as_ref()?.ids.get(&k).cloned()).unwrap_or_default()
}

/// An item's key, by its id.
pub fn item_key(id: &str) -> Option<ItemKey> {
    ITEMS.read().ok().and_then(|i| i.as_ref()?.keys.get(id).copied())
}

/// A stack's item id.
pub fn stack_id(stack: &ItemStack) -> String {
    item_id(stack.item())
}

/// An item as JSON `{id, count, components}` made a stack.
pub fn stack_from_json(item: &Value) -> Option<ItemStack> {
    let mut stack = ItemStack::new(item_key(item["id"].as_str()?)?, item["count"].as_u64().unwrap_or(1).max(1) as u32)?;
    if let Some(Value::Object(components)) = item.get("components") {
        for (id, value) in components {
            set(&mut stack, id, value);
        }
    }
    Some(stack)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mod_api::serde_json::json;

    #[test]
    fn json_round_trips_through_nbt() {
        let v = json!({"vastadventures": {"rarity": "vastadventures:rare", "sockets": 2,
                       "affixes": {"vastadventures:melee/attribute/violent": 0.5},
                       "gems": [{"gem": "vastadventures:core/ballast", "purity": "flawed"}]}});
        assert_eq!(tag_to_json(&json_to_tag(&v)), v);
        assert_eq!(json_to_tag(&json!(true)), Tag::Byte(1));
        assert_eq!(json_to_tag(&json!(5_000_000_000i64)), Tag::Long(5_000_000_000));
    }

    #[test]
    fn text_from_json() {
        let t = text(&json!({"translate": "a.b", "fallback": "%s!", "with": [{"text": "x"}], "color": "#FF0000",
                             "italic": false, "extra": ["y"]}));
        assert_eq!(t.style.color.as_deref(), Some("#FF0000"));
        assert_eq!(t.style.italic, Some(false));
        assert_eq!(t.children.len(), 1);
        match &t.content {
            TextContent::Translatable { key, fallback, arguments } => {
                assert_eq!((key.as_str(), fallback.as_deref(), arguments.len()), ("a.b", Some("%s!"), 1));
            }
            other => panic!("{other:?}"),
        }
    }
}
