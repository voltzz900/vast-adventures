// Gems go into the grindstone too, to be salvaged.
fn apotheosis_grindstone_accepts(stack: &ItemStack) -> core::ops::ControlFlow<bool> {
    if horizon_world::apotheosis_gems::gem_for_item(&stack.item).is_some() {
        core::ops::ControlFlow::Break(true)
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
