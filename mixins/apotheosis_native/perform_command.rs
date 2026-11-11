// /apotheosis tier [haven|frontier|ascent|summit|pinnacle]. The tier is the world's, saved with it.
fn apotheosis_perform_command(
    game: &mut crate::game::Game<'_>,
    player: &crate::game::native_server::NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let words: Vec<&str> = command.split_whitespace().collect();
    let ["apotheosis", "tier", rest @ ..] = words.as_slice() else { return };
    *ran = true;
    let tiers = horizon_world::apotheosis::TIERS;
    let line = match rest {
        [] => format!(
            "§6World tier: §e{}§7 (/apotheosis tier <{}>)",
            tiers[horizon_world::apotheosis::tier()],
            tiers.join("|")
        ),
        [name] => match horizon_world::apotheosis::tier_named(name) {
            Some(tier) => {
                horizon_world::apotheosis::set_tier(tier);
                use horizon_world::nbt::Tag;
                let data = Tag::compound([("tier", Tag::String(tiers[tier].into()))]);
                let tag = Tag::compound([("data", data), ("DataVersion", Tag::Int(horizon_core::version::DATA_VERSION))]);
                let path = game.world_root.join("data/apotheosis/world_tier.dat");
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
        _ => format!("§cUsage: /apotheosis tier [{}]", tiers.join("|")),
    };
    game.native_tell(player.uuid, horizon_client::text::Text::legacy(&line));
}
