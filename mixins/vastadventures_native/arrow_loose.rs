// A bow let go: its effects for the arrow, and the player's draw speed.
fn vastadventures_arrow_loose(stack: &ItemStack, charge: &mut i32) {
    crate::game::vastadventures_client::arrow_loose(stack, charge);
}
