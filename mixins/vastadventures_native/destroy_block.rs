// A block broken by the player: the tool's effects and radial mining.
fn vastadventures_destroy_block(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    crate::game::vastadventures_client::destroy_block(game, pos);
}
