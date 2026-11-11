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
    remember_worn(game);
    let slots = ["head", "chest", "legs", "feet"];
    let mut equipped: Vec<(Option<ItemStack>, &str)> = game.armor.iter().cloned().zip(slots).collect();
    equipped.push((game.inventory[game.selected].clone(), "mainhand"));
    equipped.push((game.offhand.clone(), "offhand"));
    let Some(player) = game.player.as_mut() else { return };
    let mut added = false;
    let mut apothic = Vec::new();
    for (stack, slot) in equipped {
        let Some(stack) = stack else { continue };
        for (attribute, id, amount, operation, group) in own_modifiers(&stack) {
            if !super::equipment_modifier_applies(&group, slot) {
                continue;
            }
            if fx::apothic(&attribute) {
                apothic.push((attribute, amount, operation));
                continue;
            }
            if player.set_modifier_unclamped(&attribute, &id, amount, &operation, true).is_ok() {
                game.applied_modifiers.push((attribute, id));
                added = true;
            }
        }
    }
    // The world tier's luck (Apotheosis' tier augments for players).
    let luck = [0.0, 0.2, 0.3, 0.5, 1.0][horizon_world::apotheosis::tier()];
    if luck > 0.0 && player.set_modifier_unclamped("minecraft:luck", "apotheosis:tier/luck", luck, "add_multiplied_total", true).is_ok() {
        game.applied_modifiers.push(("minecraft:luck".to_owned(), "apotheosis:tier/luck".to_owned()));
    }
    if added {
        let _ = player.clamp_health_to_max();
    }
    fx::set_stats(player.entity_id as i32, &apothic);
    let uuid = game.native_acting_id();
    if let Ok(mut u) = PLAYER_UUID.lock() {
        *u = Some(uuid);
    }
}

/// The tooltip's lines for this mod's modifiers, as the item kind's are drawn, without their slot headers.
pub(super) fn tooltip(stack: &ItemStack, lines: &mut Vec<(horizon_client::text::Text, u32)>) {
    effect_lines(stack, lines);
    let (apothic, own): (Vec<_>, Vec<_>) = own_modifiers(stack).into_iter().partition(|m| fx::apothic(&m.0));
    own_lines(&own, lines);
    for (attribute, _, amount, operation, _) in &apothic {
        let (shown, key) = fx::apothic_text(attribute, *amount, operation);
        let name = horizon_client::lang::get(&key);
        lines.push((horizon_client::text::Text::legacy(&format!("§9{shown} {name}")), u32::MAX));
    }
}

