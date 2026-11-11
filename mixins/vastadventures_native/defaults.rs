// Remember each item's default attribute modifiers and worn slot as they load.
fn vastadventures_defaults(item: &str, components: &serde_json::Map<String, serde_json::Value>) {
    crate::vastadventures::record_defaults(item, components);
}
