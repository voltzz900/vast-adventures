// A gem into an item's socket at the anvil.
fn vastadventures_anvil(
    left: &horizon_client::items::ItemStack,
    right: Option<&horizon_client::items::ItemStack>,
    output: &mut Option<horizon_client::items::ItemStack>,
    xp_cost: &mut i32,
    material_cost: &mut i32,
) {
    crate::game::vastadventures_client::anvil(left, right, output, xp_cost, material_cost);
}
