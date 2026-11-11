// Chests may hold an affix item; gear from other tables but blocks' may turn into one.
fn apotheosis_loot_table_load(id: &str, table: &mut serde_json::Value) {
    crate::apotheosis::table_load(id, table);
}
