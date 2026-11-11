// The player's melee hit on a mob: the weapon's effects.
fn vastadventures_melee(game: &mut crate::game::Game<'_>, entity: i32, weapon: Option<&ItemStack>, amount: &mut f32) {
    crate::game::vastadventures_client::melee(game, entity, weapon, amount);
}
