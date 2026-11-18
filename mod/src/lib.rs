//! Vast Adventures: rarities and affixes on gear, gems and sockets, bosses, elites and dungeons, built from
//! Apotheosis' MIT code and data (Stormraven Studios); the names, words and art are this mod's own.
//!
//! Loot tables get affix gear and gems (`loot`, `gems`); a stack's affixes and gems become attribute modifiers
//! where the game asks for them (`item_attribute_modifier`), the Apothic attributes being the mod's own; the anvil
//! sockets, reforges, cuts and seals, the grindstone salvages (`stations`); natural monsters may turn bosses or
//! elites (`spawns`); and what affixes do beyond attributes happens at the combat, mining and drop hooks
//! (`combat`).

mod bosses;
mod combat;
mod data;
mod effects;
mod gems;
mod loot;
mod nbt;
mod spawns;
mod stations;

use mod_api::serde_json::Value;
use mod_api::{
    self, AnvilCraftPostEvent, AnvilUpdateEvent, ArrowLooseEvent, AttackEntityEvent, AttributeDisplay,
    AttributeModifier, AttributeOperation, AttributeTooltip, BlockDropsEvent, CanGrindstoneRepairEvent, ClientHost,
    ClientSetupEvent, DataLoadContext, EntityJoinLevelEvent, EquipmentSlotGroup, FinalizeSpawnEvent, Flow,
    GetEnchantmentLevelEvent, GrindstonePlaceEvent, HearsProjectileImpactsEvent, Host, HostSide, Hooks, Identifier,
    ItemAttributeModifierEvent, ItemPart, ItemTooltipEvent, LevelHost, LevelRandomHost, LivingDamagePostEvent,
    LivingEquipmentChangeEvent, LivingHealEvent, LevelTickPreEvent, LivingIncomingDamageEvent, LootConditionEvent, LootDropsEvent, LootFunctionEvent,
    LootTableLoadEvent, LoopContext, ModifyDefaultComponentsEvent, PlayerDestroyedBlockEvent, PlayerLoggedInEvent,
    PlayerLoggedOutEvent, ProjectileImpactEvent, Registrar, RegisterCommandsEvent, RegistrationContext,
    RunModCommandEvent, Sentiment, ServerAboutToStartEvent, ServerHost, ServerStartedEvent, ServerStoppedEvent,
    ServerTickEvent, ServerTickPhase, Text,
};

/// A gem's item: the grindstone takes it, to salvage it into gem dust.
struct GemPart;

#[mod_api::horizon_part]
impl ItemPart for GemPart {
    fn can_grindstone_repair(&self, _context: &dyn LoopContext, event: &mut CanGrindstoneRepairEvent<'_>) {
        event.can_repair = true;
    }
}

#[derive(Default)]
struct VastAdventures;

