//! What affixes and gems do beyond the attributes the game applies itself: the weapon's effects and the Apothic
//! attributes on a melee hit, the armour's damage reduction, dodging and effects when hurt, life steal and healing
//! received, the bow's draw speed and its arrows' effects and damage, the tool's effects and radial mining, the
//! experience gained, and telepathic and festive drops.
//!
//! The damage hooks can't reach the world, so each tracked player's Apothic attributes and worn effects are read
//! every few ticks, and what the hooks decide waits for the next server tick.

use crate::effects::{self as fx, APOTHIC, Effect, Target};
use crate::gems::is_monster;
use crate::loot::{self, Rng};
use crate::nbt;
use mod_api::{
    ActiveEffect, AttributeModifier, AttributeOperation, EntityRef, EquipmentSlot, Flow, Hand, Identifier, ItemStack,
    LivingDamagePostEvent, LivingIncomingDamageEvent, LootDrops, ModifierLifetime, ServerHost,
};
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// A player the hooks know: their Apothic attributes, worn armour's effects, last bow's effects and tier luck.
#[derive(Clone)]
struct Fighter {
    uuid: u128,
    entity: u32,
    stats: Vec<(&'static str, f64)>,
    worn: Vec<Effect>,
    bow: Vec<Effect>,
    luck: f64,
}

/// The players on the server (by uuid; their entity changes when they respawn).
static PLAYERS: Mutex<Vec<u128>> = Mutex::new(Vec::new());
static FIGHTERS: Mutex<Vec<Fighter>> = Mutex::new(Vec::new());
static TICKS: AtomicU64 = AtomicU64::new(0);

/// A melee blow a player started (`attack_entity`), for the hit that follows.
struct Melee {
    target: i32,
    attacker: i32,
    scale: f64,
    health: f64,
    max_health: f64,
    armor: f64,
    toughness: f64,
    effects: Vec<Effect>,
    near4: Vec<u32>,
    near3: Vec<u32>,
    at: [f64; 3],
}

static MELEE: Mutex<Vec<Melee>> = Mutex::new(Vec::new());
/// Arrows of a player hitting a mob: (target, shooter), for the damage that follows.
static ARROW_HITS: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());

/// A player's projectile hit an entity: its bow's effects wait for the hit to be dealt (`damage_dealt`), so a hit
/// the game refuses (PvP off, a team without friendly fire, a dodge, invulnerability) gives none.
struct ArrowEffects {
    projectile: u32,
    target: u32,
    owner: u32,
    effects: Vec<Effect>,
}

static ARROW_EFFECTS: Mutex<Vec<ArrowEffects>> = Mutex::new(Vec::new());

/// When each effect may fire again, by the player's entity id and the effect (its source and target). An entry
/// goes once its time has passed (`prune`), or with its player (`forget`).
struct Cooldowns(BTreeMap<(i32, String), Instant>);

impl Cooldowns {
    const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Whether the effect `key` may fire for `entity` at `now`; when it may, it waits `cooldown` from now.
    fn ready(&mut self, entity: i32, key: String, cooldown: Duration, now: Instant) -> bool {
        let key = (entity, key);
        if self.0.get(&key).is_some_and(|until| now < *until) {
            return false;
        }
        self.0.insert(key, now + cooldown);
        true
    }

    /// Drops the entries whose time has passed.
    fn prune(&mut self, now: Instant) {
        self.0.retain(|_, until| *until > now);
    }

    /// Drops an entity's entries (a player who left or died: a new body has a new id).
    fn forget(&mut self, entity: i32) {
        self.0.retain(|(e, _), _| *e != entity);
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.0.len()
    }
}

static COOLDOWNS: Mutex<Cooldowns> = Mutex::new(Cooldowns::new());
/// Server ticks between two prunings of the cooldowns.
const PRUNE_TICKS: u64 = 200;
/// Whether radial mining is breaking its extra blocks (so they don't mine areas themselves).
static MINING: AtomicBool = AtomicBool::new(false);

