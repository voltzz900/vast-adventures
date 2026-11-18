//! The anvil and the grindstone: socketing a gem, the seals, reforging and gem cutting at the anvil; salvaging an
//! affix item or a gem at the grindstone.

use crate::gems::{self, MadeGem};
use crate::nbt;
use mod_api::item::menu::{AnvilCraft, AnvilUpdate, GrindstonePlace};
use mod_api::serde_json::{Value, json};
use mod_api::{Flow, ItemStack, ServerHost};

/// A gem as a stack: its item, name and components.
pub fn gem_stack(gem: &MadeGem) -> Option<ItemStack> {
    let mut stack = ItemStack::new(nbt::item_key(&gem.item)?, 1)?;
    nbt::set(&mut stack, "minecraft:custom_name", &gem.name);
    for (id, value) in &gem.components {
        nbt::set(&mut stack, id, value);
    }
    Some(stack)
}

/// `stack`, one of it, with the components given (`null` takes one off).
fn changed(stack: &ItemStack, components: &[(&str, &Value)]) -> ItemStack {
    let mut out = stack.clone();
    let _ = out.set_count(1);
    for (id, value) in components {
        nbt::set(&mut out, id, value);
    }
    out
}

/// The anvil with an item on the left and one of this mod's on the right: a gem goes into a free socket, a seal
/// does its work, a rarity's material reforges and gem dust cuts a gem. A pairing that does nothing has no result.
pub fn anvil_update(update: &mut AnvilUpdate) -> Flow {
    let Some(right) = update.right.clone() else { return Flow::Continue };
    let right_id = nbt::stack_id(&right);
    let left = update.left.clone();
    let left_id = nbt::stack_id(&left);
    if right_id.is_empty() || left_id.is_empty() {
        return Flow::Continue;
    }
    let custom = nbt::component(&left, "minecraft:custom_data");
    let made = if gems::is_sigil(&right_id) {
        gems::sigil(&left_id, custom.as_ref(), &right_id).map(|s| {
            let name = if s.clear_name { Some(Value::Null) } else { None };
            let mut parts = vec![("minecraft:custom_data", &s.custom), ("minecraft:lore", &s.lore)];
            if let Some(name) = &name {
                parts.push(("minecraft:custom_name", name));
            }
            (changed(&left, &parts), 1, 1)
        })
    } else if gems::is_material(&right_id) || right_id == gems::GEM_DUST {
        let custom = custom.unwrap_or(Value::Null);
        let count = right.count() as i32;
        match gems::cut_gem(&left_id, &custom, &right_id, count) {
            Some((gem, levels, dust)) => gem_stack(&gem).map(|s| (s, levels, dust)),
            None => gems::reforge(&left_id, &custom, &right_id, count).map(|r| {
                let name = r.name.clone().unwrap_or(Value::Null);
                let parts = [("minecraft:custom_name", &name), ("minecraft:custom_data", &r.custom), ("minecraft:lore", &r.lore)];
                (changed(&left, &parts), r.levels, r.material_cost)
            }),
        }
    } else if right_id.starts_with("vastadventures:gem_") {
        let gem_custom = nbt::component(&right, "minecraft:custom_data");
        gems::socket(&left_id, custom.as_ref(), &right_id, gem_custom.as_ref())
            .map(|s| (changed(&left, &[("minecraft:custom_data", &s.custom), ("minecraft:lore", &s.lore)]), 1, 1))
    } else {
        return Flow::Continue;
    };
    match made {
        Some((output, xp_cost, material_cost)) => {
            update.output = Some(output);
            update.xp_cost = xp_cost;
            update.material_cost = material_cost;
            Flow::Continue
        }
        None => Flow::Cancel,
    }
}

/// An anvil's result was taken: a Seal of Extraction's gems go to the player, or drop on the anvil.
pub fn anvil_taken(host: &mut dyn ServerHost, player: u128, pos: [i32; 3], craft: &AnvilCraft) {
    let (Some(left), Some(right)) = (&craft.left, &craft.right) else { return };
    if nbt::stack_id(right) != gems::SEAL_OF_EXTRACTION {
        return;
    }
    let Some(custom) = nbt::custom_data(left) else { return };
    let Some(p) = host.player(player) else { return };
    for gem in gems::socketed_gems(&custom) {
        let Some(stack) = gem_stack(&gem) else { continue };
        if p.give_stack(host, stack.clone()).is_err()
            && let Some(level) = p.level(host)
        {
            let at = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 1.1, f64::from(pos[2]) + 0.5];
            let _ = level.spawn_item(host, at[0], at[1], at[2], stack, 10);
        }
    }
}

/// Salvaging: an affix item or a gem alone in the grindstone gives its rarity's material or gem dust.
pub fn grindstone_place(place: &mut GrindstonePlace) -> Flow {
    let stack = match (&place.top, &place.bottom) {
        (Some(s), None) | (None, Some(s)) => s.clone(),
        (Some(a), Some(b)) => {
            // Two gems would take the grindstone's repair branch: no result.
            let gem = |s: &ItemStack| gems::gem_for_item(&nbt::stack_id(s)).is_some();
            return if gem(a) || gem(b) { Flow::Cancel } else { Flow::Continue };
        }
        (None, None) => return Flow::Continue,
    };
    let item = nbt::stack_id(&stack);
    let custom = nbt::component(&stack, "minecraft:custom_data").unwrap_or_else(|| json!({}));
    let Some((material, count)) = gems::salvage(&item, &custom) else { return Flow::Continue };
    let Some(output) = nbt::item_key(&material).and_then(|k| ItemStack::new(k, count.max(1) as u32)) else {
        return Flow::Continue;
    };
    place.output = Some(output);
    place.xp = 0;
    Flow::Continue
}
