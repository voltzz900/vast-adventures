// An item's enchantment level with its affixes' and gems' bonus levels (`GetEnchantmentLevelEvent`). This file
// is compiled in horizon-items, so it reaches only horizon-world.
fn vastadventures_enchantment_level(stack: &ItemStack, id: &str) -> core::ops::ControlFlow<u32> {
    let Some(custom) = stack.data().and_then(|d| d.other("minecraft:custom_data")) else {
        return core::ops::ControlFlow::Continue(());
    };
    let custom = horizon_client_core::scoreboard::tag_to_json(custom);
    if custom.get("vastadventures").is_none() {
        return core::ops::ControlFlow::Continue(());
    }
    let base = stack.enchantments().iter().find(|(e, _)| &**e == id).map_or(0, |(_, level)| *level);
    let effects = horizon_world::vastadventures_effects::effects(&stack.item, &custom);
    match horizon_world::vastadventures_effects::enchantment_bonus(&effects, id, base) {
        0 => core::ops::ControlFlow::Continue(()),
        bonus => core::ops::ControlFlow::Break(base + bonus),
    }
}