/// What the hooks decided, for the next server tick.
enum Pending {
    Effect { entity: u32, effect: String, amplifier: i32, duration: i32 },
    Hurt { entity: u32, amount: f32, damage_type: &'static str, causing: u32 },
    Bolt { near: u32, at: [f64; 3] },
    Heal { entity: u32, amount: f32 },
}

impl Pending {
    /// The entity it is done to (or beside): it is done in that entity's level.
    fn entity(&self) -> u32 {
        match self {
            Self::Effect { entity, .. } | Self::Hurt { entity, .. } | Self::Heal { entity, .. } => *entity,
            Self::Bolt { near, .. } => *near,
        }
    }
}

/// What waits for the world, with the server ticks it has waited.
static PENDING: Mutex<Vec<(Pending, u8)>> = Mutex::new(Vec::new());

fn queue(items: impl IntoIterator<Item = Pending>) {
    if let Ok(mut pending) = PENDING.lock() {
        pending.extend(items.into_iter().map(|p| (p, 0)));
    }
}

pub fn track(uuid: u128) {
    if let Ok(mut players) = PLAYERS.lock()
        && !players.contains(&uuid)
    {
        players.push(uuid);
    }
}

/// A player left: they are no longer read, and their cooldowns go.
pub fn untrack(uuid: u128) {
    if let Ok(mut players) = PLAYERS.lock() {
        players.retain(|u| *u != uuid);
    }
    let gone: Vec<u32> = match FIGHTERS.lock() {
        Ok(mut fighters) => {
            let gone = fighters.iter().filter(|f| f.uuid == uuid).map(|f| f.entity).collect();
            fighters.retain(|f| f.uuid != uuid);
            gone
        }
        Err(_) => Vec::new(),
    };
    for entity in gone {
        forget(entity);
    }
}

/// An entity's cooldowns go (a player who died respawns with a new entity id).
pub fn forget(entity: u32) {
    if let Ok(mut cooldowns) = COOLDOWNS.lock() {
        cooldowns.forget(entity as i32);
    }
}

/// The server starts or stops: nothing of another world's entities is kept.
pub fn reset() {
    if let Ok(mut players) = PLAYERS.lock() {
        players.clear();
    }
    if let Ok(mut fighters) = FIGHTERS.lock() {
        fighters.clear();
    }
    if let Ok(mut melee) = MELEE.lock() {
        melee.clear();
    }
    if let Ok(mut hits) = ARROW_HITS.lock() {
        hits.clear();
    }
    if let Ok(mut arrows) = ARROW_EFFECTS.lock() {
        arrows.clear();
    }
    if let Ok(mut pending) = PENDING.lock() {
        pending.clear();
    }
    if let Ok(mut cooldowns) = COOLDOWNS.lock() {
        *cooldowns = Cooldowns::new();
    }
}

/// The base of an Apothic attribute.
fn base(attribute: &str) -> f64 {
    APOTHIC.iter().find(|(a, _)| *a == attribute).map_or(0.0, |(_, b)| *b)
}

fn read<T>(entity: u32, f: impl FnOnce(&Fighter) -> T) -> Option<T> {
    FIGHTERS.lock().ok()?.iter().find(|x| x.entity == entity).map(f)
}

/// Player entity `entity`'s value of Apothic attribute `attribute` (its base for anyone else).
pub fn stat(entity: u32, attribute: &str) -> f64 {
    read(entity, |f| f.stats.iter().find(|(a, _)| *a == attribute).map(|(_, v)| *v)).flatten().unwrap_or_else(|| base(attribute))
}

pub fn stat_by_uuid(uuid: u128, attribute: &str) -> f64 {
    let entity = FIGHTERS.lock().ok().and_then(|f| f.iter().find(|x| x.uuid == uuid).map(|x| x.entity));
    entity.map_or_else(|| base(attribute), |e| stat(e, attribute))
}

/// A stack's effects, from its `custom_data`.
fn stack_effects(stack: Option<&ItemStack>) -> Vec<Effect> {
    let Some(stack) = stack else { return Vec::new() };
    nbt::custom_data(stack).map_or_else(Vec::new, |c| fx::effects(&nbt::stack_id(stack), &c))
}

/// Whether an effect from `source` may fire now for `entity` (its cooldown in ticks has passed), marking it fired.
fn ready(entity: i32, source: &str, target: Target, cooldown: i32) -> bool {
    let Ok(mut cooldowns) = COOLDOWNS.lock() else { return true };
    let wait = Duration::from_millis(cooldown.max(0) as u64 * 50);
    cooldowns.ready(entity, format!("{source}/{target:?}"), wait, Instant::now())
}

fn tier_luck_id() -> Option<Identifier> {
    Identifier::parse("vastadventures:tier/luck").ok()
}

/// Reads the attributes and worn effects of each tracked player in the level in place, and keeps their tier luck
/// (Apotheosis' tier augments).
fn refresh(host: &mut dyn ServerHost) {
    let uuids = PLAYERS.lock().map(|p| p.clone()).unwrap_or_default();
    let old = FIGHTERS.lock().map(|mut f| std::mem::take(&mut *f)).unwrap_or_default();
    let luck = [0.0, 0.2, 0.3, 0.5, 1.0][loot::tier()];
    let dimension = host.dimension();
    let mut fresh = Vec::new();
    for uuid in uuids {
        let Some(player) = host.player(uuid) else { continue };
        if player.dimension(&*host).is_none_or(|d| d.to_string() != dimension) {
            // Another level's: kept as it was read there.
            fresh.extend(old.iter().filter(|f| f.uuid == uuid).map(Fighter::clone));
            continue;
        }
        let entity = player.entity();
        let Some(living) = host.living(entity) else { continue };
        let stats = APOTHIC.iter().map(|&(a, b)| (a, living.attribute_value(&*host, a).unwrap_or(b))).collect();
        let worn = [EquipmentSlot::Head, EquipmentSlot::Chest, EquipmentSlot::Legs, EquipmentSlot::Feet]
            .into_iter()
            .flat_map(|s| stack_effects(living.equipment(&*host, s).as_ref()))
            .collect();
        let previous = old.iter().find(|f| f.uuid == uuid && f.entity == entity.id());
        let bow = previous.map(|f| f.bow.clone()).unwrap_or_default();
        if previous.is_none_or(|f| f.luck != luck)
            && let Some(id) = tier_luck_id()
        {
            if luck > 0.0 {
                let modifier = AttributeModifier::new(id, luck, AttributeOperation::AddMultipliedTotal);
                living.replace_attribute_modifier(host, "minecraft:luck", modifier, ModifierLifetime::Transient);
            } else {
                living.remove_attribute_modifier(host, "minecraft:luck", &id);
            }
        }
        fresh.push(Fighter { uuid, entity: entity.id(), stats, worn, bow, luck });
    }
    if let Ok(mut f) = FIGHTERS.lock() {
        *f = fresh;
    }
}

fn add_effect(host: &mut dyn ServerHost, entity: u32, effect: &str, amplifier: i32, duration: i32) {
    let (Some(living), Some(key)) = (host.entity(entity).and_then(|e| host.living(e)), host.effect_key(effect)) else {
        return;
    };
    living.add_effect(host, &ActiveEffect::new(key, duration, amplifier.clamp(0, 255), false, true, true));
}

/// A server tick starts: the last tick's blows are over, and what no level could do in a whole tick is dropped.
pub fn tick_start() {
    let ticks = TICKS.fetch_add(1, Ordering::Relaxed) + 1;
    if ticks.is_multiple_of(PRUNE_TICKS)
        && let Ok(mut cooldowns) = COOLDOWNS.lock()
    {
        cooldowns.prune(Instant::now());
    }
    if let Ok(mut melee) = MELEE.lock() {
        melee.clear();
    }
    if let Ok(mut hits) = ARROW_HITS.lock() {
        hits.clear();
    }
    if let Ok(mut arrows) = ARROW_EFFECTS.lock() {
        arrows.clear();
    }
    if let Ok(mut pending) = PENDING.lock() {
        pending.retain_mut(|(_, age)| {
            *age += 1;
            *age <= 2
        });
    }
}

/// The level in place ticks: its players are read now and then, and what waits for its entities is done.
pub fn level_tick(host: &mut dyn ServerHost) {
    if TICKS.load(Ordering::Relaxed).is_multiple_of(5) {
        refresh(host);
    }
    let pending = PENDING.lock().map(|mut p| std::mem::take(&mut *p)).unwrap_or_default();
    let mut later = Vec::new();
    for (p, age) in pending {
        if host.entity(p.entity()).is_none() {
            later.push((p, age));
            continue;
        }
        match p {
            Pending::Effect { entity, effect, amplifier, duration } => add_effect(host, entity, &effect, amplifier, duration),
            Pending::Hurt { entity, amount, damage_type, causing } => {
                if let Some(e) = host.entity(entity) {
                    let by = host.entity(causing);
                    let _ = e.hurt(host, damage_type, amount, by, by);
                }
            }
            Pending::Bolt { near, at } => {
                if let Some(level) = host.entity(near).and_then(|e| e.level(&*host)) {
                    level.strike_lightning(host, at, true);
                }
            }
            Pending::Heal { entity, amount } => {
                if let Some(living) = host.entity(entity).and_then(|e| host.living(e)) {
                    living.heal(host, amount);
                }
            }
        }
    }
    if let Ok(mut pending) = PENDING.lock() {
        pending.extend(later);
    }
}

/// The monsters within `radius` of `at` in `center`'s level, but `center`.
fn monsters_near(host: &dyn mod_api::Host, center: EntityRef, at: [f64; 3], radius: f64) -> Vec<u32> {
    let Some(level) = center.level(host) else { return Vec::new() };
    let min = [at[0] - radius, at[1] - radius, at[2] - radius];
    let max = [at[0] + radius, at[1] + radius, at[2] + radius];
    level
        .entities_in(host, min, max)
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.id() != center.id())
        .filter(|e| e.kind(host).is_some_and(|k| is_monster(&k.id(host))))
        .filter(|e| e.distance_to_sqr(host, at).is_some_and(|d| d <= radius * radius))
        .map(|e| e.id())
        .collect()
}

