// Damage coming to a living entity: for the player, the armour's reduction, dodging and effects when hurt; for a
// mob, the player's arrows.
fn apotheosis_incoming(entity: i32, kind: &str, damage_type: &str, causing: i32, amount: &mut f32, canceled: &mut bool) {
    crate::game::apotheosis_client::incoming(entity, kind, damage_type, causing, amount, canceled);
}
