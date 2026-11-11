// After the item kinds' modifiers are applied: the stacks' own (affixes and gems).
fn apotheosis_equipment(game: &mut crate::game::Game<'_>) {
    crate::game::apotheosis_client::apply_equipment(game);
}