/// A player's blow on `target` starts: what the hit that follows needs of the world.
pub fn attack(host: &mut dyn ServerHost, target: i32) {
    let Some(player) = host.players().acting_player() else { return };
    track(player.uuid());
    let attacker = player.entity();
    let Some(entity) = host.entity(target as u32) else { return };
    let Some(living) = host.living(entity) else { return };
    let host: &dyn mod_api::Host = &*host;
    let weapon = host.living(attacker).and_then(|l| l.hand_item(host, Hand::Main));
    let at = entity.position(host).unwrap_or_default();
    let effects = stack_effects(weapon.as_ref());
    let wants_near = effects.iter().any(|e| matches!(e, Effect::Thunderstruck { .. } | Effect::Cleaving { .. }));
    let melee = Melee {
        target,
        attacker: attacker.id() as i32,
        scale: f64::from(player.attack_strength_scale(host, 0.5).unwrap_or(1.0)),
        health: f64::from(living.health(host).unwrap_or(0.0)),
        max_health: f64::from(living.max_health(host).unwrap_or(0.0)),
        armor: living.attribute_value(host, "minecraft:armor").unwrap_or(0.0),
        toughness: living.attribute_value(host, "minecraft:armor_toughness").unwrap_or(0.0),
        near4: if wants_near { monsters_near(host, entity, at, 4.0) } else { Vec::new() },
        near3: if wants_near { monsters_near(host, entity, at, 3.0) } else { Vec::new() },
        effects,
        at,
    };
    if let Ok(mut list) = MELEE.lock() {
        list.retain(|m| m.target != target || m.attacker != melee.attacker);
        list.push(melee);
    }
}

