// Damage dealt: the player's life steal.
fn vastadventures_damage_post(kind: &str, damage_type: &str, amount: f32) {
    crate::game::vastadventures_client::damage_dealt(kind, damage_type, amount);
}
