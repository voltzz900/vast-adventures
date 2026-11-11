// Each server tick: the effects queued by hooks that can't reach the game.
fn vastadventures_server_tick(game: &mut crate::game::Game<'_>) {
    crate::game::vastadventures_client::server_tick(game);
}