/// Damage coming to a living entity: a player's melee hit or arrow on a mob, or a hit on a player.
pub fn incoming(event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
    let source = event.source;
    let entity = event.entity;
    if event.kind != "minecraft:player" {
        if source.damage_type == "minecraft:player_attack" {
            let melee = MELEE.lock().ok().and_then(|mut list| {
                let i = list.iter().position(|m| m.target == entity && m.attacker == source.causing)?;
                Some(list.swap_remove(i))
            });
            if let Some(m) = melee {
                let amount = melee_hit(&m, f64::from(event.container.new_damage()));
                event.container.set_new_damage(amount as f32);
            }
        } else if matches!(source.damage_type, "minecraft:arrow" | "minecraft:trident") && source.causing > 0 {
            let shooter = source.causing as u32;
            let marked = ARROW_HITS.lock().ok().and_then(|mut hits| {
                let i = hits.iter().position(|&(t, s)| t as i32 == entity && s == shooter)?;
                Some(hits.swap_remove(i))
            });
            if marked.is_some() {
                let s = |a: &str| stat(shooter, a);
                let bonus = s("vastadventures:arrow_damage")
                    * s("vastadventures:projectile_damage")
                    * fx::crit_multiplier(s("vastadventures:crit_chance"), s("vastadventures:crit_damage"), fx::roll);
                event.container.set_new_damage((f64::from(event.container.new_damage()) * bonus) as f32);
            }
        }
        return Flow::Continue;
    }
    // Dodging a hit from another entity.
    let player = entity as u32;
    let dodge = stat(player, "vastadventures:dodge_chance");
    if dodge > 0.0 && source.causing > 0 && source.causing != entity && fx::roll() < dodge {
        return Flow::Cancel;
    }
    let worn = read(player, |f| f.worn.clone()).unwrap_or_default();
    let cut = fx::reduction(&worn, source.damage_type);
    if cut > 0.0 {
        event.container.set_new_damage(event.container.new_damage() * (1.0 - cut) as f32);
    }
    let mut out = Vec::new();
    for effect in &worn {
        if let Effect::MobEffect { source: from, target, effect, duration, amplifier, cooldown } = effect {
            let to = match target {
                Target::HurtSelf => player,
                Target::HurtAttacker if source.causing > 0 => source.causing as u32,
                _ => continue,
            };
            if ready(entity, from, *target, *cooldown) {
                out.push(Pending::Effect { entity: to, effect: effect.clone(), amplifier: *amplifier, duration: *duration });
            }
        }
    }
    queue(out);
    Flow::Continue
}

