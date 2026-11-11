// Taking an anvil's result, before its inputs are used up: a Sigil of Withdrawal's gems are queued for the
// player. (menu.rs is compiled in horizon-client-lib too, so this reaches only horizon-world.)
fn apotheosis_anvil_take(menu: &Menu) {
    let (Some(left), Some(right)) = (menu.slot(0), menu.slot(1)) else { return };
    let custom = left.data().and_then(|d| d.other("minecraft:custom_data")).map(horizon_client_core::scoreboard::tag_to_json);
    horizon_world::apotheosis_gems::queue_withdrawn(&left.item, custom.as_ref(), &right.item);
}
