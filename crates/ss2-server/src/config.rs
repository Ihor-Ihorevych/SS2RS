//! The dedicated server's configuration file (roadmap T08-020).
//!
//! A TOML file with every key optional; a missing key takes its default and an **unknown key is an
//! error** (a typo such as `max_player = 4` must not silently leave the limit at 8). Values are
//! checked, and every problem is reported at once. The command line wins over the file
//! ([`ServerSettings::apply_launch_options`]). The session password is never printed: `Debug`
//! hides it.

use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;

use serde::Deserialize;
use ss2_core::cli::LaunchOptions;

/// A configuration file that could not be read.
#[derive(Debug)]
pub struct ConfigError(String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ConfigError {}

/// Everything an operator can set.
#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerSettings {
    /// The server's name, shown in browsers and logs (1 to 64 characters).
    pub name: String,
    /// The address and port to listen on: an IP address, not a host name.
    pub bind: String,
    /// How many players at most (1 to 64).
    pub max_players: u8,
    /// The session password; `None` for an open server.
    pub password: Option<String>,
    /// Simulation ticks per second (1 to 1000).
    pub tick_hz: u32,
    /// The level to start in.
    pub level: Option<String>,
    /// The folder with the Serious Sam 2 install.
    pub data_dir: Option<PathBuf>,
    /// Where to write the rotating log file; `None` logs to the terminal only.
    pub log_dir: Option<PathBuf>,
    /// Where to serve metrics (`127.0.0.1:9100`); `None` serves none.
    pub metrics_bind: Option<String>,
    /// Read commands from standard input.
    pub console: bool,
    /// A message shown to players who join (up to 200 characters).
    pub motd: String,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            name: "SS2RS Server".into(),
            bind: "0.0.0.0:27960".into(),
            max_players: 8,
            password: None,
            tick_hz: 60,
            level: None,
            data_dir: None,
            log_dir: None,
            metrics_bind: None,
            console: true,
            motd: String::new(),
        }
    }
}

impl fmt::Debug for ServerSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerSettings")
            .field("name", &self.name)
            .field("bind", &self.bind)
            .field("max_players", &self.max_players)
            .field("password", &self.password.as_ref().map(|_| "<set>"))
            .field("tick_hz", &self.tick_hz)
            .field("level", &self.level)
            .field("data_dir", &self.data_dir)
            .field("log_dir", &self.log_dir)
            .field("metrics_bind", &self.metrics_bind)
            .field("console", &self.console)
            .field("motd", &self.motd)
            .finish()
    }
}