/// A player's melee hit on a mob: the Apothic attributes (damage from the target's health, fire and cold damage
/// scaled by the swing, critical strikes, armour pierce and shred), then the weapon's effects.
fn melee_hit(m: &Melee, damage: f64) -> f64 {
    let s = |a: &str| stat(m.attacker as u32, a);
    let mut amount = damage;
    amount += m.health * s("vastadventures:current_hp_damage") * m.scale;
    amount += (s("vastadventures:fire_damage") + s("vastadventures:cold_damage")) * m.scale;
    amount *= fx::crit_multiplier(s("vastadventures:crit_chance"), s("vastadventures:crit_damage"), fx::roll);
    amount = fx::pierced(amount, m.armor, m.toughness, s("vastadventures:armor_shred"), s("vastadventures:armor_pierce"));
    let target = m.target as u32;
    let attacker = m.attacker as u32;
    let mut out = Vec::new();
    for effect in &m.effects {
        match effect {
            Effect::MobEffect { source, target: t @ (Target::AttackTarget | Target::AttackSelf), effect, duration, amplifier, cooldown }
                if ready(m.attacker, source, *t, *cooldown) =>
            {
                let entity = if *t == Target::AttackTarget { target } else { attacker };
                out.push(Pending::Effect { entity, effect: effect.clone(), amplifier: *amplifier, duration: *duration });
            }
            Effect::Executing { threshold } => {
                if m.max_health > 0.0 && (m.health - amount) / m.max_health < *threshold {
                    amount = amount.max(m.health + 1.0);
                }
            }
            Effect::Thunderstruck { damage } => {
                for &other in &m.near4 {
                    out.push(Pending::Hurt { entity: other, amount: *damage as f32, damage_type: "minecraft:lightning_bolt", causing: attacker });
                }
                out.push(Pending::Bolt { near: attacker, at: m.at });
            }
            Effect::Cleaving { chance, targets } if fx::roll() < *chance => {
                for &other in m.near3.iter().take(*targets as usize) {
                    out.push(Pending::Hurt { entity: other, amount: amount as f32, damage_type: "minecraft:player_attack", causing: attacker });
                }
            }
            _ => {}
        }
    }
    queue(out);
    amount
}

