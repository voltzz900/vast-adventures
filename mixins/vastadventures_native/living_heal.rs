// A heal (in horizon-client's library: no game here): the player's healing received.
fn vastadventures_living_heal(entity: u32, amount: &mut f32) {
    if horizon_world::vastadventures_effects::player() == Some(entity as i32) {
        *amount *= horizon_world::vastadventures_effects::stat("vastadventures:healing_received") as f32;
    }
}
