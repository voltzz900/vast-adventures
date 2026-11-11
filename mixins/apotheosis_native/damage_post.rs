// Damage dealt: the player's life steal.
fn apotheosis_damage_post(kind: &str, damage_type: &str, amount: f32) {
    crate::game::apotheosis_client::damage_dealt(kind, damage_type, amount);
}
