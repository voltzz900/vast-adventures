// Remember each item's default attribute modifiers and worn slot as they load.
fn apotheosis_defaults(item: &str, components: &serde_json::Map<String, serde_json::Value>) {
    crate::apotheosis::record_defaults(item, components);
}