#[mod_api::horizon_mod]
impl Hooks for VastAdventures {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        // The Apothic attributes, on every player.
        let mut on_player = Vec::new();
        for &(id, base) in effects::APOTHIC {
            let path = id.split_once(':').map_or(id, |(_, p)| p);
            let share = effects::apothic_share(id);
            let max = if matches!(path, "dodge_chance" | "armor_shred" | "current_hp_damage") { 1.0 } else { 1024.0 };
            let tooltip = AttributeTooltip {
                display: if share { AttributeDisplay::Percentage } else { AttributeDisplay::Ranged },
                ..Default::default()
            };
            r.declare_attribute(path, base, 0.0, max, true, Sentiment::Positive, tooltip)
                .expect("Vast Adventures: an attribute");
            on_player.push((id.to_owned(), None));
        }
        r.modify_entity_attributes("minecraft:player", on_player).expect("Vast Adventures: the player's attributes");
        for item in gems::gem_items() {
            let key = r.item_key(&item);
            r.item_part(key, GemPart);
        }
    }

    // ---- item ids ----

    fn client_setup(&self, host: &mut dyn ClientHost, _: &mut ClientSetupEvent) {
        nbt::learn_items(&*host);
    }

    fn server_about_to_start(&self, host: &mut dyn ServerHost, _: &mut ServerAboutToStartEvent) {
        nbt::learn_items(&*host);
    }

    // ---- loot ----

    fn modify_default_components(&self, _: &dyn DataLoadContext, event: &mut ModifyDefaultComponentsEvent<'_>) {
        loot::record_defaults(event.item, event.components);
    }

    fn loot_table_load(&self, _: &dyn DataLoadContext, event: &mut LootTableLoadEvent<'_>) {
        loot::table_load(event.id, event.table);
    }

    fn loot_condition(&self, event: &mut LootConditionEvent<'_>) {
        let entity = event.ctx.entity.as_ref().map(|e| e.kind.as_str());
        if let Some(holds) = loot::loot_condition(event.kind, has_player(event.ctx), entity) {
            event.holds = holds;
        }
    }

    fn loot_function(&self, event: &mut LootFunctionEvent<'_>) {
        if event.kind.starts_with("vastadventures:") {
            loot_function(event);
        }
    }

    // ---- what stacks give ----

    fn item_attribute_modifier(&self, event: &mut ItemAttributeModifierEvent<'_>) {
        let Some(custom) = nbt::custom_data(event.stack) else { return };
        let item = nbt::stack_id(event.stack);
        if item.is_empty() {
            return;
        }
        // What the stack's own component says of this mod (0.9 wrote them there) gives way to its data.
        event.modifiers.retain(|m| !m.modifier.id.to_string().starts_with("vastadventures:"));
        for m in loot::own_modifiers(&item, &custom) {
            let (Some(attribute), Some(id)) = (identifier(&m["type"]), identifier(&m["id"])) else { continue };
            let modifier = AttributeModifier::new(id, m["amount"].as_f64().unwrap_or(0.0), operation(&m["operation"]));
            event.modifiers.add(attribute, modifier, slot_group(&m["slot"]));
        }
    }

    fn get_enchantment_level(&self, event: &mut GetEnchantmentLevelEvent<'_>) {
        let Some(custom) = nbt::custom_data(event.stack) else { return };
        let item = nbt::stack_id(event.stack);
        let effects = effects::effects(&item, &custom);
        let mut ids: Vec<String> = match event.target {
            Some(target) => vec![target.to_string()],
            None => event.enchantments.iter().map(|(id, _)| id.to_string()).collect(),
        };
        if event.target.is_none() {
            for e in &effects {
                if let effects::Effect::Enchantment { enchantment, mode, .. } = e
                    && mode == "single"
                    && !ids.contains(enchantment)
                {
                    ids.push(enchantment.clone());
                }
            }
        }
        for id in ids {
            let Ok(key) = Identifier::parse(&id) else { continue };
            let base = event.enchantments.level(&key);
            let bonus = effects::enchantment_bonus(&effects, &id, base);
            if bonus > 0 {
                event.enchantments.set(key, base + bonus);
            }
        }
    }

    fn item_tooltip(&self, _host: &dyn ClientHost, event: &mut ItemTooltipEvent<'_>) {
        let Some(custom) = nbt::custom_data(event.stack) else { return };
        let item = nbt::stack_id(event.stack);
        for effect in effects::effects(&item, &custom) {
            let mut line = Text::literal("");
            line.style.color = Some("green".into());
            for part in effects::describe(&effect) {
                line = line.append(nbt::text(&part));
            }
            event.lines.push((line, u32::MAX));
        }
    }

    // ---- stations ----

    fn anvil_update(&self, _host: HostSide, event: &mut AnvilUpdateEvent<'_>) -> Flow {
        stations::anvil_update(event.update)
    }

    fn anvil_craft_post(&self, host: &mut dyn ServerHost, event: &mut AnvilCraftPostEvent<'_>) {
        stations::anvil_taken(host, event.player, event.pos, event.craft);
    }

    fn grindstone_place(&self, _host: &mut dyn ServerHost, event: &mut GrindstonePlaceEvent<'_>) -> Flow {
        stations::grindstone_place(event.place)
    }

    // ---- bosses, elites and the world tier ----

    fn server_started(&self, host: &mut dyn ServerHost, _: &mut ServerStartedEvent) {
        nbt::learn_items(&*host);
        spawns::load_tier(host);
    }

    fn server_stopped(&self, _host: &mut dyn ServerHost, _: &mut ServerStoppedEvent) {
        loot::set_tier(0);
    }

    fn finalize_spawn(&self, host: &dyn Host, event: &mut FinalizeSpawnEvent<'_>) -> Flow {
        spawns::finalize_spawn(host, event);
        Flow::Continue
    }

    fn entity_join_level(&self, host: &mut dyn ServerHost, event: &mut EntityJoinLevelEvent<'_>) -> Flow {
        if !event.loaded_from_disk {
            spawns::joined(host, event.entity, event.dimension, event.kind, event.pos);
        }
        Flow::Continue
    }

    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        spawns::register_commands(event.commands);
    }

    fn run_mod_command(&self, host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
        spawns::run_command(host, event.source, event.command)
    }

    fn server_tick(&self, _host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if matches!(event.phase, ServerTickPhase::ServerTickStart { .. }) {
            spawns::tick_start();
            combat::tick_start();
        }
    }

    /// Entities are reached in the level in place: what waits for the world is done in each level's tick.
    fn level_tick_pre(&self, host: HostSide, _event: &mut LevelTickPreEvent) {
        if let HostSide::Server(host) = host {
            nbt::learn_items(&*host);
            spawns::level_tick(host);
            combat::level_tick(host);
        }
    }

    // ---- combat, mining and drops ----

    fn player_logged_in(&self, _host: &mut dyn ServerHost, event: &mut PlayerLoggedInEvent<'_>) {
        combat::track(event.player);
    }

    fn player_logged_out(&self, _host: &mut dyn ServerHost, event: &mut PlayerLoggedOutEvent<'_>) {
        combat::untrack(event.player);
    }

    fn living_equipment_change(&self, host: &mut dyn ServerHost, event: &mut LivingEquipmentChangeEvent<'_>) {
        if event.kind == "minecraft:player"
            && let Some(uuid) = host.entity(event.entity).and_then(|e| e.uuid(host))
        {
            combat::track(uuid);
        }
    }

    fn attack_entity(&self, host: &mut dyn ServerHost, event: &mut AttackEntityEvent<'_>) -> Flow {
        combat::attack(host, event.entity);
        Flow::Continue
    }

    fn living_incoming_damage(&self, _level: &mut dyn LevelRandomHost, event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
        combat::incoming(event)
    }

    fn living_damage_post(&self, _level: &mut dyn LevelRandomHost, event: &mut LivingDamagePostEvent<'_>) {
        combat::damage_dealt(event);
    }

    fn living_heal(&self, _context: &dyn LoopContext, event: &mut LivingHealEvent) -> Flow {
        event.amount *= combat::stat(event.entity, "vastadventures:healing_received") as f32;
        Flow::Continue
    }

    fn arrow_loose(&self, host: &mut dyn ServerHost, event: &mut ArrowLooseEvent<'_>) -> Flow {
        combat::arrow_loose(host, event.stack, &mut event.charge);
        Flow::Continue
    }

    fn hears_projectile_impacts(&self, _host: &dyn Host, event: &mut HearsProjectileImpactsEvent) {
        event.hears = true;
    }

    fn projectile_impact(&self, host: &mut dyn ServerHost, event: &mut ProjectileImpactEvent<'_>) -> Flow {
        combat::projectile(host, event.kind, event.owner, event.entity);
        Flow::Continue
    }

    fn player_destroyed_block(&self, host: &mut dyn ServerHost, event: &mut PlayerDestroyedBlockEvent) {
        combat::destroyed_block(host, event.pos);
    }

    fn block_drops(&self, _level: &dyn LevelHost, event: &mut BlockDropsEvent<'_>) -> Flow {
        if let Some(player) = event.breaker {
            event.experience = effects::scaled_experience(event.experience, combat::stat_by_uuid(player, "vastadventures:experience_gained"));
        }
        Flow::Continue
    }

    fn loot_drops(&self, host: &mut dyn ServerHost, event: &mut LootDropsEvent<'_>) {
        combat::loot_drops(host, event.context, event.drops);
    }
}

