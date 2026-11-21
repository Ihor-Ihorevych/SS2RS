//! `cargo xtask doctor`: check this machine against `docs/DEV_TOOLS.md`.
//!
//! The tool table below is the machine-readable copy of that document; a test keeps the two in
//! sync. Checking is separated from running processes (the [`Probe`] trait) so every outcome is
//! unit-tested without needing the tools to be installed.
//!
//! Exit status is non-zero **only** when a *required* tool (or a tool of a role you asked for with
//! `--role`) is missing or too old. Everything else is a warning with an install hint.

use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

/// How much a missing tool matters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// You cannot build, test or contribute without it.
    Required,
    /// Everyone should have it, but nothing breaks without it.
    Recommended,
    /// Needed for one track's work (label as in `DEV_TOOLS.md`, e.g. `"T01"`).
    Role(&'static str),
}

/// One tool from `docs/DEV_TOOLS.md`.
#[derive(Clone, Copy, Debug)]
pub struct Tool {
    /// Name as it appears in `DEV_TOOLS.md`.
    pub name: &'static str,
    /// Alternative command lines that print the version; the first that works wins.
    pub probes: &'static [&'static [&'static str]],
    /// Importance.
    pub level: Level,
    /// Minimum `major.minor`, if the document states one.
    pub min: Option<(u32, u32)>,
    /// What to do about it when missing.
    pub hint: &'static str,
}

/// A tool probed through one command line (`cargo <sub> ...`, `naga --version`, ...).
macro_rules! tool {
    ($name:expr, [$($arg:expr),+], $level:expr, $hint:expr $(,)?) => {
        Tool {
            name: $name,
            probes: &[&[$($arg),+]],
            level: $level,
            min: None,
            hint: $hint,
        }
    };
}

/// The tools of `docs/DEV_TOOLS.md`. Keep in sync: `tests::table_matches_dev_tools_md`.
pub const TOOLS: &[Tool] = &[
    // 1. Everyone (required)
    Tool {
        name: "git",
        probes: &[&["git", "--version"]],
        level: Level::Required,
        min: None,
        hint: "https://git-scm.com/downloads",
    },
    Tool {
        name: "gh",
        probes: &[&["gh", "--version"]],
        level: Level::Required,
        min: None,
        hint: "https://cli.github.com (then `gh auth login`)",
    },
    Tool {
        name: "rustup",
        probes: &[&["rustup", "--version"]],
        level: Level::Required,
        min: None,
        hint: "https://rustup.rs",
    },
    Tool {
        name: "cargo",
        probes: &[&["cargo", "--version"]],
        level: Level::Required,
        min: None,
        hint: "install Rust via rustup (https://rustup.rs)",
    },
    tool!(
        "cargo-deny",
        ["cargo", "deny", "--version"],
        Level::Required,
        "cargo install --locked cargo-deny",
    ),
    // 2. Everyone (recommended)
    tool!(
        "cargo-nextest",
        ["cargo", "nextest", "--version"],
        Level::Recommended,
        "cargo install --locked cargo-nextest",
    ),
    tool!(
        "cargo-llvm-cov",
        ["cargo", "llvm-cov", "--version"],
        Level::Recommended,
        "cargo install --locked cargo-llvm-cov",
    ),
    tool!(
        "cargo-audit",
        ["cargo", "audit", "--version"],
        Level::Recommended,
        "cargo install --locked cargo-audit",
    ),
    Tool {
        name: "cargo-machete",
        // It does not answer `cargo machete --version`; only its own binary answers `--version`.
        probes: &[
            &["cargo-machete", "--version"],
            &["cargo", "machete", "--help"],
        ],
        level: Level::Recommended,
        min: None,
        hint: "cargo install --locked cargo-machete",
    },
    tool!(
        "cargo-udeps",
        ["cargo", "udeps", "--help"],
        Level::Recommended,
        "cargo install --locked cargo-udeps (needs nightly to run)",
    ),
    tool!(
        "cargo-semver-checks",
        ["cargo", "semver-checks", "--version"],
        Level::Recommended,
        "cargo install --locked cargo-semver-checks",
    ),
    tool!(
        "cargo-expand",
        ["cargo", "expand", "--version"],
        Level::Recommended,
        "cargo install --locked cargo-expand",
    ),
    tool!(
        "cargo-bloat",
        ["cargo", "bloat", "--help"],
        Level::Recommended,
        "cargo install --locked cargo-bloat",
    ),
    tool!(
        "cargo-release",
        ["cargo", "release", "--version"],
        Level::Recommended,
        "cargo install --locked cargo-release (maintainers, for releases)",
    ),
    Tool {
        name: "Python",
        probes: &[&["python3", "--version"], &["python", "--version"]],
        level: Level::Recommended,
        min: Some((3, 12)),
        hint: "https://www.python.org/downloads (3.12 or newer)",
    },
    // 3. By track
    Tool {
        name: "JDK",
        probes: &[&["java", "-version"]],
        level: Level::Role("T01"),
        min: Some((17, 0)),
        hint: "install JDK 17+ (the Ghidra release notes name the exact version)",
    },
    tool!(
        "cargo-fuzz",
        ["cargo", "fuzz", "--version"],
        Level::Role("T02"),
        "cargo install --locked cargo-fuzz (needs nightly; use Linux/WSL or let CI run it)",
    ),
    tool!(
        "naga-cli",
        ["naga", "--version"],
        Level::Role("T04/T05"),
        "cargo install --locked naga-cli",
    ),
    Tool {
        name: "vulkaninfo",
        probes: &[&["vulkaninfo", "--version"], &["vulkaninfo", "--summary"]],
        level: Level::Role("T04/T05"),
        min: None,
        hint: "install the Vulkan runtime/SDK tools",
    },
    Tool {
        name: "FFmpeg",
        probes: &[&["ffmpeg", "-version"]],
        level: Level::Role("T06"),
        min: None,
        hint: "https://ffmpeg.org/download.html",
    },
    tool!(
        "mdbook",
        ["mdbook", "--version"],
        Level::Role("T00/T12"),
        "cargo install --locked mdbook",
    ),
];

/// Tools that are GUI applications and cannot be detected from a shell.
pub const NOT_DETECTABLE: &[(&str, &str)] = &[
    (
        "Ghidra",
        "T01: download the latest stable release from the official NSA GitHub releases",
    ),
    ("ImHex", "T01: https://imhex.werwolv.net"),
    ("RenderDoc", "T04/T05: https://renderdoc.org"),
];

/// Result of running one probe command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProbeOutput {
    /// `true` if the process started and exited with status 0.
    pub ok: bool,
    /// Combined stdout and stderr (`java -version` prints to stderr).
    pub text: String,
}

