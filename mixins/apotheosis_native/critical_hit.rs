// The player's swing: its strength, for the Apothic damage of the hit that follows.
fn apotheosis_critical_hit(game: &crate::game::Game<'_>) {
    crate::game::apotheosis_client::critical_hit(game);
}