/// A player opened the chest or made the kill. The items engine (`/loot`, some drops) names no entity at all, so
/// only a roll that names a mob and no player killer is left out.
fn has_player(ctx: &mod_api::LootContext) -> bool {
    ctx.player.is_some() || ctx.entity.as_ref().is_none_or(|e| e.kind == "minecraft:player")
}

/// This mod's loot functions, on the pure logic's view of the stack (JSON components).
fn loot_function(event: &mut LootFunctionEvent<'_>) {
    let ctx = event.ctx;
    loot::note_context(event.kind, ctx.player.is_some(), ctx.entity.as_ref().map(|e| e.kind.as_str()));
    let stack = &mut *event.stack;
    let mut item = nbt::item_id(stack.item);
    if item.is_empty() {
        return;
    }
    let before_item = item.clone();
    let mut count = stack.count;
    let mut names = Vec::new();
    let before: Vec<(String, Value)> =
        stack.components.iter().filter_map(|(k, v)| Some((k.clone(), nbt::tag_to_json(v.as_ref()?)))).collect();
    let mut components = before.clone();
    let dimension = ctx.place.as_ref().and_then(|p| p.level.as_ref()).map(|l| l.dimension.as_str());
    let mut view = loot::Stack { item: &mut item, count: &mut count, names: &mut names, components: &mut components };
    loot::loot_function(event.kind, event.data, has_player(ctx), ctx.luck, ctx.seed, dimension, &mut view);
    stack.count = count;
    if item != before_item {
        match nbt::item_key(&item) {
            Some(k) => stack.item = k,
            None => stack.count = 0,
        }
    }
    for (kind, name) in names {
        stack.names.retain(|(k, _)| *k != kind);
        stack.names.push((kind, nbt::text(&name)));
    }
    if components != before {
        let mut out: Vec<_> = stack.components.iter().filter(|(_, v)| v.is_none()).cloned().collect();
        out.extend(components.iter().map(|(k, v)| (k.clone(), Some(nbt::json_to_tag(v)))));
        stack.components = out;
    }
}

fn identifier(value: &Value) -> Option<Identifier> {
    Identifier::parse(value.as_str()?).ok()
}

fn operation(value: &Value) -> AttributeOperation {
    match value.as_str() {
        Some("add_multiplied_base") => AttributeOperation::AddMultipliedBase,
        Some("add_multiplied_total") => AttributeOperation::AddMultipliedTotal,
        _ => AttributeOperation::AddValue,
    }
}

fn slot_group(value: &Value) -> EquipmentSlotGroup {
    match value.as_str() {
        Some("mainhand") => EquipmentSlotGroup::MainHand,
        Some("offhand") => EquipmentSlotGroup::OffHand,
        Some("hand") => EquipmentSlotGroup::Hand,
        Some("head") => EquipmentSlotGroup::Head,
        Some("chest") => EquipmentSlotGroup::Chest,
        Some("legs") => EquipmentSlotGroup::Legs,
        Some("feet") => EquipmentSlotGroup::Feet,
        Some("armor") => EquipmentSlotGroup::Armor,
        _ => EquipmentSlotGroup::Any,
    }
}
