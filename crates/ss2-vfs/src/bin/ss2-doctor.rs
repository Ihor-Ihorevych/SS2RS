//! `ss2-doctor`: checks a Serious Sam 2 install folder and says what is wrong with it (T11-013),
//! and, with `--mod`, the mods in it (T13-009); `--list-mods` and `--load-order` show which mods
//! the install has and how a load order of them sits on the base game (T13-001).
//!
//! Usage: `ss2-doctor [FOLDER] [--mod NAME]... [--mod-dir DIR]... [--all-mods] [--list-mods]
//! [--load-order]`. Without a folder
//! the `SS2_DATA_DIR` variable is used. The report goes to the standard output; the exit code is
//! 0 when the install (and the mods) are usable (warnings are allowed), 1 when there are errors
//! and 2 for a wrong command line.

use std::path::PathBuf;
use std::process::ExitCode;

use ss2_vfs::{
    ModSpec, check_install, check_install_mods, installed_mod_names, overlay_report,
    scan_installed_mods,
};

const USAGE: &str = "Usage: ss2-doctor [FOLDER] [--mod NAME]... [--mod-dir DIR]... [--all-mods]
                  [--list-mods] [--load-order]

Checks the folder that holds the game's .gro archives (All_PC_01.gro and the others) and
reports what is missing or damaged. Without FOLDER the SS2_DATA_DIR variable is used.

Mod checks (the lints and codes: docs/MOD_VALIDATION.md), after the install check:
  --mod NAME     check the mod in Content/NAME/ of the install (repeat for several: the order
                 given is the load order, and files two mods share are reported)
  --mod-dir DIR  check a loose mod folder (Content/<Mod>.mod and the files below Content/)
  --all-mods     check every mod folder of the install

Mod loading (docs/MOD_LOADING.md):
  --list-mods    list the mods of the install: name, archives, native module, what each needs
  --load-order   show the load order of the --mod names (the selected mod when there is none;
                 what a mod needs is added first) and which files each mod adds or replaces,
                 and who wins where two mods provide the same file

Options:
  --help      show this text
  --version   show the version

Exit code: 0 usable (warnings allowed), 1 errors found, 2 wrong command line.
";

fn main() -> ExitCode {
    let mut folder: Option<String> = None;
    let mut specs: Vec<ModSpec> = Vec::new();
    let mut all_mods = false;
    let mut list_mods = false;
    let mut load_order = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "--version" | "-V" => {
                println!("ss2-doctor {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "--all-mods" => all_mods = true,
            "--list-mods" => list_mods = true,
            "--load-order" => load_order = true,
            "--mod" | "--mod-dir" => {
                let Some(value) = args.next().filter(|v| !v.is_empty()) else {
                    eprintln!(
                        "ss2-doctor: {arg} needs a value

{USAGE}"
                    );
                    return ExitCode::from(2);
                };
                specs.push(if arg == "--mod" {
                    ModSpec::Installed(value)
                } else {
                    ModSpec::Folder(PathBuf::from(value))
                });
            }
            other if other.starts_with('-') => {
                eprintln!(
                    "ss2-doctor: unknown option {other}

{USAGE}"
                );
                return ExitCode::from(2);
            }
            _ if folder.is_some() => {
                eprintln!(
                    "ss2-doctor: only one folder can be checked

{USAGE}"
                );
                return ExitCode::from(2);
            }
            _ => folder = Some(arg),
        }
    }
    let Some(folder) =
        folder.or_else(|| std::env::var("SS2_DATA_DIR").ok().filter(|v| !v.is_empty()))
    else {
        eprintln!(
            "ss2-doctor: give the folder to check, or set SS2_DATA_DIR

{USAGE}"
        );
        return ExitCode::from(2);
    };
    let report = check_install(&folder);
    print!("{}", report.render());
    let mut code = report.exit_code();
    if all_mods {
        let known: Vec<String> = specs
            .iter()
            .filter_map(|s| match s {
                ModSpec::Installed(name) => Some(name.to_lowercase()),
                ModSpec::Folder(_) => None,
            })
            .collect();
        specs.extend(
            installed_mod_names(&folder)
                .into_iter()
                .filter(|n| !known.contains(&n.to_lowercase()))
                .map(ModSpec::Installed),
        );
    }
    if !specs.is_empty() {
        let mods = check_install_mods(&folder, &specs);
        println!();
        print!("{}", mods.render());
        code = code.max(mods.exit_code());
    } else if all_mods {
        println!(
            "
No mod folders (Content/<Mod>/ with .gro archives) in the install."
        );
    }
    if list_mods || load_order {
        code = code.max(mod_loading(&folder, &specs, list_mods, load_order));
    }
    if code == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// `--list-mods` and `--load-order`; returns the exit code they ask for (1 when the load order
/// cannot be made or the install cannot be read).
fn mod_loading(folder: &str, specs: &[ModSpec], list_mods: bool, load_order: bool) -> i32 {
    let found = match scan_installed_mods(folder) {
        Ok(found) => found,
        Err(error) => {
            println!(
                "
The mods of the install cannot be listed: {error}"
            );
            return 1;
        }
    };
    if list_mods {
        println!();
        print!("{}", found.render());
    }
    if !load_order {
        return 0;
    }
    let mut requested: Vec<String> = specs
        .iter()
        .filter_map(|s| match s {
            ModSpec::Installed(name) => Some(name.clone()),
            ModSpec::Folder(_) => None,
        })
        .collect();
    if requested.is_empty() {
        requested = found.default_request();
    }
    println!();
    let plan = match found.plan(&requested) {
        Ok(plan) => plan,
        Err(error) => {
            println!("The load order cannot be made: {error}");
            return 1;
        }
    };
    print!("{}", plan.render());
    for planned in &plan.mods {
        if let Some(m) = found.get(&planned.id)
            && !m.is_data_only()
        {
            println!(
                "Note: {} names the native module {}, which SS2RS cannot load; only its content is used.",
                planned.id,
                m.native_modules().join(", ")
            );
        }
    }
    match overlay_report(folder, &found, &plan) {
        Ok(report) => {
            if !plan.mods.is_empty() {
                print!("{}", report.render());
            }
            0
        }
        Err(error) => {
            println!("The mods cannot be mounted: {error}");
            1
        }
    }
}
