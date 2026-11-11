// A bow let go: its effects for the arrow, and the player's draw speed.
fn apotheosis_arrow_loose(stack: &ItemStack, charge: &mut i32) {
    crate::game::apotheosis_client::arrow_loose(stack, charge);
}