/// A hit was dealt (it passed the PvP and team checks, the dodge and the invulnerability): a player's projectile
/// gives its bow's effects now, and damage a player dealt in melee heals them by their life steal.
pub fn damage_dealt(event: &LivingDamagePostEvent<'_>) {
    let source = event.source;
    if source.direct > 0 && event.entity > 0 {
        arrow_dealt(source.direct as u32, event.entity as u32);
    }
    if event.kind == "minecraft:player" || source.damage_type != "minecraft:player_attack" || source.causing <= 0 {
        return;
    }
    let attacker = source.causing as u32;
    let heal = f64::from(event.container.health_damage()) * stat(attacker, "vastadventures:life_steal");
    if heal > 0.0 {
        queue([Pending::Heal { entity: attacker, amount: heal as f32 }]);
    }
}

/// A bow let go: its effects for the arrow, and the player's draw speed.
pub fn arrow_loose(host: &mut dyn ServerHost, stack: &ItemStack, charge: &mut i32) {
    let Some(player) = host.players().acting_player() else { return };
    track(player.uuid());
    let entity = player.entity().id();
    let effects = stack_effects(Some(stack));
    if let Ok(mut fighters) = FIGHTERS.lock()
        && let Some(f) = fighters.iter_mut().find(|f| f.entity == entity)
    {
        f.bow = effects;
    }
    *charge = (f64::from(*charge) * stat(entity, "vastadventures:draw_speed")).round() as i32;
}

/// A projectile hit something: a player's arrow hitting a mob (for its damage), and the bow's arrow effects, kept
/// until the hit is dealt.
pub fn projectile(host: &mut dyn ServerHost, projectile: u32, kind: &str, owner: u32, entity: Option<u32>) {
    let Some(target) = entity else { return };
    if !(kind.ends_with("arrow") || kind.ends_with("trident")) {
        return;
    }
    let Some(mut effects) = read(owner, |f| f.bow.clone()) else { return };
    if let Ok(mut hits) = ARROW_HITS.lock() {
        hits.push((target, owner));
    }
    if effects.is_empty() {
        let held = host.entity(owner).and_then(|e| host.living(e)).and_then(|l| {
            let is_bow = |s: &ItemStack| loot::category(&nbt::stack_id(s)) == Some("bow");
            l.hand_item(&*host, Hand::Main).filter(is_bow).or_else(|| l.hand_item(&*host, Hand::Off).filter(is_bow))
        });
        effects = stack_effects(held.as_ref());
    }
    effects.retain(|e| matches!(e, Effect::MobEffect { target: Target::ArrowTarget | Target::ArrowSelf, .. }));
    if effects.is_empty() {
        return;
    }
    if let Ok(mut arrows) = ARROW_EFFECTS.lock() {
        arrows.retain(|a| a.projectile != projectile || a.target != target);
        arrows.push(ArrowEffects { projectile, target, owner, effects });
    }
}

/// The hit of `projectile` on `target` was dealt: the bow's effects kept for it fire (each once its cooldown
/// has passed).
fn arrow_dealt(projectile: u32, target: u32) {
    let kept = ARROW_EFFECTS.lock().ok().and_then(|mut arrows| {
        let i = arrows.iter().position(|a| a.projectile == projectile && a.target == target)?;
        Some(arrows.swap_remove(i))
    });
    let Some(ArrowEffects { target, owner, effects, .. }) = kept else { return };
    let mut out = Vec::new();
    for effect in &effects {
        if let Effect::MobEffect { source, target: t @ (Target::ArrowTarget | Target::ArrowSelf), effect, duration, amplifier, cooldown } = effect
            && ready(owner as i32, source, *t, *cooldown)
        {
            let to = if *t == Target::ArrowTarget { target } else { owner };
            out.push(Pending::Effect { entity: to, effect: effect.clone(), amplifier: *amplifier, duration: *duration });
        }
    }
    queue(out);
}

