// Apotheosis for Horizon, the game's side: injected into horizon-client's game.rs as `apotheosis_client`.
//
// Horizon applies and lists only an item kind's own attribute modifiers, not a stack's `attribute_modifiers`, so
// this adds the ones this mod puts on a stack (ids `apotheosis:...`, from affixes and gems) to the player's
// attributes and to the tooltip; and it socketes gems at the anvil.

use horizon_client::items::ItemStack;
use serde_json::Value;

/// A stack's component as JSON, from the components Horizon keeps as they were saved.
fn component(stack: &ItemStack, id: &str) -> Option<Value> {
    stack.data()?.other(id).map(horizon_client_core::scoreboard::tag_to_json)
}

/// This mod's modifiers on a stack: (attribute, id, amount, operation, slot group), worked out from its
/// `custom_data`, which reaches the window (its `attribute_modifiers` doesn't).
fn own_modifiers(stack: &ItemStack) -> Vec<(String, String, f64, String, String)> {
    let Some(custom) = component(stack, "minecraft:custom_data") else { return Vec::new() };
    horizon_world::apotheosis::own_modifiers(&stack.item, &custom)
        .iter()
        .map(|m| {
            (
                m["type"].as_str().unwrap_or_default().to_owned(),
                m["id"].as_str().unwrap_or_default().to_owned(),
                m["amount"].as_f64().unwrap_or(0.0),
                m["operation"].as_str().unwrap_or("add_value").to_owned(),
                m["slot"].as_str().unwrap_or("any").to_owned(),
            )
        })
        .collect()
}

/// At the end of `Game::apply_equipment`: the worn and held stacks' own modifiers, applied as the item kind's are
/// and recorded so the next call takes them off again.
pub(super) fn apply_equipment(game: &mut super::Game<'_>) {
    let slots = ["head", "chest", "legs", "feet"];
    let mut equipped: Vec<(Option<ItemStack>, &str)> = game.armor.iter().cloned().zip(slots).collect();
    equipped.push((game.inventory[game.selected].clone(), "mainhand"));
    equipped.push((game.offhand.clone(), "offhand"));
    let Some(player) = game.player.as_mut() else { return };
    let mut added = false;
    for (stack, slot) in equipped {
        let Some(stack) = stack else { continue };
        for (attribute, id, amount, operation, group) in own_modifiers(&stack) {
            if !super::equipment_modifier_applies(&group, slot) {
                continue;
            }
            if player.set_modifier_unclamped(&attribute, &id, amount, &operation, true).is_ok() {
                game.applied_modifiers.push((attribute, id));
                added = true;
            }
        }
    }
    if added {
        let _ = player.clamp_health_to_max();
    }
}

/// The tooltip's lines for this mod's modifiers, as the item kind's are drawn, without their slot headers.
pub(super) fn tooltip(stack: &ItemStack, lines: &mut Vec<(horizon_client::text::Text, u32)>) {
    let own = own_modifiers(stack);
    if own.is_empty() {
        return;
    }
    let translate = |key: &str| {
        let text = horizon_client::lang::get(key);
        (text != key).then_some(text)
    };
    let headers: Vec<String> = ["any", "mainhand", "offhand", "hand", "feet", "legs", "chest", "head", "armor", "body"]
        .iter()
        .map(|g| translate(&format!("item.modifiers.{g}")).unwrap_or_else(|| format!("item.modifiers.{g}")))
        .collect();
    for (line, color) in horizon_client::item_tooltip::attribute_lines(&own, None, &translate) {
        if line.is_empty() || headers.contains(&line) {
            continue;
        }
        lines.push((horizon_client::text::Text::legacy(&line), color));
    }
}

/// At the anvil: an item on the left and one of this mod's gems on the right give the item with the gem in its
/// first free socket, for a level; a gem that can't go in leaves no result.
pub(super) fn anvil(
    left: &ItemStack,
    right: Option<&ItemStack>,
    output: &mut Option<ItemStack>,
    xp_cost: &mut i32,
    material_cost: &mut i32,
) {
    let Some(right) = right else { return };
    let gems = horizon_world::apotheosis_gems::is_material(&right.item) || &*right.item == horizon_world::apotheosis_gems::GEM_DUST;
    if gems {
        craft(left, right, output, xp_cost, material_cost);
        return;
    }
    if !right.item.starts_with("apotheosis:gem_") {
        return;
    }
    let socketed = horizon_world::apotheosis_gems::socket(
        &left.item,
        component(left, "minecraft:custom_data").as_ref(),
        component(left, "minecraft:attribute_modifiers").as_ref(),
        &right.item,
        component(right, "minecraft:custom_data").as_ref(),
    );
    let Some(socketed) = socketed else {
        *output = None;
        *xp_cost = 0;
        return;
    };
    let mut out = left.clone();
    out.count = 1;
    let data = out.data_mut();
    data.set_loot_component("minecraft:custom_data", &socketed.custom);
    data.set_loot_component("minecraft:attribute_modifiers", &socketed.modifiers);
    data.set_loot_component("minecraft:lore", &socketed.lore);
    out.tidy();
    *output = Some(out);
    *xp_cost = 1;
    *material_cost = 1;
}

