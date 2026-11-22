//! Any console line must be answered (a value or an error), never a panic.
#![no_main]

use libfuzzer_sys::fuzz_target;
use ss2_core::console::{CVarSpec, CVarValue, Console};
use ss2_game::cheats::Cheats;

fuzz_target!(|data: &str| {
    let mut console = Console::new();
    let _ = console.register_cvar(
        CVarSpec::new("fov", CVarValue::Float(90.0), "field of view").range(30.0, 150.0),
    );
    let _ = console.register_cvar(CVarSpec::new(
        "name",
        CVarValue::Str("player".into()),
        "player name",
    ));
    let _ = console.register_cvar(CVarSpec::new(
        "fullscreen",
        CVarValue::Bool(false),
        "fullscreen",
    ));
    let _ = console
        .register_cvar(CVarSpec::new("skill", CVarValue::Int(1), "difficulty").range(0.0, 3.0));
    let _cheats = Cheats::register(&mut console);
    let _ = console.execute("sv_cheats 1");
    for line in data.lines().take(8) {
        let _ = console.execute(line);
    }
    let _ = console.execute(data);
    let _ = console.complete(data);
    let _ = ss2_core::console::tokenize(data);
});
