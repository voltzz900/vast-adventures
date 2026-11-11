// Chests may hold an affix item; gear from other tables but blocks' may turn into one.
fn vastadventures_loot_table_load(id: &str, table: &mut serde_json::Value) {
    crate::vastadventures::table_load(id, table);
}
