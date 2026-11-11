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
