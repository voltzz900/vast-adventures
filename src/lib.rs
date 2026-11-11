// The injected modules, built as an ordinary crate so `cargo test` runs their tests. In the game they are
// `horizon_world::vastadventures` and `horizon_world::vastadventures_data`.
#[path = "../mixins/vastadventures_native/runtime.rs"]
pub mod vastadventures;
#[path = "../mixins/vastadventures_native/data.rs"]
pub mod vastadventures_data;
#[path = "../mixins/vastadventures_native/gems.rs"]
pub mod vastadventures_gems;
#[path = "../mixins/vastadventures_native/bosses.rs"]
pub mod vastadventures_bosses;
#[path = "../mixins/vastadventures_native/effects.rs"]
pub mod vastadventures_effects;
