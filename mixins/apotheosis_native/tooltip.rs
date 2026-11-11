// The stacks' own modifiers in the tooltip.
fn apotheosis_tooltip(stack: &horizon_client::items::ItemStack, lines: &mut Vec<(horizon_client::text::Text, u32)>) {
    crate::game::apotheosis_client::tooltip(stack, lines);
}
