// /apotheosis tier: shows the world tier; /apotheosis tier <tier>: sets it; /apotheosis boss: a boss here (operators).
fn apotheosis_register_commands(commands: &mut Vec<crate::game::native_server::NativeCommand>) {
    commands.push(crate::game::native_server::NativeCommand::new("apotheosis tier"));
    commands.push(crate::game::native_server::NativeCommand::new("apotheosis tier <tier:word>"));
    commands.push(crate::game::native_server::NativeCommand::op("apotheosis boss", 2));
}
