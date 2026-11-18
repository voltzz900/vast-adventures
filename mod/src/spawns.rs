//! Bosses, elites and the world tier: monsters the world spawns naturally are marked as they are finalized, and once
//! they join, now and then a boss takes one's place (an invader, at most one every three minutes) or the mob
//! becomes an elite. A boss dungeon's spawner makes a marked mob: the spawner goes and a boss stands there. The
//! world tier is saved with the world and set with `/vastadventures tier`.

use crate::bosses::{self, Plan};
use crate::gems::is_monster;
use crate::loot::{self, Rng, TIERS};
use crate::nbt::{self, key};
use mod_api::command::{CommandArgumentValue, CommandInvocation, CommandSource};
use mod_api::{
    ActiveEffect, BlockUpdateFlags, Command, Compound, ConfigPath, EntityRef, EquipmentSlot, FinalizeSpawnEvent, Flow,
    Host, ServerHost, SpawnReason, Tag, Text,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// The tag a naturally spawned monster carries until it is looked at once it has joined.
const NATURAL_TAG: &str = "vastadventures.natural";
/// The tag of the mob a boss dungeon's spawner makes (`tools/build_worldgen.py`).
const BOSS_SPAWNER_TAG: &str = "vastadventures.boss_spawner";
/// Apotheosis waits `bossSpawnCooldown` (3600 ticks, three minutes) between bosses.
const BOSS_COOLDOWN_SECS: u64 = 180;
static LAST_BOSS: AtomicU64 = AtomicU64::new(0);

/// Mobs that joined since the last tick: (entity, dimension, kind, position).
static JOINED: Mutex<Vec<(u32, String, String, [f64; 3])>> = Mutex::new(Vec::new());

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn seed(id: u32, pos: [f64; 3]) -> u64 {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
    u64::from(id) ^ pos[0].to_bits().rotate_left(17) ^ pos[2].to_bits() ^ nanos
}

/// A monster the world spawns naturally is marked (finalization must stay a pure function of the mob).
pub fn finalize_spawn(_host: &dyn Host, event: &mut FinalizeSpawnEvent<'_>) {
    if matches!(event.reason, SpawnReason::Natural) && is_monster(event.kind) {
        let _ = event.edits.add_tag(event.mob, NATURAL_TAG);
    }
}

/// A mob joined a level (not loaded from disk): looked at next tick.
pub fn joined(entity: u32, dimension: &str, kind: &str, pos: [f64; 3]) {
    if let Ok(mut joined) = JOINED.lock() {
        joined.push((entity, dimension.to_owned(), kind.to_owned(), pos));
    }
}

pub fn server_tick(host: &mut dyn ServerHost) {
    let joined = JOINED.lock().map(|mut j| std::mem::take(&mut *j)).unwrap_or_default();
    for (id, dimension, kind, pos) in joined {
        let Some(entity) = host.entity(id) else { continue };
        let tags = entity.tags(&*host).unwrap_or_default();
        if tags.iter().any(|t| t == BOSS_SPAWNER_TAG) {
            boss_spawner(host, entity, &dimension, pos);
        } else if tags.iter().any(|t| t == NATURAL_TAG) {
            entity.remove_tag(host, NATURAL_TAG);
            natural_monster(host, entity, &dimension, &kind, pos);
        }
    }
}

/// A natural monster: now and then a boss in its place, or the mob made an elite.
fn natural_monster(host: &mut dyn ServerHost, entity: EntityRef, dimension: &str, kind: &str, pos: [f64; 3]) {
    let tier = loot::tier();
    let mut rng = Rng::new(seed(entity.id(), pos));
    let ready = now_secs().saturating_sub(LAST_BOSS.load(Ordering::Relaxed)) >= BOSS_COOLDOWN_SECS;
    if ready
        && f64::from(rng.next_f32()) < bosses::spawn_chance(dimension, tier)
        && spawn_boss(host, dimension, pos, Some(entity), &mut rng)
    {
        return;
    }
    if let Some(plan) = bosses::plan_elite(kind, dimension, tier, 0.0, &mut rng) {
        apply_plan(host, entity, &plan);
    }
}

/// A boss dungeon's spawner made its marked mob (a player came near): the spawner goes, and a boss stands where
/// it was, as Apotheosis' boss spawner does.
fn boss_spawner(host: &mut dyn ServerHost, entity: EntityRef, dimension: &str, pos: [f64; 3]) {
    entity.discard(host);
    let (Some(level), Some(spawner), Some(air)) =
        (host.level(dimension), host.block_state_id("minecraft:spawner"), host.block_state_id("minecraft:air"))
    else {
        return;
    };
    let base = [pos[0].floor() as i32, pos[1].floor() as i32, pos[2].floor() as i32];
    let mut near: Vec<[i32; 3]> = (-4..=4)
        .flat_map(|dx| (-2..=2).flat_map(move |dy| (-4..=4).map(move |dz| [base[0] + dx, base[1] + dy, base[2] + dz])))
        .collect();
    near.sort_by_key(|p| (p[0] - base[0]).pow(2) + (p[1] - base[1]).pow(2) + (p[2] - base[2]).pow(2));
    // No spawner near: the mob wandered off before this saw it. It goes, and the spawner stays for next time.
    let Some(p) = near.into_iter().find(|p| level.block_state(&*host, *p) == Some(spawner)) else { return };
    let _ = level.set_block(host, p, air, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS);
    let at = [f64::from(p[0]) + 0.5, f64::from(p[1]), f64::from(p[2]) + 0.5];
    let mut rng = Rng::new(seed(entity.id(), at));
    spawn_boss(host, dimension, at, None, &mut rng);
}

/// Summons a boss at `pos` in `dimension` (in place of `replaces`, which goes) and announces it. False when no
/// boss could be made there.
fn spawn_boss(host: &mut dyn ServerHost, dimension: &str, pos: [f64; 3], replaces: Option<EntityRef>, rng: &mut Rng) -> bool {
    let block = [pos[0].floor() as i32, pos[1].floor() as i32, pos[2].floor() as i32];
    let biome = host.level(dimension).and_then(|l| l.biome_at(&*host, block));
    let Some(plan) = bosses::plan_boss(dimension, biome.as_deref(), loot::tier(), 0.0, rng) else { return false };
    let Some(boss) = host.summon(dimension, &plan.kind, pos, Compound::new()).and_then(|id| host.entity(id)) else {
        return false;
    };
    apply_plan(host, boss, &plan);
    if let Some(old) = replaces {
        old.discard(host);
    }
    LAST_BOSS.store(now_secs(), Ordering::Relaxed);
    let rarity = plan.rarity.clone().unwrap_or_default();
    let mut rarity_name = Text::translate(format!("rarity.{rarity}"), Vec::new());
    rarity_name.style.color = Some(loot::rarity_color(rarity_index(&rarity)));
    let coordinate = |v: f64| Text::literal(format!("{}", v as i64));
    let mut line = Text::translate(
        "info.vastadventures.boss_spawn",
        vec![rarity_name, nbt::text(&plan.name), coordinate(pos[0]), coordinate(pos[1]), coordinate(pos[2])],
    );
    line.style.color = Some("gold".into());
    let _ = host.messages().broadcast(line);
    eprintln!("Vast Adventures: boss {} ({rarity}) at {pos:?}", plan.kind);
    true
}

fn rarity_index(id: &str) -> usize {
    (0..5).find(|&i| loot::rarity_id(i) == id).unwrap_or(0)
}

fn slot(name: &str) -> Option<EquipmentSlot> {
    Some(match name {
        "mainhand" => EquipmentSlot::MainHand,
        "offhand" => EquipmentSlot::OffHand,
        "head" => EquipmentSlot::Head,
        "chest" => EquipmentSlot::Chest,
        "legs" => EquipmentSlot::Legs,
        "feet" => EquipmentSlot::Feet,
        _ => return None,
    })
}

/// Gives a mob a plan's name, gear (the affix item and the gem always drop), effects, attributes and persistence.
fn apply_plan(host: &mut dyn ServerHost, entity: EntityRef, plan: &Plan) {
    let _ = entity.set_custom_name(host, Some(nbt::text(&plan.name)));
    let Some(living) = host.living(entity) else { return };
    for (name, item) in &plan.equipment {
        if let (Some(slot), Some(stack)) = (slot(name), nbt::stack_from_json(item)) {
            let _ = living.set_equipment(host, slot, Some(stack));
        }
    }
    for (effect, amplifier, ticks) in &plan.effects {
        if let Some(key) = host.effect_key(effect) {
            living.add_effect(host, &ActiveEffect::new(key, *ticks, *amplifier, false, true, true));
        }
    }
    let mut done: Vec<&str> = Vec::new();
    for (attribute, _, _) in &plan.attrs {
        if done.contains(&attribute.as_str()) {
            continue;
        }
        done.push(attribute);
        let Some(base) = living.attribute_base(&*host, attribute) else { continue };
        let value = bosses::final_value(base, attribute, &plan.attrs);
        living.set_attribute_base(host, attribute, value);
        if attribute == "minecraft:max_health" {
            living.set_health(host, value as f32);
        }
    }
    let drops: Compound = plan.always_drops.iter().map(|s| (key(s), Tag::Float(2.0f32.to_bits()))).collect();
    let mut nbt = Compound::new();
    nbt.insert(key("PersistenceRequired"), Tag::Byte(1));
    if !drops.is_empty() {
        nbt.insert(key("drop_chances"), Tag::Compound(drops));
    }
    entity.merge_nbt(host, nbt);
}

// ---- the world tier and the commands ------------------------------------------------------

fn tier_file() -> Option<ConfigPath> {
    ConfigPath::new("world_tier.txt").ok()
}

/// The world's tier, read as it opens (calm when none was set).
pub fn load_tier(host: &mut dyn ServerHost) {
    let saved = tier_file().and_then(|f| host.world_data_files().read(&f).ok());
    let tier = saved.and_then(|b| loot::tier_named(String::from_utf8_lossy(&b).trim())).unwrap_or(0);
    loot::set_tier(tier);
}

/// `/vastadventures tier [<tier>]` shows or sets the world tier; `/vastadventures boss` summons a boss (operators).
pub fn register_commands(commands: &mut Vec<Command>) {
    commands.push(Command::new("vastadventures tier"));
    commands.push(Command::new("vastadventures tier <tier:word>"));
    commands.push(Command::op("vastadventures boss", 2));
}

fn tell(host: &mut dyn ServerHost, line: Text, color: &str) {
    let mut line = line;
    line.style.color = Some(color.into());
    let _ = host.commands().feedback(line);
}

pub fn run_command(host: &mut dyn ServerHost, source: &CommandSource, command: &CommandInvocation) -> Flow {
    match command.literals.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["vastadventures", "boss"] => {
            let CommandSource::Player { player, .. } = source else { return Flow::Cancel };
            let (Some(pos), Some(dimension)) = (player.entity().position(&*host), player.dimension(&*host)) else {
                return Flow::Cancel;
            };
            let mut rng = Rng::new(seed(player.entity().id(), pos));
            if !spawn_boss(host, &dimension.to_string(), pos, None, &mut rng) {
                tell(host, Text::translate("commands.vastadventures.no_boss", Vec::new()), "red");
            }
            Flow::Cancel
        }
        ["vastadventures", "tier"] => {
            let wanted = match command.argument("tier") {
                Some(CommandArgumentValue::String(name)) => Some(name.clone()),
                _ => None,
            };
            let list = Text::literal(TIERS.join(", "));
            match wanted {
                None => {
                    let name = Text::translate(format!("tier.vastadventures.{}", TIERS[loot::tier()]), Vec::new());
                    tell(host, Text::translate("commands.vastadventures.tier", vec![name, list]), "gold");
                }
                Some(name) => match loot::tier_named(&name) {
                    Some(tier) => {
                        loot::set_tier(tier);
                        let saved = tier_file().is_some_and(|f| host.world_data_files().write(&f, TIERS[tier].as_bytes()).is_ok());
                        let name = Text::translate(format!("tier.vastadventures.{}", TIERS[tier]), Vec::new());
                        let key = if saved { "commands.vastadventures.tier_set" } else { "commands.vastadventures.tier_unsaved" };
                        tell(host, Text::translate(key, vec![name]), "gold");
                    }
                    None => tell(host, Text::translate("commands.vastadventures.no_tier", vec![Text::literal(name), list]), "red"),
                },
            }
            Flow::Cancel
        }
        _ => Flow::Continue,
    }
}
