// The world tier is the world's own: read it when a world opens, back to haven when it closes.
fn apotheosis_world_open(game: &mut crate::game::Game<'_>, phase: horizon_client::mod_api::WorldPhase) {
    use horizon_client::mod_api::WorldPhase;
    match phase {
        WorldPhase::WorldOpenReady => {
            let saved = horizon_client::save::read_dat(&game.world_root.join("data/apotheosis/world_tier.dat"));
            let tier = saved
                .as_ref()
                .and_then(|t| t.get("data"))
                .and_then(|d| d.get("tier"))
                .and_then(|t| match t {
                    horizon_world::nbt::Tag::String(s) => s.to_string().ok(),
                    _ => None,
                })
                .and_then(|name| horizon_world::apotheosis::tier_named(&name))
                .unwrap_or(0);
            horizon_world::apotheosis::set_tier(tier);
        }
        WorldPhase::WorldCloseFinal => horizon_world::apotheosis::set_tier(0),
        _ => {}
    }
}
