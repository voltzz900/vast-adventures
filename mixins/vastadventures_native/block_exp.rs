// A broken block's experience: the player's experience gained.
fn vastadventures_block_exp(experience: &mut u32) {
    *experience = horizon_world::vastadventures_effects::scaled_experience(*experience);
}
