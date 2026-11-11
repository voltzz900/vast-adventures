// Damage coming to a living entity: for the player, the armour's reduction and effects when hurt.
fn apotheosis_incoming(kind: &str, damage_type: &str, causing: i32, amount: &mut f32) {
    crate::game::apotheosis_client::incoming(kind, damage_type, causing, amount);
}
