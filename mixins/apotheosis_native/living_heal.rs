// A heal (in horizon-client's library: no game here): the player's healing received.
fn apotheosis_living_heal(entity: u32, amount: &mut f32) {
    if horizon_world::apotheosis_effects::player() == Some(entity as i32) {
        *amount *= horizon_world::apotheosis_effects::stat("apothic_attributes:healing_received") as f32;
    }
}