impl ServerSettings {
    /// Read a configuration file's text.
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        toml::from_str(text).map_err(|e| ConfigError(e.to_string()))
    }

    /// Let the command line override the file: `--bind`, `--level` and `--data`.
    pub fn apply_launch_options(&mut self, options: &LaunchOptions) {
        if let Some(bind) = &options.bind {
            self.bind = bind.clone();
        }
        if let Some(level) = &options.level {
            self.level = Some(level.clone());
        }
        if let Some(dir) = &options.data_dir {
            self.data_dir = Some(dir.clone());
        }
    }

    /// Every problem with the values, as sentences; empty when the settings are usable.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.name.is_empty()
            || self.name.chars().count() > 64
            || self.name.chars().any(char::is_control)
        {
            problems.push("name must be 1 to 64 characters without control characters".to_owned());
        }
        if self.bind.parse::<SocketAddr>().is_err() {
            problems.push(format!(
                "bind {:?} is not an IP address and port such as 0.0.0.0:27960",
                self.bind
            ));
        }
        if !(1..=64).contains(&self.max_players) {
            problems.push(format!(
                "max_players {} is not from 1 to 64",
                self.max_players
            ));
        }
        if !(1..=1000).contains(&self.tick_hz) {
            problems.push(format!("tick_hz {} is not from 1 to 1000", self.tick_hz));
        }
        if let Some(password) = &self.password
            && (password.is_empty() || password.len() > 128)
        {
            problems.push(
                "password must be 1 to 128 bytes (leave it out for an open server)".to_owned(),
            );
        }
        if let Some(addr) = &self.metrics_bind
            && addr.parse::<SocketAddr>().is_err()
        {
            problems.push(format!(
                "metrics_bind {addr:?} is not an IP address and port such as 127.0.0.1:9100"
            ));
        }
        if self.motd.chars().count() > 200 {
            problems.push("motd is longer than 200 characters".to_owned());
        }
        problems
    }

    /// A commented example file showing every key at its default.
    pub fn example_toml() -> String {
        let d = Self::default();
        format!(
            "# ss2-server configuration. Every key is optional; the values below are the defaults.\n\
             # A key that is not listed here is an error, so a typo is caught.\n\n\
             name = {name:?}\n\
             # An IP address and port, not a host name.\n\
             bind = {bind:?}\n\
             max_players = {max}\n\
             tick_hz = {hz}\n\
             console = {console}\n\
             motd = {motd:?}\n\n\
             # Leave the next keys out for the default of none.\n\
             # password = \"a session password\"\n\
             # level = \"Levels/Egypt/Sirius\"\n\
             # data_dir = \"/srv/ss2\"\n\
             # log_dir = \"/var/log/ss2\"\n\
             # metrics_bind = \"127.0.0.1:9100\"   # off unless set; do not expose it without authentication in front\n",
            name = d.name,
            bind = d.bind,
            max = d.max_players,
            hz = d.tick_hz,
            console = d.console,
            motd = d.motd,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_core::cli::LaunchOptions;

    #[test]
    fn an_empty_file_gives_the_defaults_and_they_are_valid() {
        let s = ServerSettings::from_toml("").unwrap();
        assert_eq!(s, ServerSettings::default());
        assert_eq!(s.bind, "0.0.0.0:27960");
        assert_eq!((s.max_players, s.tick_hz), (8, 60));
        assert!(s.password.is_none() && s.metrics_bind.is_none() && s.level.is_none());
        assert!(s.validate().is_empty());
    }

    #[test]
    fn a_full_file_is_read_key_by_key() {
        let text = r#"
            name = "Sirius Co-op"
            bind = "127.0.0.1:30000"
            max_players = 4
            password = "hunter2"
            tick_hz = 60
            level = "Levels/Egypt/Sirius"
            data_dir = "/srv/ss2"
            log_dir = "/var/log/ss2"
            metrics_bind = "127.0.0.1:9100"
            console = false
            motd = "Welcome"
        "#;
        let s = ServerSettings::from_toml(text).unwrap();
        assert_eq!(s.name, "Sirius Co-op");
        assert_eq!(
            (s.bind.as_str(), s.max_players, s.tick_hz),
            ("127.0.0.1:30000", 4, 60)
        );
        assert_eq!(s.password.as_deref(), Some("hunter2"));
        assert_eq!(s.level.as_deref(), Some("Levels/Egypt/Sirius"));
        assert_eq!(s.metrics_bind.as_deref(), Some("127.0.0.1:9100"));
        assert!(!s.console);
        assert_eq!(s.motd, "Welcome");
        assert!(s.validate().is_empty());
    }

    #[test]
    fn an_unknown_key_or_a_wrong_type_is_an_error_naming_it() {
        let typo = ServerSettings::from_toml("max_player = 4").unwrap_err();
        assert!(typo.to_string().contains("max_player"), "{typo}");
        let wrong = ServerSettings::from_toml("max_players = \"four\"").unwrap_err();
        assert!(wrong.to_string().contains("max_players"), "{wrong}");
        assert!(ServerSettings::from_toml("this is not toml").is_err());
    }

    #[test]
    fn every_problem_is_reported_at_once() {
        let s = ServerSettings {
            max_players: 0,
            tick_hz: 5000,
            bind: "not an address".into(),
            name: String::new(),
            metrics_bind: Some("also bad".into()),
            password: Some("x".repeat(200)),
            ..ServerSettings::default()
        };
        let problems = s.validate();
        for needle in [
            "max_players",
            "tick_hz",
            "bind",
            "name",
            "metrics_bind",
            "password",
        ] {
            assert!(
                problems.iter().any(|p| p.contains(needle)),
                "{needle} in {problems:?}"
            );
        }
        assert_eq!(problems.len(), 6, "{problems:?}");
        let ok = ServerSettings {
            max_players: 64,
            tick_hz: 1000,
            ..ServerSettings::default()
        };
        assert!(ok.validate().is_empty());
        let too_many = ServerSettings {
            max_players: 65,
            ..ServerSettings::default()
        };
        assert_eq!(too_many.validate().len(), 1);
    }

    #[test]
    fn a_bind_address_must_be_an_ip_and_port_not_a_name_that_needs_a_lookup() {
        for good in ["0.0.0.0:27960", "127.0.0.1:1", "[::1]:27960", "[::]:27960"] {
            assert!(
                ServerSettings {
                    bind: good.into(),
                    ..ServerSettings::default()
                }
                .validate()
                .is_empty(),
                "{good}"
            );
        }
        for bad in ["localhost:27960", "0.0.0.0", ":27960", "1.2.3.4:99999", ""] {
            assert!(
                !ServerSettings {
                    bind: bad.into(),
                    ..ServerSettings::default()
                }
                .validate()
                .is_empty(),
                "{bad}"
            );
        }
    }

    #[test]
    fn the_command_line_wins_over_the_file() {
        let mut s =
            ServerSettings::from_toml("bind = \"1.2.3.4:1000\"\nlevel = \"FromFile\"").unwrap();
        s.apply_launch_options(&LaunchOptions {
            bind: Some("5.6.7.8:2000".into()),
            level: Some("FromCli".into()),
            data_dir: Some("/data".into()),
            ..LaunchOptions::default()
        });
        assert_eq!(
            (s.bind.as_str(), s.level.as_deref()),
            ("5.6.7.8:2000", Some("FromCli"))
        );
        assert_eq!(s.data_dir.as_deref(), Some(std::path::Path::new("/data")));
        // Options that were not given leave the file's values alone.
        let mut t = ServerSettings::from_toml("bind = \"1.2.3.4:1000\"").unwrap();
        t.apply_launch_options(&LaunchOptions::default());
        assert_eq!(t.bind, "1.2.3.4:1000");
    }

    #[test]
    fn the_password_never_appears_in_debug_output() {
        let s = ServerSettings {
            password: Some("hunter2".into()),
            ..ServerSettings::default()
        };
        let shown = format!("{s:?}");
        assert!(!shown.contains("hunter2"), "{shown}");
        assert!(shown.contains("<set>"), "it says that one is set: {shown}");
        assert!(
            format!("{:?}", ServerSettings::default()).contains("None")
                || !format!("{:?}", ServerSettings::default()).contains("<set>")
        );
    }

    #[test]
    fn the_example_file_parses_and_names_every_key() {
        let example = ServerSettings::example_toml();
        let parsed = ServerSettings::from_toml(&example).expect("the example is a valid file");
        assert_eq!(
            parsed,
            ServerSettings::default(),
            "the example shows the defaults"
        );
        for key in [
            "name",
            "bind",
            "max_players",
            "password",
            "tick_hz",
            "level",
            "data_dir",
            "log_dir",
            "metrics_bind",
            "console",
            "motd",
        ] {
            assert!(example.contains(key), "{key} is documented in the example");
        }
    }
}