/// A player broke a block: the tool's effects on breaking, and radial mining.
pub fn destroyed_block(host: &mut dyn ServerHost, pos: [i32; 3]) {
    if MINING.load(Ordering::Relaxed) {
        return;
    }
    let Some(player) = host.players().acting_player() else { return };
    let entity = player.entity();
    let tool = host.living(entity).and_then(|l| l.hand_item(&*host, Hand::Main));
    let effects = stack_effects(tool.as_ref());
    let telepathic = effects.iter().any(|e| matches!(e, Effect::Telepathic));
    for effect in &effects {
        match effect {
            Effect::MobEffect { source, target: Target::BreakSelf, effect, duration, amplifier, cooldown }
                if ready(entity.id() as i32, source, Target::BreakSelf, *cooldown) =>
            {
                add_effect(host, entity.id(), effect, *amplifier, *duration);
            }
            Effect::Radial { width, x_off, height, y_off } => {
                let ([yaw, pitch], Some(level), Some(air)) =
                    (entity.rotation(&*host).unwrap_or_default(), player.level(&*host), host.block_state_id("minecraft:air"))
                else {
                    continue;
                };
                MINING.store(true, Ordering::Relaxed);
                for p in fx::radial_area(pos, yaw, pitch, *width, *x_off, *height, *y_off) {
                    // Breakable, and a block the tool is made for, so a pickaxe leaves the dirt around the ore.
                    let state = level.block_state(&*host, p);
                    let suits = level.block_break_speed(&*host, p, tool.as_ref()).is_some_and(|s| s > 1.0);
                    if state.is_some_and(|s| s != air) && suits {
                        player.destroy_block(host, p, telepathic);
                    }
                }
                MINING.store(false, Ordering::Relaxed);
            }
            _ => {}
        }
    }
}

/// Drops of a block or a mob: telepathic ones go to the player, and a festive kill now and then showers its loot.
pub fn loot_drops(host: &mut dyn ServerHost, context: &LootDrops<'_>, drops: &mut Vec<ItemStack>) {
    let Some(uuid) = context.player else { return };
    let Some(player) = host.player(uuid) else { return };
    let held = host.living(player.entity()).and_then(|l| l.hand_item(&*host, Hand::Main));
    let tool = if context.block.is_some() { context.tool.cloned() } else { context.weapon.cloned().or(held) };
    let effects = stack_effects(tool.as_ref());
    if effects.is_empty() {
        return;
    }
    if context.entity.is_some() {
        for effect in &effects {
            if let Effect::Festive { chance, rolls } = effect {
                let mut rng = Rng::new(fx::roll().to_bits() ^ (drops.len() as u64).rotate_left(7) ^ context.origin[0].to_bits());
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
            if player.give_stack(host, stack.clone()).is_err() {
                drops.push(stack);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldowns_wait_then_expire() {
        let mut c = Cooldowns::new();
        let t0 = Instant::now();
        let wait = Duration::from_secs(2);
        assert!(c.ready(7, "a/ArrowTarget".into(), wait, t0), "fires the first time");
        assert!(!c.ready(7, "a/ArrowTarget".into(), wait, t0 + Duration::from_secs(1)), "waits its cooldown");
        assert!(c.ready(8, "a/ArrowTarget".into(), wait, t0 + Duration::from_secs(1)), "another player's own");
        assert!(c.ready(7, "a/ArrowTarget".into(), wait, t0 + wait), "fires again once it passed");
        // A pruning keeps only what still waits.
        c.ready(9, "b/HurtSelf".into(), Duration::ZERO, t0);
        assert_eq!(c.len(), 3);
        c.prune(t0 + Duration::from_secs(3));
        assert_eq!(c.len(), 1, "player 7's second firing waits until t0 + 4s");
        c.prune(t0 + Duration::from_secs(10));
        assert_eq!(c.len(), 0, "all expired");
    }

    #[test]
    fn cooldowns_go_with_their_player() {
        let mut c = Cooldowns::new();
        let t0 = Instant::now();
        let wait = Duration::from_secs(60);
        c.ready(1, "a/AttackTarget".into(), wait, t0);
        c.ready(1, "b/AttackSelf".into(), wait, t0);
        c.ready(2, "a/AttackTarget".into(), wait, t0);
        c.forget(1);
        assert_eq!(c.len(), 1);
        assert!(c.ready(1, "a/AttackTarget".into(), wait, t0), "a forgotten player fires again");
        assert!(!c.ready(2, "a/AttackTarget".into(), wait, t0));
    }
}
