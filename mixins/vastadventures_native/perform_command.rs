// /vastadventures tier [calm|wilds|rising|peril|apex]: the world's tier, saved with it.
// /vastadventures boss: summons a boss at the player (operators).
fn vastadventures_perform_command(
    game: &mut crate::game::Game<'_>,
    player: &crate::game::native_server::NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let words: Vec<&str> = command.split_whitespace().collect();
    if words.as_slice() == ["vastadventures", "boss"] {
        *ran = true;
        if player.permission < 2 {
            game.native_tell(player.uuid, horizon_client::text::Text::legacy("§cOnly operators can summon a boss."));
            return;
        }
        let seed = player.uuid as u64 ^ std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
        let mut rng = horizon_world::vastadventures::Rng::new(seed);
        let dimension = player.dimension.clone();
        if crate::game::vastadventures_client::spawn_boss(game, &dimension, player.position, None, &mut rng).is_none() {
            game.native_tell(player.uuid, horizon_client::text::Text::legacy("§cNo boss could be summoned here."));
        }
        return;
    }
    let ["vastadventures", "tier", rest @ ..] = words.as_slice() else { return };
    *ran = true;
    let tiers = horizon_world::vastadventures::TIERS;
    let line = match rest {
        [] => format!(
            "§6World tier: §e{}§7 (/vastadventures tier <{}>)",
            tiers[horizon_world::vastadventures::tier()],
            tiers.join("|")
        ),
        [name] => match horizon_world::vastadventures::tier_named(name) {
            Some(tier) => {
                horizon_world::vastadventures::set_tier(tier);
                use horizon_world::nbt::Tag;
                let data = Tag::compound([("tier", Tag::String(tiers[tier].into()))]);
                let tag = Tag::compound([("data", data), ("DataVersion", Tag::Int(horizon_core::version::DATA_VERSION))]);
                let path = game.world_root.join("data/vastadventures/world_tier.dat");
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                match horizon_client::save::write_dat(&path, tag) {
                    Ok(()) => format!("§6World tier is now §e{}§6: new loot rolls at it.", tiers[tier]),
                    Err(e) => format!("§cWorld tier set to {} but not saved: {e}", tiers[tier]),
                }
            }
            None => format!("§cNo tier {name}: {}", tiers.join(", ")),
        },
        _ => format!("§cUsage: /vastadventures tier [{}]", tiers.join("|")),
    };
    game.native_tell(player.uuid, horizon_client::text::Text::legacy(&line));
}
