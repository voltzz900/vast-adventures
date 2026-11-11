// Salvaging: an affix item or a gem alone in the grindstone gives its material or gem dust. (This file is also
// compiled in horizon-client-lib, so it reaches only horizon-world.)
fn vastadventures_grindstone(input: Option<&ItemStack>, additional: Option<&ItemStack>) -> core::ops::ControlFlow<Option<ItemStack>> {
    let stack = match (input, additional) {
        (Some(s), None) | (None, Some(s)) => s,
        _ => return core::ops::ControlFlow::Continue(()),
    };
    let custom = stack.data().and_then(|d| d.other("minecraft:custom_data")).map(horizon_client_core::scoreboard::tag_to_json);
    match custom.and_then(|c| horizon_world::vastadventures_gems::salvage(&stack.item, &c)) {
        Some((material, count)) => core::ops::ControlFlow::Break(Some(ItemStack::new(&material, count.max(1) as u32))),
        None => core::ops::ControlFlow::Continue(()),
    }
}