/// Runs commands and reads settings; fakeable in tests.
pub trait Probe {
    /// Run `argv[0]` with `argv[1..]`; `None` if it could not be started at all.
    fn run(&self, argv: &[&str]) -> Option<ProbeOutput>;
    /// An environment variable.
    fn env(&self, key: &str) -> Option<std::ffi::OsString>;
}

/// The real thing: spawns processes, reads the environment.
pub struct SystemProbe;

impl Probe for SystemProbe {
    fn run(&self, argv: &[&str]) -> Option<ProbeOutput> {
        let (program, args) = argv.split_first()?;
        let out = Command::new(program).args(args).output().ok()?;
        let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        Some(ProbeOutput {
            ok: out.status.success(),
            text,
        })
    }

    fn env(&self, key: &str) -> Option<std::ffi::OsString> {
        std::env::var_os(key)
    }
}

/// Verdict for one check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Present (and new enough); carries the version line.
    Ok(String),
    /// Present but older than the minimum.
    TooOld {
        /// What was found.
        found: String,
        /// The minimum `major.minor`.
        min: (u32, u32),
    },
    /// Not found.
    Missing,
}

/// One row of the report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// Tool name.
    pub name: &'static str,
    /// Its importance for this run.
    pub level: Level,
    /// Verdict.
    pub status: Status,
    /// Install hint.
    pub hint: &'static str,
}

impl Finding {
    /// A problem that should make `doctor` fail.
    pub fn is_blocking(&self) -> bool {
        !matches!(self.status, Status::Ok(_)) && self.level != Level::Recommended
    }
}

