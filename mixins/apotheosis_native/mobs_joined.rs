// New mobs in the world: some monsters become bosses or elites.
fn apotheosis_mobs_joined(game: &mut crate::game::Game<'_>, from: usize, natural: bool) {
    crate::game::apotheosis_client::mobs_joined(game, from, natural);
}
