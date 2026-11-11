// The injected modules, built as an ordinary crate so `cargo test` runs their tests. In the game they are
// `horizon_world::apotheosis` and `horizon_world::apotheosis_data`.
#[path = "../mixins/apotheosis_native/runtime.rs"]
pub mod apotheosis;
#[path = "../mixins/apotheosis_native/data.rs"]
pub mod apotheosis_data;
#[path = "../mixins/apotheosis_native/gems.rs"]
pub mod apotheosis_gems;
#[path = "../mixins/apotheosis_native/bosses.rs"]
pub mod apotheosis_bosses;
