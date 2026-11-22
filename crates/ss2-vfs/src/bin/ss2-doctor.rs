//! `ss2-doctor`: checks a Serious Sam 2 install folder and says what is wrong with it (T11-013).
//!
//! Usage: `ss2-doctor [FOLDER]`. Without a folder the `SS2_DATA_DIR` variable is used. The report
//! goes to the standard output; the exit code is 0 when the install is usable (warnings are
//! allowed), 1 when it has errors and 2 for a wrong command line.

use std::process::ExitCode;

const USAGE: &str = "Usage: ss2-doctor [FOLDER]\n\n\
Checks the folder that holds the game's .gro archives (All_PC_01.gro and the others) and\n\
reports what is missing or damaged. Without FOLDER the SS2_DATA_DIR variable is used.\n\n\
Options:\n  --help      show this text\n  --version   show the version\n\n\
Exit code: 0 usable (warnings allowed), 1 errors found, 2 wrong command line.\n";

fn main() -> ExitCode {
    let mut folder: Option<String> = None;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--help" | "-h" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "--version" | "-V" => {
                println!("ss2-doctor {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            other if other.starts_with('-') => {
                eprintln!("ss2-doctor: unknown option {other}\n\n{USAGE}");
                return ExitCode::from(2);
            }
            _ if folder.is_some() => {
                eprintln!("ss2-doctor: only one folder can be checked\n\n{USAGE}");
                return ExitCode::from(2);
            }
            _ => folder = Some(arg),
        }
    }
    let Some(folder) =
        folder.or_else(|| std::env::var("SS2_DATA_DIR").ok().filter(|v| !v.is_empty()))
    else {
        eprintln!("ss2-doctor: give the folder to check, or set SS2_DATA_DIR\n\n{USAGE}");
        return ExitCode::from(2);
    };
    let report = ss2_vfs::check_install(&folder);
    print!("{}", report.render());
    if report.exit_code() == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