/// Extract the first `major.minor` (or `major`) from a tool's version output, ignoring a leading
/// `1.` of Java 8 style `1.8.0` (treated as major 8).
pub fn parse_version(text: &str) -> Option<(u32, u32)> {
    let re = regex::Regex::new(r"(\d+)(?:\.(\d+))?").ok()?;
    let caps = re.captures(text)?;
    let major: u32 = caps.get(1)?.as_str().parse().ok()?;
    let minor: u32 = caps.get(2).map_or(Some(0), |m| m.as_str().parse().ok())?;
    if major == 1 && minor > 0 && text.contains("1.8") {
        return Some((minor, 0));
    }
    Some((major, minor))
}

/// First non-empty line, trimmed, shortened for display.
fn first_line(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    line.chars().take(80).collect()
}

/// Check one tool.
pub fn check_tool(probe: &dyn Probe, tool: &Tool) -> Status {
    for argv in tool.probes {
        let Some(out) = probe.run(argv) else { continue };
        if !out.ok {
            continue;
        }
        let line = first_line(&out.text);
        if let Some(min) = tool.min {
            match parse_version(&out.text) {
                Some(found) if found < min => {
                    return Status::TooOld { found: line, min };
                }
                None => return Status::TooOld { found: line, min },
                Some(_) => {}
            }
        }
        return Status::Ok(line);
    }
    Status::Missing
}

/// Check the whole table. Role tools are only included for the roles in `roles` (matched
/// case-insensitively against the track label); they are blocking when requested.
pub fn check_tools(probe: &dyn Probe, roles: &[String]) -> Vec<Finding> {
    TOOLS
        .iter()
        .filter(|t| match t.level {
            Level::Role(label) => roles.iter().any(|r| role_matches(r, label)),
            _ => true,
        })
        .map(|t| Finding {
            name: t.name,
            level: t.level,
            status: check_tool(probe, t),
            hint: t.hint,
        })
        .collect()
}

/// `--role t05` matches the label `T04/T05`; `--role T01` matches `T01`.
fn role_matches(requested: &str, label: &str) -> bool {
    let requested = requested.trim().to_ascii_uppercase();
    !requested.is_empty() && label.split('/').any(|part| part == requested)
}

/// State of the repository hook configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HooksState {
    /// `core.hooksPath` is `.githooks`.
    Configured,
    /// Unset or pointing elsewhere (carries the value, if any).
    NotConfigured(Option<String>),
    /// Could not ask git.
    Unknown,
}

/// Check `git config core.hooksPath`.
pub fn check_hooks(probe: &dyn Probe) -> HooksState {
    match probe.run(&["git", "config", "--get", "core.hooksPath"]) {
        None => HooksState::Unknown,
        Some(out) if !out.ok => HooksState::NotConfigured(None),
        Some(out) => {
            let value = out.text.trim().to_owned();
            if value == ".githooks" {
                HooksState::Configured
            } else {
                HooksState::NotConfigured(Some(value))
            }
        }
    }
}

/// Check the pinned toolchain from `rust-toolchain.toml` against the running `rustc`.
pub fn check_toolchain(probe: &dyn Probe, toolchain_toml: &str) -> Option<Result<String, String>> {
    let pinned = regex::Regex::new(r#"channel\s*=\s*"([^"]+)""#)
        .ok()?
        .captures(toolchain_toml)?
        .get(1)?
        .as_str()
        .to_owned();
    let Some(out) = probe.run(&["rustc", "--version"]) else {
        return Some(Err(format!(
            "rustc not found (pinned toolchain is {pinned})"
        )));
    };
    if out.text.contains(&pinned) || !pinned.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        Some(Ok(format!("{} (pinned {pinned})", first_line(&out.text))))
    } else {
        Some(Err(format!(
            "{} but rust-toolchain.toml pins {pinned}; run `rustup show` inside the repository",
            first_line(&out.text)
        )))
    }
}

/// The full report.
#[derive(Debug)]
pub struct Report {
    /// Tool findings.
    pub tools: Vec<Finding>,
    /// Toolchain verdict (`None` if `rust-toolchain.toml` could not be read).
    pub toolchain: Option<Result<String, String>>,
    /// Hook configuration.
    pub hooks: HooksState,
    /// Game data folder verdict from `SS2_DATA_DIR`.
    pub data_dir: Result<std::path::PathBuf, String>,
    /// `gh auth status` succeeded.
    pub gh_logged_in: Option<bool>,
    /// Roles that were requested but not recognized.
    pub unknown_roles: Vec<String>,
}

