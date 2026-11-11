// `vastadventures:has_player`: a player opened the chest or killed the mob.
fn vastadventures_loot_condition(kind: &str, ctx: &crate::loot::NativeLootContext<'_>, holds: &mut bool) {
    // A player opened the chest or made the kill. The items engine (`/loot`, some drops) names no entity at
    // all, so only a roll that names a mob and no player killer is left out.
    let has_player = ctx.player.is_some() || ctx.entity.is_none_or(|e| e.kind == "minecraft:player");
    if let Some(answer) = crate::vastadventures::loot_condition(kind, has_player, ctx.entity.map(|e| e.kind.as_str())) {
        *holds = answer;
    }
}
