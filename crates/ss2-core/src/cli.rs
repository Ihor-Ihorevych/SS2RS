//! Launch options shared by `ss2-app` and `ss2-server` (T03-020).
//!
//! One small parser for the options both programs understand, so they behave the same and are
//! tested once. The app adds its own developer modes (`--screenshot`, `--smoke-frames`) on top.
//!
//! ```text
//! ss2-app    [--connect HOST[:PORT] | --listen] [--level NAME] [--data DIR] [--portable] [--set KEY=VALUE]...
//! ss2-server [--bind ADDR] [--level NAME] [--data DIR] [--portable] [--set KEY=VALUE]...
//! ```
//!
//! Options are `--name value` or `--name=value`; `--help`/`-h` and `--version`/`-V` stop parsing
//! and are reported as [`Parsed::Help`] and [`Parsed::Version`].

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::{Error, Result};

/// Which program is parsing; it decides which options exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// `ss2-app`: the windowed client (can join with `--connect` or host with `--listen`).
    Client,
    /// `ss2-server`: the headless dedicated server (no window, GPU or audio).
    Server,
}

/// The options both programs share.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LaunchOptions {
    /// `--data DIR`: the folder with the user's own Serious Sam 2 install.
    pub data_dir: Option<PathBuf>,
    /// `--level NAME`: start directly in this level.
    pub level: Option<String>,
    /// `--connect HOST[:PORT]` (client): join a server.
    pub connect: Option<String>,
    /// `--listen` (client): host a game from this client.
    pub listen: bool,
    /// `--bind ADDR` (server): the address to listen on.
    pub bind: Option<String>,
    /// `--portable`: keep all files next to the executable.
    pub portable: bool,
    /// `--set KEY=VALUE` (repeatable, in order): overrides for settings, applied with
    /// [`Settings::apply_overrides`](crate::settings::Settings::apply_overrides).
    pub set: Vec<String>,
}

/// The outcome of parsing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parsed {
    /// Run with these options.
    Run(Box<LaunchOptions>),
    /// Print the usage text and exit successfully.
    Help,
    /// Print the version and exit successfully.
    Version,
}

fn invalid(detail: impl Into<String>) -> Error {
    Error::invalid("command line", detail)
}

/// The usage text for `program` (for `--help` and for error messages).
pub fn usage(role: Role, program: &str) -> String {
    let (synopsis, specific) = match role {
        Role::Client => (
            "[--connect HOST[:PORT] | --listen] [--level NAME] [--data DIR] [--portable] [--set KEY=VALUE]...",
            "  --connect HOST[:PORT]  join a server (IPv6 in brackets: [::1]:27960)\n  \
             --listen               host a game from this client\n",
        ),
        Role::Server => (
            "[--bind ADDR] [--level NAME] [--data DIR] [--portable] [--set KEY=VALUE]...",
            "  --bind ADDR            address to listen on\n",
        ),
    };
    format!(
        "usage: {program} {synopsis}\n\n\
         {specific}  \
         --level NAME           start directly in this level\n  \
         --data DIR             folder with your own Serious Sam 2 install\n  \
         --portable             keep all files next to the executable\n  \
         --set KEY=VALUE        override a setting (repeatable), e.g. --set vsync=off\n  \
         -h, --help             show this text\n  \
         -V, --version          show the version\n"
    )
}

/// Parse the arguments after the program name.
///
/// # Errors
/// [`Error::Invalid`] naming the offending option: an unknown argument, a missing, empty or
/// malformed value, an option given twice, or `--connect` together with `--listen`.
pub fn parse_launch_options<I: IntoIterator<Item = String>>(role: Role, args: I) -> Result<Parsed> {
    let mut options = LaunchOptions::default();
    let mut seen: BTreeSet<&'static str> = BTreeSet::new();
    let mut args = args.into_iter().peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Parsed::Help),
            "--version" | "-V" => return Ok(Parsed::Version),
            _ => {}
        }
        let (name, attached) = match arg.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value.to_owned())),
            _ => (arg.as_str(), None),
        };
        let flag: &'static str = match (name, role) {
            ("--portable", _) => "--portable",
            ("--listen", Role::Client) => "--listen",
            ("--data", _) => "--data",
            ("--level", _) => "--level",
            ("--connect", Role::Client) => "--connect",
            ("--bind", Role::Server) => "--bind",
            ("--set", _) => "--set",
            _ => return Err(unknown(&arg, role)),
        };
        // `--set` may be repeated; every other option may be given once.
        if flag != "--set" && !seen.insert(flag) {
            return Err(invalid(format!("{flag} was given twice")));
        }
        if matches!(flag, "--portable" | "--listen") {
            if attached.is_some() {
                return Err(invalid(format!("{flag} takes no value")));
            }
            if flag == "--portable" {
                options.portable = true;
            } else {
                options.listen = true;
            }
            continue;
        }
        let value = match attached {
            Some(value) => value,
            None => match args.next_if(|next| !next.starts_with("--")) {
                Some(value) => value,
                None => return Err(invalid(format!("{flag} needs a value"))),
            },
        };
        if value.is_empty() {
            return Err(invalid(format!("{flag} needs a non-empty value")));
        }
        match flag {
            "--data" => options.data_dir = Some(PathBuf::from(value)),
            "--level" => options.level = Some(value),
            "--set" => {
                match value.split_once('=') {
                    Some((key, _)) if !key.trim().is_empty() => options.set.push(value),
                    _ => {
                        return Err(invalid(format!(
                            "--set {value:?} is not KEY=VALUE (for example --set vsync=off)"
                        )));
                    }
                };
            }
            "--connect" => {
                validate_endpoint(&value)
                    .map_err(|why| invalid(format!("--connect {value}: {why}")))?;
                options.connect = Some(value);
            }
            _ => options.bind = Some(value),
        }
    }
    if options.connect.is_some() && options.listen {
        return Err(invalid("--connect and --listen cannot be combined"));
    }
    Ok(Parsed::Run(Box::new(options)))
}