// ---- bosses and elites --------------------------------------------------------

use horizon_world::nbt::{Compound, Tag};
use std::sync::atomic::{AtomicU64, Ordering};

/// When the last boss spawned (seconds since the epoch): Apotheosis waits `bossSpawnCooldown` (3600 ticks,
/// three minutes) between bosses.
static LAST_BOSS: AtomicU64 = AtomicU64::new(0);
const BOSS_COOLDOWN_SECS: u64 = 180;

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn compound(entries: Vec<(&str, Tag)>) -> Compound {
    entries.into_iter().map(|(k, v)| (k.into(), v)).collect()
}

/// A text component as plain words in the player's language, for the chat line.
fn render(text: &Value) -> String {
    match text {
        Value::String(s) => s.clone(),
        Value::Object(o) => {
            let mut out = if let Some(key) = o.get("translate").and_then(Value::as_str) {
                let pattern = horizon_client::lang::get_or(key, o.get("fallback").and_then(Value::as_str).unwrap_or(key));
                let args: Vec<String> = o.get("with").and_then(Value::as_array).into_iter().flatten().map(render).collect();
                let mut s = pattern;
                for (i, a) in args.iter().enumerate() {
                    s = s.replace(&format!("%{}$s", i + 1), a);
                }
                for a in &args {
                    s = s.replacen("%s", a, 1);
                }
                s
            } else {
                o.get("text").and_then(Value::as_str).unwrap_or_default().to_owned()
            };
            for e in o.get("extra").and_then(Value::as_array).into_iter().flatten() {
                out.push_str(&render(e));
            }
            out
        }
        _ => String::new(),
    }
}

/// The data a plan sets on a mob: its name, gear (the affix item and the gem always drop), effects and
/// persistence.
fn plan_nbt(plan: &horizon_world::apotheosis_bosses::Plan) -> Compound {
    use horizon_client_core::scoreboard::json_to_tag;
    let equipment: Compound = plan.equipment.iter().map(|(slot, item)| (slot.as_str().into(), json_to_tag(item))).collect();
    let drops: Compound = plan.always_drops.iter().map(|slot| (slot.as_str().into(), Tag::Float(2.0f32.to_bits()))).collect();
    let effects = plan
        .effects
        .iter()
        .map(|(id, amplifier, ticks)| {
            Tag::Compound(compound(vec![
                ("id", Tag::String(id.as_str().into())),
                ("amplifier", Tag::Byte(*amplifier as i8)),
                ("duration", Tag::Int(*ticks)),
                ("show_particles", Tag::Byte(1)),
            ]))
        })
        .collect();
    let mut nbt = compound(vec![
        ("CustomName", json_to_tag(&plan.name)),
        ("PersistenceRequired", Tag::Byte(1)),
        ("active_effects", Tag::List(effects)),
    ]);
    if !equipment.is_empty() {
        nbt.insert("equipment".into(), Tag::Compound(equipment));
        nbt.insert("drop_chances".into(), Tag::Compound(drops));
    }
    nbt
}

/// Sets mob `id`'s attribute bases from its own defaults and the plan's modifiers, and heals it to the new
/// maximum (Horizon loads a mob's attribute bases, not its modifiers).
fn apply_attributes(game: &mut super::Game<'_>, id: u32, plan: &horizon_world::apotheosis_bosses::Plan) -> bool {
    use horizon_world::apotheosis_bosses::final_value;
    let Some(m) = game.mobs.iter().find(|m| m.id == id && !m.removed) else { return false };
    let a = &m.attrs;
    let bases = [
        ("minecraft:max_health", f64::from(a.max_health)),
        ("minecraft:movement_speed", a.speed),
        ("minecraft:attack_damage", f64::from(a.attack_damage)),
        ("minecraft:attack_knockback", a.attack_knockback),
    ];
    let mut attributes = Vec::new();
    let mut health = f64::from(a.max_health);
    for (attribute, base) in bases {
        if !plan.attrs.iter().any(|(at, _, _)| at == attribute) {
            continue;
        }
        let value = final_value(base, attribute, &plan.attrs);
        if attribute == "minecraft:max_health" {
            health = value;
        }
        attributes.push(Tag::Compound(compound(vec![
            ("id", Tag::String(attribute.into())),
            ("base", Tag::Double(value.to_bits())),
        ])));
    }
    if attributes.is_empty() {
        return true;
    }
    let Some(entity) = game.native_entity(id) else { return false };
    let nbt = compound(vec![("attributes", Tag::List(attributes)), ("Health", Tag::Float((health as f32).to_bits()))]);
    game.native_entity_merge_nbt(entity, nbt)
}