/// The game's attributes among this mod's modifiers, as the item kind's are drawn, without their slot headers.
fn own_lines(own: &[(String, String, f64, String, String)], lines: &mut Vec<(horizon_client::text::Text, u32)>) {
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
    for (line, color) in horizon_client::item_tooltip::attribute_lines(own, None, &translate) {
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
    if horizon_world::apotheosis_gems::is_sigil(&right.item) {
        sigil(left, right, output, xp_cost, material_cost);
        return;
    }
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

/// A sigil on an item at the anvil, for a level.
fn sigil(left: &ItemStack, right: &ItemStack, output: &mut Option<ItemStack>, xp_cost: &mut i32, material_cost: &mut i32) {
    let custom = component(left, "minecraft:custom_data");
    let modifiers = component(left, "minecraft:attribute_modifiers");
    let Some(s) = horizon_world::apotheosis_gems::sigil(&left.item, custom.as_ref(), modifiers.as_ref(), &right.item) else {
        *output = None;
        *xp_cost = 0;
        return;
    };
    let mut out = left.clone();
    out.count = 1;
    let data = out.data_mut();
    if s.clear_name {
        data.set_loot_component("minecraft:custom_name", &Value::Null);
    }
    data.set_loot_component("minecraft:custom_data", &s.custom);
    data.set_loot_component("minecraft:attribute_modifiers", &s.modifiers);
    data.set_loot_component("minecraft:lore", &s.lore);
    out.tidy();
    *output = Some(out);
    *xp_cost = 1;
    *material_cost = 1;
}

/// At the head of `Game::anvil_used`, once an anvil's result is taken: the gems a Sigil of Withdrawal took out pop
/// out on top of the anvil, for the player to pick up (summoned, so the server's world has them).
pub(super) fn anvil_used(game: &mut super::Game<'_>, pos: [i32; 3]) {
    use horizon_client_core::scoreboard::json_to_tag;
    let dimension = game.dim.id().to_owned();
    for gem in horizon_world::apotheosis_gems::take_withdrawn() {
        let mut components = serde_json::Map::new();
        components.insert("minecraft:custom_name".into(), gem.name.clone());
        for (id, value) in &gem.components {
            components.insert(id.clone(), value.clone());
        }
        let item = serde_json::json!({"id": gem.item, "count": 1, "components": components});
        let nbt = compound(vec![("Item", json_to_tag(&item)), ("PickupDelay", Tag::Short(10))]);
        let at = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 1.1, f64::from(pos[2]) + 0.5];
        if game.native_summon(&dimension, "minecraft:item", at, nbt).is_none() {
            log::warn!("Apotheosis: a withdrawn gem couldn't be dropped: {}", gem.item);
        }
    }
}

// ---- effects: affixes and gems beyond attributes -------------------------------------

use horizon_world::apotheosis_effects::{self as fx, Effect, Target};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::AtomicU32;
use std::time::{Duration, Instant};

/// The worn armour's effects (damage reduction, effects when hurt), worked out with the equipment.
static WORN: Mutex<Vec<Effect>> = Mutex::new(Vec::new());
/// Effects waiting for the next server tick: they come from hooks that can't reach the game.
static PENDING: Mutex<Vec<Pending>> = Mutex::new(Vec::new());
/// The mobs the player's arrows are hitting this tick (the damage that follows names no player).
static ARROW_HITS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
/// When each effect last fired, for its cooldown.
static COOLDOWNS: Mutex<Option<HashMap<String, Instant>>> = Mutex::new(None);

enum Pending {
    OnPlayer { effect: String, amplifier: i32, duration: i32 },
    OnMob { id: u32, effect: String, amplifier: i32, duration: i32 },
    Hurt { id: u32, amount: f32, damage_type: &'static str },
    Bolt { at: [f64; 3] },
    Heal { amount: f32 },
}

/// A stack's effects, from its `custom_data`.
fn stack_effects(stack: Option<&ItemStack>) -> Vec<Effect> {
    let Some(stack) = stack else { return Vec::new() };
    component(stack, "minecraft:custom_data").map_or_else(Vec::new, |c| fx::effects(&stack.item, &c))
}

/// Whether an effect from `source` may fire now (its cooldown in ticks has passed), marking it fired.
fn ready(source: &str, target: Target, cooldown: i32) -> bool {
    let Ok(mut map) = COOLDOWNS.lock() else { return true };
    let map = map.get_or_insert_with(HashMap::new);
    let key = format!("{source}/{target:?}");
    let now = Instant::now();
    if map.get(&key).is_some_and(|t| now.duration_since(*t) < Duration::from_millis(cooldown.max(0) as u64 * 50)) {
        return false;
    }
    map.insert(key, now);
    true
}

fn effect_on_player(game: &mut super::Game<'_>, effect: &str, amplifier: i32, duration: i32) {
    let player = player_uuid(game);
    let effect = horizon_client::mod_api::NativeEffect {
        id: effect.to_owned(),
        amplifier: amplifier.clamp(0, 255) as u8,
        duration,
        ambient: false,
        particles: true,
    };
    game.native_add_effect(player, &effect);
}

fn effect_on_mob(game: &mut super::Game<'_>, id: u32, effect: &str, amplifier: i32, duration: i32) {
    let Some(entity) = game.native_entity(id) else { return };
    let nbt = compound(vec![(
        "active_effects",
        Tag::List(vec![Tag::Compound(compound(vec![
            ("id", Tag::String(effect.into())),
            ("amplifier", Tag::Byte(amplifier.clamp(0, 127) as i8)),
            ("duration", Tag::Int(duration)),
            ("show_particles", Tag::Byte(1)),
        ]))]),
    )]);
    let applied = game.native_entity_merge_nbt(entity, nbt);
    log::debug!("Apotheosis: {effect} on mob {id}: {applied}");
}

fn queue_on_mob(id: u32, effect: &str, amplifier: i32, duration: i32) {
    if let Ok(mut pending) = PENDING.lock() {
        pending.push(Pending::OnMob { id, effect: effect.to_owned(), amplifier, duration });
    }
}

/// The monsters within `radius` of `at` in the world, but `except`.
fn monsters_near(game: &super::Game<'_>, at: [f64; 3], radius: f64, except: u32) -> Vec<u32> {
    game.mobs
        .iter()
        .filter(|m| !m.removed && m.id != except)
        .filter(|m| horizon_world::apotheosis_gems::is_monster(&super::native_hooks::mob_kind(m)))
        .filter(|m| {
            let d = [m.position.x - at[0], m.position.y - at[1], m.position.z - at[2]];
            d[0] * d[0] + d[1] * d[1] + d[2] * d[2] <= radius * radius
        })
        .map(|m| m.id)
        .collect()
}

/// Remembers the worn armour's effects, with the equipment (`apply_equipment`).
pub(super) fn remember_worn(game: &super::Game<'_>) {
    let mut worn = Vec::new();
    for piece in game.armor.iter() {
        worn.extend(stack_effects(piece.as_ref()));
    }
    if let Ok(mut w) = WORN.lock() {
        *w = worn;
    }
}

/// At the head of `native_living_incoming_damage_v2`, the player's melee hit on mob `entity`: the weapon's
/// effects. Executing changes the damage; the rest wait for the next tick, as the game is mid-hurt here.
pub(super) fn melee(game: &super::Game<'_>, entity: i32, weapon: Option<&ItemStack>, damage: &mut f32) {
    let Some(mob) = game.mobs.iter().find(|m| i64::from(m.id) == i64::from(entity) && !m.removed) else { return };
    apothic_melee(mob, damage);
    let effects = stack_effects(weapon);
    if effects.is_empty() {
        return;
    }
    let id = mob.id;
    let victim = [mob.position.x, mob.position.y, mob.position.z];
    let mut amount = *damage;
    let mut queued = Vec::new();
    for effect in &effects {
        match effect {
            Effect::MobEffect { source, target: Target::AttackTarget, effect, duration, amplifier, cooldown } if ready(source, Target::AttackTarget, *cooldown) => {
                queued.push(Pending::OnMob { id, effect: effect.clone(), amplifier: *amplifier, duration: *duration });
            }
            Effect::MobEffect { source, target: Target::AttackSelf, effect, duration, amplifier, cooldown } if ready(source, Target::AttackSelf, *cooldown) => {
                queued.push(Pending::OnPlayer { effect: effect.clone(), amplifier: *amplifier, duration: *duration });
            }
            Effect::Executing { threshold } => {
                let max = horizon_client::mobs::finalize::max_health(mob);
                if max > 0.0 && f64::from((mob.health - amount) / max) < *threshold {
                    amount = amount.max(mob.health + 1.0);
                }
            }
            Effect::Thunderstruck { damage } => {
                for other in monsters_near(game, victim, 4.0, id) {
                    queued.push(Pending::Hurt { id: other, amount: *damage as f32, damage_type: "minecraft:lightning_bolt" });
                }
                queued.push(Pending::Bolt { at: victim });
            }
            Effect::Cleaving { chance, targets } => {
                if fx::roll() < *chance {
                    for other in monsters_near(game, victim, 3.0, id).into_iter().take(*targets as usize) {
                        queued.push(Pending::Hurt { id: other, amount, damage_type: "minecraft:player_attack" });
                    }
                }
            }
            _ => {}
        }
    }
    if let Ok(mut pending) = PENDING.lock() {
        pending.extend(queued);
    }
    *damage = amount;
}

/// The attack strength of the player's last swing, from `critical_hit` (the swing resets it before the hit lands).
static SWING: AtomicU32 = AtomicU32::new(0x3f80_0000); // 1.0

/// At the head of `native_critical_hit`, inside `Player.attack`: the swing's strength, for the hit that follows.
pub(super) fn critical_hit(game: &super::Game<'_>) {
    SWING.store(game.attack_strength_scale().to_bits(), Ordering::Relaxed);
}

/// The Apothic attributes on the player's melee hit: damage from the target's health, fire and cold damage
/// (scaled by the swing), critical strikes, then armour pierce and shred.
fn apothic_melee(mob: &horizon_client::mobs::Mob, damage: &mut f32) {
    let scale = f64::from(f32::from_bits(SWING.swap(0x3f80_0000, Ordering::Relaxed)));
    let mut amount = f64::from(*damage);
    amount += f64::from(mob.health) * fx::stat("apothic_attributes:current_hp_damage") * scale;
    amount += (fx::stat("apothic_attributes:fire_damage") + fx::stat("apothic_attributes:cold_damage")) * scale;
    amount *= apothic_crit();
    let (armor, toughness) = horizon_client::mobs::armor::values(mob);
    amount = fx::pierced(
        amount,
        f64::from(armor),
        f64::from(toughness),
        fx::stat("apothic_attributes:armor_shred"),
        fx::stat("apothic_attributes:armor_pierce"),
    );
    *damage = amount as f32;
}

/// The player's critical strike multiplier for one hit (1 without one).
fn apothic_crit() -> f64 {
    fx::crit_multiplier(fx::stat("apothic_attributes:crit_chance"), fx::stat("apothic_attributes:crit_damage"), fx::roll)
}

/// At the head of `native_living_incoming_damage` (no game here): for the player, the worn armour's damage
/// reduction, and its effects when hurt, queued for the next tick.
pub(super) fn incoming(entity: i32, kind: &str, damage_type: &str, causing: i32, amount: &mut f32, canceled: &mut bool) {
    let player = fx::player();
    if kind != "minecraft:player" {
        // The player's arrows (marked as they hit): arrow and projectile damage, and critical strikes.
        let marked = ARROW_HITS.lock().ok().and_then(|mut hits| {
            let i = hits.iter().position(|&id| id as i32 == entity)?;
            Some(hits.swap_remove(i))
        });
        if marked.is_some() && matches!(damage_type, "minecraft:arrow" | "minecraft:trident") {
            let bonus = fx::stat("apothic_attributes:arrow_damage") * fx::stat("apothic_attributes:projectile_damage");
            *amount = (f64::from(*amount) * bonus * apothic_crit()) as f32;
        }
        return;
    }
    // Dodging a hit from another entity.
    let dodge = fx::stat("apothic_attributes:dodge_chance");
    if dodge > 0.0 && causing > 0 && player != Some(causing) {
        if fx::roll() < dodge {
            *canceled = true;
            return;
        }
    }
    let Ok(worn) = WORN.lock() else { return };
    if worn.is_empty() {
        return;
    }
    let cut = fx::reduction(&worn, damage_type);
    if cut > 0.0 {
        *amount *= (1.0 - cut) as f32;
    }
    let Ok(mut pending) = PENDING.lock() else { return };
    for effect in worn.iter() {
        match effect {
            Effect::MobEffect { source, target: Target::HurtSelf, effect, duration, amplifier, cooldown } if ready(source, Target::HurtSelf, *cooldown) => {
                pending.push(Pending::OnPlayer { effect: effect.clone(), amplifier: *amplifier, duration: *duration });
            }
            Effect::MobEffect { source, target: Target::HurtAttacker, effect, duration, amplifier, cooldown }
                if causing > 0 && ready(source, Target::HurtAttacker, *cooldown) =>
            {
                pending.push(Pending::OnMob { id: causing as u32, effect: effect.clone(), amplifier: *amplifier, duration: *duration });
            }
            _ => {}
        }
    }
}

/// At the head of `native_living_damage_post`: damage the player dealt in melee heals them by their life steal,
/// next tick.
pub(super) fn damage_dealt(kind: &str, damage_type: &str, amount: f32) {
    // A mob's damage names no player as its cause: the player's melee is its damage type.
    if kind == "minecraft:player" || damage_type != "minecraft:player_attack" || amount <= 0.0 {
        return;
    }
    let heal = f64::from(amount) * fx::stat("apothic_attributes:life_steal");
    if heal > 0.0 && let Ok(mut pending) = PENDING.lock() {
        pending.push(Pending::Heal { amount: heal as f32 });
    }
}

/// The player's uuid, recorded with the equipment: the server's game reaches them by it.
static PLAYER_UUID: Mutex<Option<u128>> = Mutex::new(None);

fn player_uuid(game: &mut super::Game<'_>) -> u128 {
    PLAYER_UUID.lock().ok().and_then(|u| *u).unwrap_or_else(|| game.native_acting_id())
}

/// Heals the player by `amount` times their healing received, in the server's game (where they may not be in
/// place: their health is read from their record and set through it).
fn heal_player(game: &mut super::Game<'_>, amount: f32) {
    let uuid = player_uuid(game);
    let Some(Tag::Compound(record)) = game.native_player_compound(uuid) else { return };
    let Some(Tag::Float(bits)) = record.get(&"Health".into()).cloned() else { return };
    let health = f32::from_bits(bits);
    if health <= 0.0 {
        return;
    }
    let amount = amount * fx::stat("apothic_attributes:healing_received") as f32;
    game.native_set_player_status(uuid, Some(health + amount), None, None);
}

/// At the head of `native_server_tick`: the effects queued by the hooks that can't reach the game.
pub(super) fn server_tick(game: &mut super::Game<'_>) {
    let pending = PENDING.lock().map(|mut p| std::mem::take(&mut *p)).unwrap_or_default();
    for p in pending {
        match p {
            Pending::OnPlayer { effect, amplifier, duration } => effect_on_player(game, &effect, amplifier, duration),
            Pending::OnMob { id, effect, amplifier, duration } => effect_on_mob(game, id, &effect, amplifier, duration),
            Pending::Hurt { id, amount, damage_type } => {
                let dimension = game.dim.id().to_owned();
                game.native_hurt_entity(&dimension, id, amount, damage_type, None);
            }
            Pending::Bolt { at } => game.spawn_bolt(at, true),
            Pending::Heal { amount } => heal_player(game, amount),
        }
    }
}

/// The bow (or crossbow) the player holds, main hand first.
fn held_bow(game: &super::Game<'_>) -> Option<ItemStack> {
    let is_bow = |s: &ItemStack| horizon_world::apotheosis::category(&s.item) == Some("bow");
    game.inventory[game.selected].clone().filter(is_bow).or_else(|| game.offhand.clone().filter(is_bow))
}

/// At the head of `native_projectile_impact`: the player's arrow hitting a mob, with the bow's arrow effects.
pub(super) fn projectile(game: &mut super::Game<'_>, kind: &str, owner: u32, entity: Option<u32>) {
    let Some(target) = entity else { return };
    // The player's own arrows (`owner` is the shooter's entity id).
    // This runs in the server's game, which may not hold the player in place: their id is the one the
    // equipment recorded.
    if fx::player() != Some(owner as i32) || !(kind.ends_with("arrow") || kind.ends_with("trident")) {
        return;
    }
    if let Ok(mut hits) = ARROW_HITS.lock() {
        if hits.len() > 64 {
            hits.clear();
        }
        hits.push(target);
    }
    let mut effects = BOW.lock().map(|b| b.clone()).unwrap_or_default();
    if effects.is_empty() {
        effects = stack_effects(held_bow(game).as_ref());
    }
    for effect in &effects {
        if let Effect::MobEffect { source, target: t, effect, duration, amplifier, cooldown } = effect {
            match t {
                Target::ArrowTarget if ready(source, *t, *cooldown) => queue_on_mob(target, effect, *amplifier, *duration),
                Target::ArrowSelf if ready(source, *t, *cooldown) => {
                    if let Ok(mut pending) = PENDING.lock() {
                        pending.push(Pending::OnPlayer { effect: effect.clone(), amplifier: *amplifier, duration: *duration });
                    }
                }
                _ => {}
            }
        }
    }
}

/// The effects of the bow the player last let go, for its arrows' hits.
static BOW: Mutex<Vec<Effect>> = Mutex::new(Vec::new());

/// At the head of `native_arrow_loose`: the bow's effects for its arrow, and the player's draw speed.
pub(super) fn arrow_loose(stack: &ItemStack, charge: &mut i32) {
    if let Ok(mut bow) = BOW.lock() {
        *bow = stack_effects(Some(stack));
    }
    let speed = fx::stat("apothic_attributes:draw_speed");
    *charge = (f64::from(*charge) * speed).round() as i32;
}

/// Whether radial mining is breaking its extra blocks (so they don't mine areas themselves).
static MINING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// At the head of `native_destroy_block`: the tool's effects on breaking a block, and radial mining.
pub(super) fn destroy_block(game: &mut super::Game<'_>, pos: [i32; 3]) {
    if MINING.load(Ordering::Relaxed) {
        return;
    }
    let tool = game.inventory[game.selected].clone();
    let effects = stack_effects(tool.as_ref());
    for effect in &effects {
        match effect {
            Effect::MobEffect { source, target: Target::BreakSelf, effect, duration, amplifier, cooldown } if ready(source, Target::BreakSelf, *cooldown) => {
                effect_on_player(game, effect, *amplifier, *duration);
            }
            Effect::Radial { width, x_off, height, y_off } => {
                let telepathic = effects.iter().any(|e| matches!(e, Effect::Telepathic));
                let player = game.native_acting_id();
                MINING.store(true, Ordering::Relaxed);
                for p in fx::radial_area(pos, game.yaw, game.pitch, *width, *x_off, *height, *y_off) {
                    if !radial_breaks(game, p, tool.as_ref()) {
                        continue;
                    }
                    let drops = game.native_machine_destroy_block(p, tool.as_ref(), telepathic).unwrap_or_default();
                    for stack in drops {
                        if let Some(rest) = game.native_give_stack(player, stack) {
                            drop_stack(game, p, &rest);
                        }
                    }
                }
                MINING.store(false, Ordering::Relaxed);
            }
            _ => {}
        }
    }
}

/// `stack` dropped as an item at the block `pos`.
fn drop_stack(game: &mut super::Game<'_>, pos: [i32; 3], stack: &ItemStack) {
    let dimension = game.dim.id().to_owned();
    let nbt = compound(vec![("Item", horizon_items::item_nbt::item_to_nbt(stack)), ("PickupDelay", Tag::Short(10))]);
    let at = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.25, f64::from(pos[2]) + 0.5];
    let _ = game.native_summon(&dimension, "minecraft:item", at, nbt);
}

/// Whether radial mining takes the block at `pos` too: breakable, and one the tool is made for (as
/// Apotheosis' radial affix asks), so a pickaxe leaves the dirt and bedrock around the ore.
fn radial_breaks(game: &super::Game<'_>, pos: [i32; 3], tool: Option<&ItemStack>) -> bool {
    let state = game.world.block(pos[0], pos[1], pos[2]);
    let facts = game.res.facts().get(state);
    facts.hardness >= 0.0
        && game.items().mining_speed(tool, &facts.name) > 1.0
        && (!facts.requires_tool || game.items().correct_for_drops(tool, &facts.name))
}

/// At the head of `native_loot_drops`: telepathic drops go to the player, and a festive kill now and then
/// showers its loot.
pub(super) fn loot_drops(game: &mut super::Game<'_>, context: &horizon_client::mod_api::NativeLootDrops<'_>, drops: &mut Vec<ItemStack>) {
    let Some(player) = context.player else { return };
    let held = game.inventory[game.selected].clone();
    let effects = if context.block.is_some() { stack_effects(context.tool) } else { stack_effects(held.as_ref()) };
    if effects.is_empty() {
        return;
    }
    if context.entity.is_some() {
        for effect in &effects {
            if let Effect::Festive { chance, rolls } = effect {
                let mut rng = horizon_world::apotheosis::Rng::new(now_secs() ^ u64::from(drops.len() as u32).rotate_left(7) ^ context.origin[0].to_bits());
                if f64::from(rng.next_f32()) < *chance {
                    let original = drops.clone();
                    for _ in 1..*rolls {
                        drops.extend(original.iter().cloned());
                    }
                }
            }
        }
    }
    if effects.iter().any(|e| matches!(e, Effect::Telepathic)) {
        let all = std::mem::take(drops);
        for stack in all {
            if let Some(rest) = game.native_give_stack(player, stack) {
                drops.push(rest);
            }
        }
    }
}

/// The tooltip lines for a stack's effects beyond attributes.
fn effect_lines(stack: &ItemStack, lines: &mut Vec<(horizon_client::text::Text, u32)>) {
    for effect in stack_effects(Some(stack)) {
        let text: String = fx::describe(&effect).iter().map(render).collect();
        lines.push((horizon_client::text::Text::legacy(&format!("§a{text}")), u32::MAX));
    }
}
