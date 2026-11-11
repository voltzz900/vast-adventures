// The withdrawn gems to the player, once the take is done.
fn apotheosis_anvil_used(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    crate::game::apotheosis_client::anvil_used(game, pos);
}