impl Report {
    /// `true` if anything blocking was found (exit status 1).
    pub fn has_blocking_problem(&self) -> bool {
        self.tools.iter().any(Finding::is_blocking) || matches!(self.toolchain, Some(Err(_)))
    }
}

/// Run every check.
pub fn run_checks(probe: &dyn Probe, repo_root: &Path, roles: &[String]) -> Report {
    let known: Vec<&str> = TOOLS
        .iter()
        .filter_map(|t| match t.level {
            Level::Role(l) => Some(l),
            _ => None,
        })
        .flat_map(|l| l.split('/'))
        .collect();
    let unknown_roles = roles
        .iter()
        .filter(|r| !known.iter().any(|k| k.eq_ignore_ascii_case(r.trim())))
        .cloned()
        .collect();

    let toolchain_toml =
        std::fs::read_to_string(repo_root.join("rust-toolchain.toml")).unwrap_or_default();
    let data_dir = ss2_testkit::resolve_game_data_dir(probe.env(ss2_testkit::DATA_DIR_ENV))
        .map_err(|e| e.to_string());
    Report {
        tools: check_tools(probe, roles),
        toolchain: check_toolchain(probe, &toolchain_toml),
        hooks: check_hooks(probe),
        data_dir,
        gh_logged_in: probe.run(&["gh", "auth", "status"]).map(|o| o.ok),
        unknown_roles,
    }
}

/// Render the report as plain text (ASCII only, so any Windows console prints it).
pub fn render(report: &Report) -> String {
    let mut out = String::new();
    let w = &mut out;
    let tag = |s: &Status, level: Level| -> &'static str {
        match (s, level) {
            (Status::Ok(_), _) => "[ ok ]",
            (_, Level::Recommended) => "[warn]",
            _ => "[FAIL]",
        }
    };

    let _ = writeln!(w, "SS2RS doctor (checks docs/DEV_TOOLS.md)\n");
    let _ = writeln!(w, "Toolchain");
    match &report.toolchain {
        Some(Ok(msg)) => {
            let _ = writeln!(w, "  [ ok ] {msg}");
        }
        Some(Err(msg)) => {
            let _ = writeln!(w, "  [FAIL] {msg}");
        }
        None => {
            let _ = writeln!(w, "  [warn] could not read rust-toolchain.toml");
        }
    }

    for (title, pick) in [
        ("Required", Level::Required),
        ("Recommended", Level::Recommended),
    ] {
        let _ = writeln!(w, "\n{title} tools");
        for f in report.tools.iter().filter(|f| f.level == pick) {
            print_finding(w, f, tag(&f.status, f.level));
        }
    }
    let roles: Vec<&Finding> = report
        .tools
        .iter()
        .filter(|f| matches!(f.level, Level::Role(_)))
        .collect();
    if !roles.is_empty() {
        let _ = writeln!(w, "\nRole tools (you asked for these)");
        for f in roles {
            print_finding(w, f, tag(&f.status, f.level));
        }
    }
    for r in &report.unknown_roles {
        let _ = writeln!(
            w,
            "  [warn] unknown role {r:?}; known: T01, T02, T04, T05, T06, T00, T12"
        );
    }

    let _ = writeln!(w, "\nRepository setup");
    match &report.hooks {
        HooksState::Configured => {
            let _ = writeln!(
                w,
                "  [ ok ] core.hooksPath = .githooks (proprietary-content guard active)"
            );
        }
        HooksState::NotConfigured(v) => {
            let _ = writeln!(
                w,
                "  [warn] core.hooksPath is {}; run: git config core.hooksPath .githooks",
                v.as_deref()
                    .map_or("not set".to_owned(), |v| format!("{v:?}"))
            );
        }
        HooksState::Unknown => {
            let _ = writeln!(w, "  [warn] could not ask git about core.hooksPath");
        }
    }
    match &report.data_dir {
        Ok(p) => {
            let _ = writeln!(
                w,
                "  [ ok ] SS2_DATA_DIR = {} (data-gated tests will run)",
                p.display()
            );
        }
        Err(why) => {
            let _ = writeln!(
                w,
                "  [info] {why}; data-gated tests will skip (fine without a game install)"
            );
        }
    }
    match report.gh_logged_in {
        Some(true) => {
            let _ = writeln!(w, "  [ ok ] gh is logged in");
        }
        Some(false) => {
            let _ = writeln!(
                w,
                "  [warn] gh is not logged in; run: gh auth login (pushing workflows also needs: gh auth refresh -s workflow)"
            );
        }
        None => {}
    }

    let _ = writeln!(
        w,
        "\nCannot be detected from a shell (install by hand if you work on that track)"
    );
    for (name, hint) in NOT_DETECTABLE {
        let _ = writeln!(w, "  [info] {name}: {hint}");
    }

    let blocking = report.has_blocking_problem();
    let _ = writeln!(
        w,
        "\n{}",
        if blocking {
            "Result: FAIL, fix the [FAIL] lines above."
        } else {
            "Result: OK (warnings are optional)."
        }
    );
    out
}

