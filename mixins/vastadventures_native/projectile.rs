// A projectile hitting something: the player's bow effects on an arrow hit.
fn vastadventures_projectile(game: &mut crate::game::Game<'_>, kind: &str, owner: u32, entity: Option<u32>) {
    crate::game::vastadventures_client::projectile(game, kind, owner, entity);
}