fn unknown(arg: &str, role: Role) -> Error {
    let program = match role {
        Role::Client => "ss2-app",
        Role::Server => "ss2-server",
    };
    invalid(format!(
        "unknown argument '{arg}' for {program}; try --help"
    ))
}

/// `HOST`, `HOST:PORT`, `[IPV6]` or `[IPV6]:PORT` with a port from 1 to 65535.
fn validate_endpoint(text: &str) -> std::result::Result<(), String> {
    fn port(text: &str) -> std::result::Result<(), String> {
        match text.parse::<u16>() {
            Ok(p) if p != 0 => Ok(()),
            _ => Err(format!("{text:?} is not a port from 1 to 65535")),
        }
    }
    if text.is_empty() || text.contains(char::is_whitespace) {
        return Err("expected HOST or HOST:PORT without spaces".into());
    }
    if let Some(rest) = text.strip_prefix('[') {
        let (host, after) = rest
            .split_once(']')
            .ok_or_else(|| "missing the closing ']' of the IPv6 address".to_owned())?;
        if host.is_empty() {
            return Err("empty IPv6 address".into());
        }
        return match after.strip_prefix(':') {
            _ if after.is_empty() => Ok(()),
            Some(p) => port(p),
            None => Err("expected ':PORT' after the ']'".into()),
        };
    }
    match text.matches(':').count() {
        0 => Ok(()),
        1 => {
            let (host, p) = text.split_once(':').unwrap_or((text, ""));
            if host.is_empty() {
                return Err("the host is empty".into());
            }
            port(p)
        }
        _ => Err("wrap an IPv6 address in brackets, like [::1]:27960".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn parse(role: Role, args: &[&str]) -> Result<Parsed> {
        parse_launch_options(role, args.iter().map(|s| (*s).to_owned()))
    }

    fn run(role: Role, args: &[&str]) -> LaunchOptions {
        match parse(role, args).unwrap() {
            Parsed::Run(o) => *o,
            other => panic!("expected Run, got {other:?}"),
        }
    }

    fn err(role: Role, args: &[&str]) -> String {
        parse(role, args).unwrap_err().to_string()
    }

    #[test]
    fn no_arguments_give_the_defaults() {
        assert_eq!(run(Role::Client, &[]), LaunchOptions::default());
        assert_eq!(run(Role::Server, &[]), LaunchOptions::default());
    }

    #[test]
    fn every_client_option_is_read() {
        let o = run(
            Role::Client,
            &[
                "--data",
                "D:/Games/SS2",
                "--level",
                "1_1_jungle",
                "--portable",
            ],
        );
        assert_eq!(o.data_dir, Some(PathBuf::from("D:/Games/SS2")));
        assert_eq!(o.level.as_deref(), Some("1_1_jungle"));
        assert!(o.portable && !o.listen && o.connect.is_none() && o.bind.is_none());

        let o = run(Role::Client, &["--listen"]);
        assert!(o.listen);
        let o = run(Role::Client, &["--connect", "example.org:27960"]);
        assert_eq!(o.connect.as_deref(), Some("example.org:27960"));
    }

    #[test]
    fn the_equals_form_works_like_the_separate_form() {
        let a = run(Role::Client, &["--data=C:/SS2", "--level=dm_stadium"]);
        let b = run(Role::Client, &["--data", "C:/SS2", "--level", "dm_stadium"]);
        assert_eq!(a, b);
    }

    #[test]
    fn help_and_version_are_recognised() {
        for flag in ["--help", "-h"] {
            assert!(matches!(parse(Role::Client, &[flag]), Ok(Parsed::Help)));
        }
        for flag in ["--version", "-V"] {
            assert!(matches!(parse(Role::Server, &[flag]), Ok(Parsed::Version)));
        }
    }

    #[test]
    fn unknown_arguments_are_errors_that_point_to_help() {
        for args in [&["--bogus"][..], &["stray"], &["--data", "d", "extra"]] {
            let m = err(Role::Client, args);
            assert!(m.contains("--help"), "{m}");
        }
        assert!(err(Role::Client, &["--bogus"]).contains("--bogus"));
    }

    #[test]
    fn a_value_flag_without_a_value_names_the_flag() {
        for flag in ["--data", "--level", "--connect"] {
            let m = err(Role::Client, &[flag]);
            assert!(m.contains(flag) && m.contains("needs a value"), "{m}");
        }
        // the next word is another flag: the value is missing, not "--level"
        let m = err(Role::Client, &["--data", "--level", "x"]);
        assert!(m.contains("--data") && m.contains("needs a value"), "{m}");
    }

    #[test]
    fn empty_and_repeated_options_are_errors() {
        assert!(err(Role::Client, &["--level="]).contains("--level"));
        let m = err(Role::Client, &["--data", "a", "--data", "b"]);
        assert!(m.contains("--data") && m.contains("twice"), "{m}");
        assert!(err(Role::Client, &["--portable", "--portable"]).contains("twice"));
    }

    #[test]
    fn connecting_and_listening_cannot_be_combined() {
        let m = err(Role::Client, &["--connect", "host", "--listen"]);
        assert!(m.contains("--connect") && m.contains("--listen"), "{m}");
    }

    #[test]
    fn connect_accepts_hosts_ports_and_bracketed_ipv6() {
        for ok in ["host", "host:27960", "127.0.0.1:1", "[::1]:27960", "[::1]"] {
            assert!(parse(Role::Client, &["--connect", ok]).is_ok(), "{ok}");
        }
        for bad in [
            ":27960",
            "host:0",
            "host:65536",
            "host:abc",
            "::1",
            "a b",
            "[::1",
        ] {
            let m = err(Role::Client, &["--connect", bad]);
            assert!(m.contains("--connect"), "{bad}: {m}");
        }
    }

    #[test]
    fn options_belong_to_their_role() {
        let o = run(
            Role::Server,
            &["--bind", "0.0.0.0:27960", "--level", "dm_x"],
        );
        assert_eq!(o.bind.as_deref(), Some("0.0.0.0:27960"));
        assert!(err(Role::Server, &["--connect", "h"]).contains("--connect"));
        assert!(err(Role::Server, &["--listen"]).contains("--listen"));
        assert!(err(Role::Client, &["--bind", "x"]).contains("--bind"));
    }

    #[test]
    fn set_overrides_are_collected_in_order_and_may_repeat() {
        for role in [Role::Client, Role::Server] {
            let o = run(role, &["--set", "vsync=off", "--set=master_volume=0.5"]);
            assert_eq!(o.set, ["vsync=off", "master_volume=0.5"]);
        }
    }

    #[test]
    fn set_needs_a_key_value_pair() {
        for args in [&["--set", "vsync"][..], &["--set", "=1"], &["--set="]] {
            let m = err(Role::Client, args);
            assert!(m.contains("--set"), "{m}");
        }
        let m = err(Role::Client, &["--set", "vsync"]);
        assert!(m.contains("KEY=VALUE"), "{m}");
        assert!(err(Role::Client, &["--set"]).contains("needs a value"));
    }

    #[test]
    fn set_values_feed_the_settings_overrides() {
        let o = run(
            Role::Client,
            &["--set", "vsync=off", "--set", "fullscreen=1"],
        );
        let mut settings = crate::settings::Settings::default();
        settings.apply_overrides(&o.set).unwrap();
        assert!(!settings.vsync && settings.fullscreen);
    }

    #[test]
    fn usage_lists_the_options_of_the_role() {
        let client = usage(Role::Client, "ss2-app");
        for needle in [
            "ss2-app",
            "--connect",
            "--listen",
            "--level",
            "--data",
            "--portable",
            "--set",
            "--help",
        ] {
            assert!(client.contains(needle), "{needle}\n{client}");
        }
        assert!(!client.contains("--bind"));
        let server = usage(Role::Server, "ss2-server");
        assert!(server.contains("--bind") && !server.contains("--connect"));
        assert!(server.contains("--set"));
    }
}