fn print_finding(w: &mut String, f: &Finding, tag: &str) {
    match &f.status {
        Status::Ok(v) => {
            let _ = writeln!(w, "  {tag} {:<20} {v}", f.name);
        }
        Status::TooOld { found, min } => {
            let _ = writeln!(
                w,
                "  {tag} {:<20} {found} (need {}.{} or newer)\n         -> {}",
                f.name, min.0, min.1, f.hint
            );
        }
        Status::Missing => {
            let _ = writeln!(
                w,
                "  {tag} {:<20} not found on PATH\n         -> {}",
                f.name, f.hint
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::ffi::OsString;

    use super::*;

    /// Scripted probe: maps a joined command line to its output; anything else "is not installed".
    #[derive(Default)]
    struct Fake {
        commands: BTreeMap<String, ProbeOutput>,
        env: BTreeMap<String, OsString>,
    }

    impl Fake {
        fn with(mut self, cmd: &str, ok: bool, text: &str) -> Self {
            self.commands.insert(
                cmd.to_owned(),
                ProbeOutput {
                    ok,
                    text: text.to_owned(),
                },
            );
            self
        }

        /// Every tool in the table present, versions fine.
        fn complete() -> Self {
            let mut f = Self::default();
            for t in TOOLS {
                for argv in t.probes {
                    let text = match t.name {
                        "Python" => "Python 3.12.4",
                        "JDK" => "openjdk version \"21.0.2\" 2024-01-16",
                        _ => "tool 1.2.3",
                    };
                    f = f.with(&argv.join(" "), true, text);
                }
            }
            f.with("rustc --version", true, "rustc 1.99.1 (abc 2026-10-13)")
                .with("git config --get core.hooksPath", true, ".githooks\n")
                .with("gh auth status", true, "Logged in")
        }

        fn without(mut self, tool: &str) -> Self {
            let t = TOOLS
                .iter()
                .find(|t| t.name == tool)
                .expect("tool in table");
            for argv in t.probes {
                self.commands.remove(&argv.join(" "));
            }
            self
        }
    }

    impl Probe for Fake {
        fn run(&self, argv: &[&str]) -> Option<ProbeOutput> {
            self.commands.get(&argv.join(" ")).cloned()
        }

        fn env(&self, key: &str) -> Option<OsString> {
            self.env.get(key).cloned()
        }
    }

    fn report(fake: &Fake, roles: &[&str]) -> Report {
        let roles: Vec<String> = roles.iter().map(|s| (*s).to_owned()).collect();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        run_checks(fake, &root, &roles)
    }

    #[test]
    fn healthy_machine_passes() {
        let r = report(&Fake::complete(), &[]);
        assert!(!r.has_blocking_problem(), "{}", render(&r));
        assert_eq!(r.hooks, HooksState::Configured);
        assert!(matches!(r.toolchain, Some(Ok(_))), "{:?}", r.toolchain);
        assert!(render(&r).contains("Result: OK"));
    }

    #[test]
    fn missing_required_tool_is_blocking_with_a_hint() {
        let r = report(&Fake::complete().without("cargo-deny"), &[]);
        assert!(r.has_blocking_problem());
        let text = render(&r);
        assert!(text.contains("[FAIL] cargo-deny"), "{text}");
        assert!(text.contains("cargo install --locked cargo-deny"), "{text}");
        assert!(text.contains("Result: FAIL"));
    }

    #[test]
    fn missing_recommended_tool_only_warns() {
        let r = report(
            &Fake::complete().without("cargo-nextest").without("Python"),
            &[],
        );
        assert!(!r.has_blocking_problem());
        let text = render(&r);
        assert!(text.contains("[warn] cargo-nextest"), "{text}");
        assert!(text.contains("[warn] Python"), "{text}");
    }

    #[test]
    fn old_python_is_flagged_but_not_blocking() {
        let fake = Fake::complete().with("python3 --version", true, "Python 3.11.9");
        let r = report(&fake, &[]);
        let py = r.tools.iter().find(|f| f.name == "Python").unwrap();
        assert!(
            matches!(py.status, Status::TooOld { min: (3, 12), .. }),
            "{py:?}"
        );
        assert!(!r.has_blocking_problem());
        assert!(render(&r).contains("need 3.12 or newer"));
    }

    #[test]
    fn python_falls_back_to_the_second_probe() {
        let mut fake = Fake::complete().without("Python");
        fake = fake.with("python --version", true, "Python 3.13.0");
        let r = report(&fake, &[]);
        let py = r.tools.iter().find(|f| f.name == "Python").unwrap();
        assert!(
            matches!(&py.status, Status::Ok(v) if v.contains("3.13")),
            "{py:?}"
        );
    }

    #[test]
    fn role_tools_are_hidden_unless_requested_and_blocking_when_requested() {
        let healthy_without_roles = Fake::complete().without("FFmpeg").without("JDK");
        let r = report(&healthy_without_roles, &[]);
        assert!(r.tools.iter().all(|f| !matches!(f.level, Level::Role(_))));
        assert!(
            !r.has_blocking_problem(),
            "role tools are not checked by default"
        );

        let r = report(&healthy_without_roles, &["t06"]);
        assert!(
            r.tools.iter().any(|f| f.name == "FFmpeg"),
            "role matched case-insensitively"
        );
        assert!(
            r.tools.iter().all(|f| f.name != "JDK"),
            "other roles stay hidden"
        );
        assert!(r.has_blocking_problem());

        let r = report(&Fake::complete(), &["T05"]);
        assert!(
            r.tools.iter().any(|f| f.name == "naga-cli"),
            "T05 matches the shared T04/T05 label"
        );
        assert!(!r.has_blocking_problem());
    }

    #[test]
    fn java_version_goes_to_stderr_and_is_still_read() {
        let fake = Fake::complete().with(
            "java -version",
            true,
            "openjdk version \"11.0.2\" 2019-01-15",
        );
        let r = report(&fake, &["T01"]);
        let jdk = r.tools.iter().find(|f| f.name == "JDK").unwrap();
        assert!(
            matches!(jdk.status, Status::TooOld { min: (17, 0), .. }),
            "{jdk:?}"
        );
        assert!(r.has_blocking_problem(), "role tool too old when requested");
    }

    #[test]
    fn unknown_role_is_reported() {
        let r = report(&Fake::complete(), &["T99"]);
        assert_eq!(r.unknown_roles, ["T99"]);
        assert!(render(&r).contains("unknown role"));
    }

    #[test]
    fn hooks_states() {
        let fake = Fake::default();
        assert_eq!(check_hooks(&fake), HooksState::Unknown);
        let fake = Fake::default().with("git config --get core.hooksPath", false, "");
        assert_eq!(check_hooks(&fake), HooksState::NotConfigured(None));
        let fake = Fake::default().with("git config --get core.hooksPath", true, "other/dir\n");
        assert_eq!(
            check_hooks(&fake),
            HooksState::NotConfigured(Some("other/dir".into()))
        );
        let r = report(
            &Fake::complete().with("git config --get core.hooksPath", false, ""),
            &[],
        );
        assert!(!r.has_blocking_problem(), "hooks are a warning");
        assert!(render(&r).contains("git config core.hooksPath .githooks"));
    }

    #[test]
    fn toolchain_mismatch_is_blocking() {
        let toml = "[toolchain]\nchannel = \"1.99.1\"\n";
        assert!(matches!(
            check_toolchain(&Fake::complete(), toml),
            Some(Ok(_))
        ));
        let fake = Fake::complete().with("rustc --version", true, "rustc 1.80.0 (xyz)");
        let verdict = check_toolchain(&fake, toml).unwrap();
        assert!(
            verdict.as_ref().is_err_and(|m| m.contains("pins 1.99.1")),
            "{verdict:?}"
        );
        assert!(
            matches!(check_toolchain(&Fake::default(), toml), Some(Err(_))),
            "no rustc at all"
        );
        assert!(check_toolchain(&Fake::complete(), "no channel here").is_none());
        // A named channel such as `stable` cannot be compared textually: accepted.
        assert!(matches!(
            check_toolchain(&fake, "channel = \"stable\""),
            Some(Ok(_))
        ));
    }

    #[test]
    fn data_dir_unset_is_info_not_failure() {
        let r = report(&Fake::complete(), &[]);
        assert!(r.data_dir.is_err());
        let text = render(&r);
        assert!(text.contains("[info] SS2_DATA_DIR is not set"), "{text}");
        assert!(!r.has_blocking_problem());
    }

    #[test]
    fn data_dir_pointing_at_an_install_is_ok() {
        let tmp = tempdir();
        std::fs::write(tmp.join("All_PC_01.gro"), b"PK").unwrap();
        let mut fake = Fake::complete();
        fake.env
            .insert("SS2_DATA_DIR".into(), tmp.clone().into_os_string());
        let r = report(&fake, &[]);
        assert!(r.data_dir.is_ok());
        assert!(render(&r).contains("data-gated tests will run"));
        std::fs::remove_dir_all(tmp).ok();
    }

    #[test]
    fn gh_login_state_is_reported() {
        let r = report(
            &Fake::complete().with("gh auth status", false, "not logged in"),
            &[],
        );
        assert!(render(&r).contains("gh is not logged in"));
        assert!(!r.has_blocking_problem());
    }

    #[test]
    fn version_parsing() {
        assert_eq!(parse_version("Python 3.12.4"), Some((3, 12)));
        assert_eq!(
            parse_version("openjdk version \"21.0.2\" 2024-01-16"),
            Some((21, 0))
        );
        assert_eq!(parse_version("java version \"1.8.0_292\""), Some((8, 0)));
        assert_eq!(parse_version("tool 7"), Some((7, 0)));
        assert_eq!(parse_version("no digits"), None);
    }

    #[test]
    fn output_is_plain_ascii() {
        let text = render(&report(&Fake::complete().without("gh"), &["T01", "T06"]));
        assert!(
            text.is_ascii(),
            "non-ASCII in the report breaks some Windows consoles"
        );
    }

    /// The table in this file and `docs/DEV_TOOLS.md` must describe the same tools.
    #[test]
    fn table_matches_dev_tools_md() {
        let md = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/DEV_TOOLS.md"),
        )
        .expect("docs/DEV_TOOLS.md exists");
        let lower = md.to_lowercase();
        for tool in TOOLS {
            // `naga-cli` appears as the crate name; GUI/other tools by display name.
            assert!(
                lower.contains(&tool.name.to_lowercase()),
                "tool {:?} is checked by doctor but not mentioned in docs/DEV_TOOLS.md",
                tool.name
            );
        }
        // Every `cargo install --locked <crates...>` in the document must be a tool we check.
        let re = regex::Regex::new(r"cargo install --locked ([a-z0-9 \-]+)").unwrap();
        for caps in re.captures_iter(&md) {
            for krate in caps[1].split_whitespace() {
                assert!(
                    TOOLS.iter().any(|t| t.name == krate),
                    "docs/DEV_TOOLS.md installs {krate:?} but `cargo xtask doctor` does not check it"
                );
            }
        }
        for (name, _) in NOT_DETECTABLE {
            assert!(
                lower.contains(&name.to_lowercase()),
                "{name} missing from DEV_TOOLS.md"
            );
        }
    }

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ss2-doctor-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
