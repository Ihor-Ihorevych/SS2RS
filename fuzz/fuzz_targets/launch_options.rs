//! Any command line must be parsed or refused with a message, never a panic.
#![no_main]

use libfuzzer_sys::fuzz_target;
use ss2_core::cli::{Role, parse_launch_options};

fuzz_target!(|data: &str| {
    let args: Vec<String> = data.split(['\0', '\n']).map(str::to_owned).collect();
    for role in [Role::Client, Role::Server] {
        let _ = parse_launch_options(role, args.clone());
    }
    let spaced: Vec<String> = data.split_whitespace().map(str::to_owned).collect();
    let _ = parse_launch_options(Role::Client, spaced);
});
