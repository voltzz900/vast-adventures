// After the item kinds' modifiers are applied: the stacks' own (affixes and gems).
fn vastadventures_equipment(game: &mut crate::game::Game<'_>) {
    crate::game::vastadventures_client::apply_equipment(game);
}
