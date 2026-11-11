//! Apotheosis' trait mod: empty. The game asks some hook points (the player's melee hit among them) only while a
//! trait mod is loaded, and this bundle's mixin injections are there; its own work is all in the injections.
use horizon_client::mod_api::Hooks;

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Apotheosis)
}

struct Apotheosis;

impl Hooks for Apotheosis {}
