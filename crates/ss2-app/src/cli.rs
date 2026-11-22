//! Command-line parsing for `ss2-app`.

use ss2_core::cli::{LaunchOptions, Parsed, Role, parse_launch_options};
use ss2_ui::ScreenId;
use std::path::PathBuf;

const USAGE: &str = "usage:\n  ss2-app [launch options]   (see --help)\n  ss2-app --screenshot <main|options|pause|about> <out.png> [--size WxH]\n  ss2-app --smoke-frames <N>";

/// What `ss2-app` was asked to do.
#[derive(Debug, PartialEq)]
pub enum Mode {
    /// Open the window and run interactively with these launch options.
    Window(LaunchOptions),
    /// Print the usage text and exit.
    Help,
    /// Print the version and exit.
    Version,
    /// Render one menu screen offscreen to a PNG and exit.
    Screenshot {
        screen: ScreenId,
        out: PathBuf,
        size: (u32, u32),
    },
    /// Open the window, render this many frames, then exit (automated smoke test).
    Smoke { frames: u32 },
}

fn parse_size(text: &str) -> Result<(u32, u32), String> {
    let bad = || format!("invalid size '{text}': expected WxH with both positive, e.g. 1280x720");
    let (w, h) = text.split_once('x').ok_or_else(bad)?;
    let (w, h) = (
        w.parse::<u32>().map_err(|_| bad())?,
        h.parse::<u32>().map_err(|_| bad())?,
    );
    if w == 0 || h == 0 {
        return Err(bad());
    }
    Ok((w, h))
}

/// Parse the arguments after the program name.
pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Mode, String> {
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Ok(Mode::Window(LaunchOptions::default()));
    };
    match first.as_str() {
        "--screenshot" => {
            let name = args
                .next()
                .ok_or_else(|| format!("--screenshot needs a screen and a path\n{USAGE}"))?;
            let screen = match name.as_str() {
                "main" => ScreenId::Main,
                "options" => ScreenId::Options,
                "pause" => ScreenId::Pause,
                "about" => ScreenId::About,
                other => {
                    return Err(format!(
                        "unknown screen '{other}' (expected main, options, pause or about)"
                    ));
                }
            };
            let out = PathBuf::from(
                args.next()
                    .ok_or_else(|| format!("--screenshot needs an output path\n{USAGE}"))?,
            );
            let mut size = (1280, 720);
            while let Some(flag) = args.next() {
                match flag.as_str() {
                    "--size" => {
                        size =
                            parse_size(&args.next().ok_or("--size needs a value like 1280x720")?)?;
                    }
                    other => return Err(format!("unknown argument '{other}'\n{USAGE}")),
                }
            }
            Ok(Mode::Screenshot { screen, out, size })
        }
        "--smoke-frames" => {
            let value = args
                .next()
                .ok_or_else(|| format!("--smoke-frames needs a count\n{USAGE}"))?;
            let frames: u32 = value
                .parse()
                .map_err(|_| format!("invalid frame count '{value}'"))?;
            if frames == 0 {
                return Err("--smoke-frames must be at least 1".to_string());
            }
            if let Some(extra) = args.next() {
                return Err(format!("unexpected argument '{extra}'\n{USAGE}"));
            }
            Ok(Mode::Smoke { frames })
        }
        _ => {
            // Everything else is a launch option shared with ss2-server.
            let all = std::iter::once(first).chain(args);
            match parse_launch_options(Role::Client, all).map_err(|e| e.to_string())? {
                Parsed::Run(options) => Ok(Mode::Window(*options)),
                Parsed::Help => Ok(Mode::Help),
                Parsed::Version => Ok(Mode::Version),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_ui::ScreenId;
    use std::path::PathBuf;

    fn parse(args: &[&str]) -> Result<Mode, String> {
        parse_args(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn no_args_is_window() {
        assert_eq!(parse(&[]), Ok(Mode::Window(LaunchOptions::default())));
    }

    #[test]
    fn launch_options_open_the_window_with_those_options() {
        let Ok(Mode::Window(o)) = parse(&["--level", "1_1_jungle", "--data=C:/SS2", "--portable"])
        else {
            panic!("expected a window mode");
        };
        assert_eq!(o.level.as_deref(), Some("1_1_jungle"));
        assert_eq!(o.data_dir, Some(PathBuf::from("C:/SS2")));
        assert!(o.portable);
        let Ok(Mode::Window(o)) = parse(&["--connect", "example.org:27960"]) else {
            panic!("expected a window mode");
        };
        assert_eq!(o.connect.as_deref(), Some("example.org:27960"));
        assert!(parse(&["--listen"]).is_ok());
    }

    #[test]
    fn help_and_version_have_their_own_modes() {
        assert_eq!(parse(&["--help"]), Ok(Mode::Help));
        assert_eq!(parse(&["-V"]), Ok(Mode::Version));
    }

    #[test]
    fn bad_launch_options_are_errors_with_the_reason() {
        let e = parse(&["--connect", "h", "--listen"]).unwrap_err();
        assert!(e.contains("--connect") && e.contains("--listen"), "{e}");
        assert!(parse(&["--data"]).unwrap_err().contains("--data"));
        assert!(parse(&["--bind", "x"]).unwrap_err().contains("--bind"));
    }

    #[test]
    fn screenshot_main_with_default_size() {
        assert_eq!(
            parse(&["--screenshot", "main", "out.png"]),
            Ok(Mode::Screenshot {
                screen: ScreenId::Main,
                out: PathBuf::from("out.png"),
                size: (1280, 720)
            })
        );
    }

    #[test]
    fn screenshot_about_is_a_known_screen_and_unknown_ones_list_the_choices() {
        assert_eq!(
            parse(&["--screenshot", "about", "a.png"]),
            Ok(Mode::Screenshot {
                screen: ScreenId::About,
                out: PathBuf::from("a.png"),
                size: (1280, 720)
            })
        );
        let err = parse(&["--screenshot", "credits", "a.png"]).unwrap_err();
        assert!(err.contains("about"), "{err}");
    }

    #[test]
    fn screenshot_with_custom_size() {
        assert_eq!(
            parse(&["--screenshot", "options", "o.png", "--size", "800x600"]),
            Ok(Mode::Screenshot {
                screen: ScreenId::Options,
                out: PathBuf::from("o.png"),
                size: (800, 600)
            })
        );
    }

    #[test]
    fn screenshot_rejects_zero_and_garbage_sizes() {
        assert!(parse(&["--screenshot", "main", "o.png", "--size", "0x0"]).is_err());
        assert!(parse(&["--screenshot", "main", "o.png", "--size", "abc"]).is_err());
        assert!(parse(&["--screenshot", "main", "o.png", "--size"]).is_err());
    }

    #[test]
    fn screenshot_missing_path_is_error() {
        assert!(parse(&["--screenshot", "main"]).is_err());
        assert!(parse(&["--screenshot"]).is_err());
    }

    #[test]
    fn screenshot_unknown_screen_is_error() {
        let e = parse(&["--screenshot", "nope", "o.png"]).unwrap_err();
        assert!(e.contains("nope"), "{e}");
    }

    #[test]
    fn smoke_frames_parses_and_rejects_zero() {
        assert_eq!(
            parse(&["--smoke-frames", "30"]),
            Ok(Mode::Smoke { frames: 30 })
        );
        assert!(parse(&["--smoke-frames", "0"]).is_err());
        assert!(parse(&["--smoke-frames", "x"]).is_err());
    }

    #[test]
    fn unknown_flag_is_error() {
        assert!(parse(&["--bogus"]).is_err());
    }
}
