// Drops of a block or a mob: telepathic and festive.
fn vastadventures_loot_drops(
    game: &mut crate::game::Game<'_>,
    context: &horizon_client::mod_api::NativeLootDrops<'_>,
    drops: &mut Vec<ItemStack>,
) {
    crate::game::vastadventures_client::loot_drops(game, context, drops);
}
