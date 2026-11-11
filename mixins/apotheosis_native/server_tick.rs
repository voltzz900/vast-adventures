// Each server tick: the effects queued by hooks that can't reach the game.
fn apotheosis_server_tick(game: &mut crate::game::Game<'_>) {
    crate::game::apotheosis_client::server_tick(game);
}
