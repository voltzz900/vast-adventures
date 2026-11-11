// The stacks' own modifiers in the tooltip.
fn vastadventures_tooltip(stack: &horizon_client::items::ItemStack, lines: &mut Vec<(horizon_client::text::Text, u32)>) {
    crate::game::vastadventures_client::tooltip(stack, lines);
}
