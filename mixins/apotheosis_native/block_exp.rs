// A broken block's experience: the player's experience gained.
fn apotheosis_block_exp(experience: &mut u32) {
    *experience = horizon_world::apotheosis_effects::scaled_experience(*experience);
}