/// Summons a boss at `pos` in `dimension` (in place of mob `replaces`, which goes), with its attributes set, and
/// says so in the chat: the chat line, or None when no boss could be made there.
pub(super) fn spawn_boss(
    game: &mut super::Game<'_>,
    dimension: &str,
    pos: [f64; 3],
    replaces: Option<u32>,
    rng: &mut horizon_world::apotheosis::Rng,
) -> Option<String> {
    use horizon_world::{apotheosis as core, apotheosis_bosses as bosses};
    let plan = bosses::plan_boss(dimension, core::tier(), 0.0, rng)?;
    let boss = game.native_summon(dimension, &plan.kind, pos, plan_nbt(&plan))?;
    apply_attributes(game, boss, &plan);
    if let Some(id) = replaces {
        game.native_remove_mob(dimension, id);
    }
    LAST_BOSS.store(now_secs(), Ordering::Relaxed);
    let rarity = plan.rarity.clone().unwrap_or_default();
    let rarity_name = horizon_client::lang::get_or(&format!("rarity.{rarity}"), rarity.rsplit(':').next().unwrap_or_default());
    let line = horizon_client::lang::get_or("info.apotheosis.boss_spawn", "A %s boss has appeared: %s at %s %s %s")
        .replacen("%s", &rarity_name, 1)
        .replacen("%s", &render(&plan.name), 1)
        .replacen("%s", &format!("{}", pos[0] as i64), 1)
        .replacen("%s", &format!("{}", pos[1] as i64), 1)
        .replacen("%s", &format!("{}", pos[2] as i64), 1);
    log::info!("Apotheosis: boss {} ({}) at {:?}: {}", plan.kind, rarity, pos, render(&plan.name));
    game.native_feedback(horizon_client::text::Text::legacy(&format!("§6{line}")));
    Some(line)
}

/// At the head of `native_mobs_joined`: for each monster the world just spawned, now and then a boss in its place
/// (an invader, at most one every three minutes) or the mob made an elite.
pub(super) fn mobs_joined(game: &mut super::Game<'_>, from: usize, natural: bool) {
    use horizon_world::{apotheosis as core, apotheosis_bosses as bosses, apotheosis_gems::is_monster};
    if !natural || game.remote.is_some() {
        return;
    }
    let dimension = game.dim.id().to_owned();
    let tier = core::tier();
    let joined: Vec<(u32, String, [f64; 3])> = game
        .mobs
        .get(from..)
        .unwrap_or_default()
        .iter()
        .filter(|m| !m.removed)
        .map(|m| (m.id, super::native_hooks::mob_kind(m), [m.position.x, m.position.y, m.position.z]))
        .collect();
    for (id, kind, pos) in joined {
        if !is_monster(&kind) {
            continue;
        }
        let seed = u64::from(id) ^ pos[0].to_bits().rotate_left(17) ^ pos[2].to_bits() ^ std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        let mut rng = core::Rng::new(seed);
        let ready = now_secs().saturating_sub(LAST_BOSS.load(Ordering::Relaxed)) >= BOSS_COOLDOWN_SECS;
        if ready && f64::from(rng.next_f32()) < bosses::spawn_chance(&dimension, tier) && spawn_boss(game, &dimension, pos, Some(id), &mut rng).is_some() {
            continue;
        }
        if let Some(plan) = bosses::plan_elite(&kind, &dimension, tier, 0.0, &mut rng) {
            if let Some(entity) = game.native_entity(id) {
                game.native_entity_merge_nbt(entity, plan_nbt(&plan));
                apply_attributes(game, id, &plan);
                log::info!("Apotheosis: elite {} at {:?}", kind, pos);
            }
        }
    }
}

/// The anvil's reforging (an affix item and its rarity's material) and gem cutting (a gem and gem dust).
fn craft(left: &ItemStack, right: &ItemStack, output: &mut Option<ItemStack>, xp_cost: &mut i32, material_cost: &mut i32) {
    use horizon_world::apotheosis_gems as g;
    let left_custom = component(left, "minecraft:custom_data").unwrap_or(Value::Null);
    let count = right.count as i32;
    if let Some((gem, levels, dust)) = g::cut_gem(&left.item, &left_custom, &right.item, count) {
        let mut out = ItemStack::new(&gem.item, 1);
        let data = out.data_mut();
        data.set_loot_component("minecraft:custom_name", &gem.name);
        for (id, value) in &gem.components {
            data.set_loot_component(id, value);
        }
        out.tidy();
        *output = Some(out);
        *xp_cost = levels;
        *material_cost = dust;
        return;
    }
    let modifiers = component(left, "minecraft:attribute_modifiers");
    let Some(r) = g::reforge(&left.item, &left_custom, modifiers.as_ref(), &right.item, count) else {
        *output = None;
        *xp_cost = 0;
        return;
    };
    let mut out = left.clone();
    out.count = 1;
    let data = out.data_mut();
    data.set_loot_component("minecraft:custom_name", r.name.as_ref().unwrap_or(&Value::Null));
    data.set_loot_component("minecraft:custom_data", &r.custom);
    data.set_loot_component("minecraft:attribute_modifiers", &r.modifiers);
    data.set_loot_component("minecraft:lore", &r.lore);
    out.tidy();
    *output = Some(out);
    *xp_cost = r.levels;
    *material_cost = r.material_cost;
}
