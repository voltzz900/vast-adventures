// This mod's loot functions: rolling a chest's affix item, and turning dropped gear into affix loot.
fn apotheosis_loot_function(
    kind: &str,
    data: &serde_json::Value,
    ctx: &crate::loot::NativeLootContext<'_>,
    stack: &mut crate::loot::LootStack,
) {
    if !kind.starts_with("apotheosis:") {
        return;
    }
    // Apotheosis rolls only for a player: the one who opened the chest or killed the mob.
    // A player opened the chest or made the kill. The items engine (`/loot`, some drops) names no entity at
    // all, so only a roll that names a mob and no player killer is left out.
    let has_player = ctx.player.is_some() || ctx.entity.is_none_or(|e| e.kind == "minecraft:player");
    crate::apotheosis::note_context(kind, ctx.player.is_some(), ctx.entity.map(|e| e.kind.as_str()));
    let mut view = crate::apotheosis::Stack {
        item: &mut stack.item,
        count: &mut stack.count,
        names: &mut stack.names,
        components: &mut stack.components,
    };
    let dimension = ctx.place.as_ref().and_then(|p| p.level.as_ref()).map(|l| l.dimension.as_str());
    crate::apotheosis::loot_function(kind, data, has_player, ctx.luck, ctx.seed, dimension, &mut view);
}
