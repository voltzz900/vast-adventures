// /vastadventures tier: shows the world tier; /vastadventures tier <tier>: sets it; /vastadventures boss: a boss here (operators).
fn vastadventures_register_commands(commands: &mut Vec<crate::game::native_server::NativeCommand>) {
    commands.push(crate::game::native_server::NativeCommand::new("vastadventures tier"));
    commands.push(crate::game::native_server::NativeCommand::new("vastadventures tier <tier:word>"));
    commands.push(crate::game::native_server::NativeCommand::op("vastadventures boss", 2));
}
