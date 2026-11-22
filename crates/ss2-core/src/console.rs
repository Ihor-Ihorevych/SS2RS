//! The console backend: variables (cvars), commands, history and autocomplete (T03-015).
//!
//! This is the logic behind the in-game console; the window with the text box is T10-007. A
//! [`Console`] owns typed variables ([`CVarSpec`], [`CVarValue`]), registered commands, a bounded
//! command history and prefix completion:
//!
//! ```
//! use ss2_core::console::{Console, CVarSpec, CVarValue};
//!
//! let mut console = Console::new();
//! console
//!     .register_cvar(CVarSpec::new("r_fov", CVarValue::Float(80.0), "field of view").range(30.0, 120.0))
//!     .unwrap();
//! assert_eq!(console.execute("r_fov 200").unwrap(), "r_fov = 120 (default 80)");
//! assert_eq!(console.get_float("r_fov"), Some(120.0));
//! assert_eq!(console.complete("r_").candidates, ["r_fov"]);
//! ```
//!
//! A line is a command name followed by arguments separated by spaces; `"double quotes"` keep
//! spaces inside one argument (`\"` is a quote) and `//` at the start of a word begins a comment.
//! Naming a variable prints it, naming it with a value sets it (`set`, `toggle` and `reset` do
//! the same explicitly). Everything is deterministic: names are kept in sorted maps.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;

use crate::{Error, Result};

/// Names that are always commands.
const BUILTINS: [(&str, &str); 5] = [
    ("cvarlist", "list all variables with their values"),
    ("help", "list commands and variables, or `help <name>`"),
    ("reset", "reset a variable to its default: `reset <name>`"),
    ("set", "set a variable: `set <name> <value>`"),
    ("toggle", "flip a boolean variable: `toggle <name>`"),
];

/// The value of a console variable; its type is fixed when it is registered.
#[derive(Clone, Debug, PartialEq)]
pub enum CVarValue {
    /// `on`/`off`, `1`/`0`, `true`/`false`.
    Bool(bool),
    /// A whole number.
    Int(i64),
    /// A finite real number.
    Float(f64),
    /// Text (the whole remaining line when unquoted).
    Str(String),
}

impl fmt::Display for CVarValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(v) => write!(f, "{v}"),
            Self::Int(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::Str(v) => f.write_str(v),
        }
    }
}

/// How a variable is declared.
#[derive(Clone, Debug)]
pub struct CVarSpec {
    name: String,
    default: CVarValue,
    help: String,
    read_only: bool,
    range: Option<(f64, f64)>,
}

impl CVarSpec {
    /// A variable called `name` with its default value and a one-line description.
    pub fn new(name: impl Into<String>, default: CVarValue, help: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            default,
            help: help.into(),
            read_only: false,
            range: None,
        }
    }

    /// The console cannot change it (build information, for example).
    #[must_use]
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// Numbers set from the console are clamped to `min..=max` (ignored for booleans and text).
    #[must_use]
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.range = Some((min, max));
        self
    }
}

#[derive(Debug)]
struct CVar {
    spec: CVarSpec,
    value: CVarValue,
}

/// The variables of a [`Console`]; command handlers receive it so they can read and set them.
#[derive(Debug, Default)]
pub struct CVars {
    vars: BTreeMap<String, CVar>,
}

impl CVars {
    /// The current value, if the variable exists.
    pub fn get(&self, name: &str) -> Option<&CVarValue> {
        self.vars.get(name).map(|v| &v.value)
    }

    /// A boolean variable's value (`None` if missing or of another type).
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        match self.get(name) {
            Some(CVarValue::Bool(v)) => Some(*v),
            _ => None,
        }
    }

    /// An integer variable's value.
    pub fn get_int(&self, name: &str) -> Option<i64> {
        match self.get(name) {
            Some(CVarValue::Int(v)) => Some(*v),
            _ => None,
        }
    }

    /// A float variable's value.
    pub fn get_float(&self, name: &str) -> Option<f64> {
        match self.get(name) {
            Some(CVarValue::Float(v)) => Some(*v),
            _ => None,
        }
    }

    /// A string variable's value.
    pub fn get_str(&self, name: &str) -> Option<&str> {
        match self.get(name) {
            Some(CVarValue::Str(v)) => Some(v),
            _ => None,
        }
    }

    /// Parse `text` for the variable's type, clamp it to the range and store it. Returns the
    /// stored value.
    ///
    /// # Errors
    /// [`Error::NotFound`] for an unknown name; [`Error::Invalid`] for a read-only variable or a
    /// value that does not parse. A failed `set` changes nothing.
    pub fn set(&mut self, name: &str, text: &str) -> Result<CVarValue> {
        let var = self
            .vars
            .get_mut(name)
            .ok_or_else(|| Error::NotFound(format!("variable {name}")))?;
        if var.spec.read_only {
            return Err(Error::invalid(
                "console variable",
                format!("{name} is read-only"),
            ));
        }
        let value = parse_value(name, &var.spec, text)?;
        var.value = value.clone();
        Ok(value)
    }

    /// Restore the default value.
    ///
    /// # Errors
    /// [`Error::NotFound`] for an unknown name; [`Error::Invalid`] for a read-only variable.
    pub fn reset(&mut self, name: &str) -> Result<CVarValue> {
        let var = self
            .vars
            .get_mut(name)
            .ok_or_else(|| Error::NotFound(format!("variable {name}")))?;
        if var.spec.read_only {
            return Err(Error::invalid(
                "console variable",
                format!("{name} is read-only"),
            ));
        }
        var.value = var.spec.default.clone();
        Ok(var.value.clone())
    }

    fn describe(&self, name: &str) -> Option<String> {
        self.vars
            .get(name)
            .map(|v| format!("{name} = {} (default {})", v.value, v.spec.default))
    }
}

fn parse_value(name: &str, spec: &CVarSpec, text: &str) -> Result<CVarValue> {
    let bad = |what: &str| {
        Error::invalid(
            "console variable",
            format!("{name}: {text:?} is not {what}"),
        )
    };
    Ok(match &spec.default {
        CVarValue::Bool(_) => match text.to_ascii_lowercase().as_str() {
            "1" | "true" | "on" | "yes" => CVarValue::Bool(true),
            "0" | "false" | "off" | "no" => CVarValue::Bool(false),
            _ => return Err(bad("a boolean (use on/off, 1/0 or true/false)")),
        },
        CVarValue::Int(_) => {
            let v: i64 = text.parse().map_err(|_| bad("an integer"))?;
            CVarValue::Int(match spec.range {
                // The range bounds are whole numbers in practice; saturating casts are fine.
                Some((lo, hi)) => v.clamp(lo as i64, hi as i64),
                None => v,
            })
        }
        CVarValue::Float(_) => {
            let v: f64 = text.parse().map_err(|_| bad("a number"))?;
            if !v.is_finite() {
                return Err(bad("a finite number"));
            }
            CVarValue::Float(match spec.range {
                Some((lo, hi)) => v.clamp(lo, hi),
                None => v,
            })
        }
        CVarValue::Str(_) => CVarValue::Str(text.to_owned()),
    })
}

/// What a command handler is: it gets the variables and its arguments and returns the text to
/// show.
type Handler = Box<dyn FnMut(&mut CVars, &[String]) -> Result<String>>;

struct Command {
    help: String,
    run: Handler,
}

/// The candidates for a prefix and what the line can be extended with at once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    /// All names starting with the prefix, sorted.
    pub candidates: Vec<String>,
    /// The longest prefix shared by all candidates (the prefix itself if there are none).
    pub common: String,
}

/// Variables, commands, history and completion.
pub struct Console {
    cvars: CVars,
    commands: BTreeMap<String, Command>,
    history: VecDeque<String>,
    history_limit: usize,
    /// Position while walking the history with [`Console::history_prev`]; `None` = the empty
    /// line after the newest entry.
    cursor: Option<usize>,
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

impl Console {
    /// An empty console with the built-in commands and room for 100 history lines.
    pub fn new() -> Self {
        Self::with_history_limit(100)
    }

    /// Like [`Console::new`] with room for `limit` history lines (at least 1).
    pub fn with_history_limit(limit: usize) -> Self {
        Self {
            cvars: CVars::default(),
            commands: BTreeMap::new(),
            history: VecDeque::new(),
            history_limit: limit.max(1),
            cursor: None,
        }
    }

    /// The variables (read access for the rest of the program).
    pub fn cvars(&self) -> &CVars {
        &self.cvars
    }

    /// The variables, for the program to set values directly.
    pub fn cvars_mut(&mut self) -> &mut CVars {
        &mut self.cvars
    }

    /// See [`CVars::get`].
    pub fn cvar(&self, name: &str) -> Option<&CVarValue> {
        self.cvars.get(name)
    }

    /// See [`CVars::get_bool`].
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.cvars.get_bool(name)
    }

    /// See [`CVars::get_int`].
    pub fn get_int(&self, name: &str) -> Option<i64> {
        self.cvars.get_int(name)
    }

    /// See [`CVars::get_float`].
    pub fn get_float(&self, name: &str) -> Option<f64> {
        self.cvars.get_float(name)
    }

    /// See [`CVars::get_str`].
    pub fn get_str(&self, name: &str) -> Option<&str> {
        self.cvars.get_str(name)
    }

    /// Declare a variable.
    ///
    /// # Errors
    /// [`Error::Invalid`] for a bad or already used name (letters, digits, `_` and `.` only).
    pub fn register_cvar(&mut self, spec: CVarSpec) -> Result<()> {
        self.check_new_name(&spec.name)?;
        let value = spec.default.clone();
        self.cvars
            .vars
            .insert(spec.name.clone(), CVar { spec, value });
        Ok(())
    }

    /// Declare a command. `help` is its one-line description.
    ///
    /// # Errors
    /// [`Error::Invalid`] for a bad or already used name.
    pub fn register_command(
        &mut self,
        name: &str,
        help: &str,
        handler: impl FnMut(&mut CVars, &[String]) -> Result<String> + 'static,
    ) -> Result<()> {
        self.check_new_name(name)?;
        self.commands.insert(
            name.to_owned(),
            Command {
                help: help.to_owned(),
                run: Box::new(handler),
            },
        );
        Ok(())
    }

    fn check_new_name(&self, name: &str) -> Result<()> {
        let valid = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
        if !valid {
            return Err(Error::invalid(
                "console name",
                format!("{name:?} must be letters, digits, `_` or `.`"),
            ));
        }
        let taken = self.cvars.vars.contains_key(name)
            || self.commands.contains_key(name)
            || BUILTINS.iter().any(|(b, _)| *b == name);
        if taken {
            return Err(Error::invalid(
                "console name",
                format!("{name} is already registered"),
            ));
        }
        Ok(())
    }

    /// Run one line and return the text to show. Every non-empty line is added to the history,
    /// including lines that fail (so they can be recalled and fixed).
    ///
    /// # Errors
    /// [`Error::NotFound`] for an unknown name (with a suggestion when one is close),
    /// [`Error::Invalid`] for a bad value, a read-only variable, a malformed line or whatever
    /// a command reports.
    pub fn execute(&mut self, line: &str) -> Result<String> {
        let tokens = tokenize(line)?;
        let Some((name, args)) = tokens.split_first() else {
            return Ok(String::new());
        };
        self.record(line.trim());
        match name.as_str() {
            "help" => self.help(args),
            "cvarlist" => Ok(self.cvar_list()),
            "set" => match args {
                [var, value] => self.set_and_describe(var, value),
                _ => Err(Error::invalid(
                    "console command",
                    "usage: set <name> <value>",
                )),
            },
            "toggle" => match args {
                [var] => self.toggle(var),
                _ => Err(Error::invalid("console command", "usage: toggle <name>")),
            },
            "reset" => match args {
                [var] => {
                    self.cvars.reset(var)?;
                    Ok(self.cvars.describe(var).unwrap_or_default())
                }
                _ => Err(Error::invalid("console command", "usage: reset <name>")),
            },
            _ => {
                if let Some(command) = self.commands.get_mut(name) {
                    return (command.run)(&mut self.cvars, args);
                }
                if self.cvars.vars.contains_key(name) {
                    return self.use_cvar(name, args);
                }
                Err(self.unknown(name))
            }
        }
    }

    fn use_cvar(&mut self, name: &str, args: &[String]) -> Result<String> {
        match args {
            [] => Ok(self.cvars.describe(name).unwrap_or_default()),
            _ => {
                let is_text = matches!(self.cvars.get(name), Some(CVarValue::Str(_)));
                if args.len() > 1 && !is_text {
                    return Err(Error::invalid(
                        "console variable",
                        format!("{name} takes one value"),
                    ));
                }
                self.set_and_describe(name, &args.join(" "))
            }
        }
    }

    fn set_and_describe(&mut self, name: &str, value: &str) -> Result<String> {
        self.cvars.set(name, value)?;
        Ok(self.cvars.describe(name).unwrap_or_default())
    }

    fn toggle(&mut self, name: &str) -> Result<String> {
        let Some(now) = self.cvars.get_bool(name) else {
            return if self.cvars.vars.contains_key(name) {
                Err(Error::invalid(
                    "console variable",
                    format!("{name} is not a boolean"),
                ))
            } else {
                Err(Error::NotFound(format!("variable {name}")))
            };
        };
        self.set_and_describe(name, if now { "0" } else { "1" })
    }

    fn cvar_list(&self) -> String {
        self.cvars
            .vars
            .iter()
            .map(|(name, v)| format!("{name} = {}", v.value))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn help(&self, args: &[String]) -> Result<String> {
        if let Some(name) = args.first() {
            if let Some(v) = self.cvars.vars.get(name.as_str()) {
                let mut text = format!(
                    "{} = {} (default {})\n{}",
                    name, v.value, v.spec.default, v.spec.help
                );
                if let Some((lo, hi)) = v.spec.range {
                    text.push_str(&format!("\nrange {lo}..{hi}"));
                }
                if v.spec.read_only {
                    text.push_str("\nread-only");
                }
                return Ok(text);
            }
            if let Some(c) = self.commands.get(name.as_str()) {
                return Ok(format!("{name}: {}", c.help));
            }
            if let Some((_, help)) = BUILTINS.iter().find(|(b, _)| *b == name) {
                return Ok(format!("{name}: {help}"));
            }
            return Err(self.unknown(name));
        }
        let mut commands: BTreeMap<&str, &str> = BUILTINS.iter().copied().collect();
        for (name, c) in &self.commands {
            commands.insert(name, &c.help);
        }
        let mut text = String::from("Commands:\n");
        for (name, help) in commands {
            text.push_str(&format!("  {name:<12} {help}\n"));
        }
        text.push_str("Variables:\n");
        for (name, v) in &self.cvars.vars {
            text.push_str(&format!("  {name:<20} {}\n", v.spec.help));
        }
        Ok(text)
    }

    fn all_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .cvars
            .vars
            .keys()
            .map(String::as_str)
            .chain(self.commands.keys().map(String::as_str))
            .chain(BUILTINS.iter().map(|(n, _)| *n))
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    fn unknown(&self, name: &str) -> Error {
        let best = self
            .all_names()
            .into_iter()
            .map(|candidate| (edit_distance(name, candidate), candidate))
            .filter(|(d, _)| *d <= 2)
            .min_by_key(|(d, _)| *d);
        match best {
            Some((_, candidate)) => Error::NotFound(format!(
                "command or variable {name}; did you mean {candidate}?"
            )),
            None => Error::NotFound(format!("command or variable {name}")),
        }
    }

    /// The names that start with `prefix` (commands and variables) and their common prefix, for
    /// the Tab key.
    pub fn complete(&self, prefix: &str) -> Completion {
        let candidates: Vec<String> = self
            .all_names()
            .into_iter()
            .filter(|n| n.starts_with(prefix))
            .map(str::to_owned)
            .collect();
        let common = match candidates.split_first() {
            None => prefix.to_owned(),
            Some((first, rest)) => rest.iter().fold(first.clone(), |acc, c| {
                let shared = acc
                    .chars()
                    .zip(c.chars())
                    .take_while(|(a, b)| a == b)
                    .count();
                acc.chars().take(shared).collect()
            }),
        };
        Completion { candidates, common }
    }

    fn record(&mut self, line: &str) {
        self.cursor = None;
        if self.history.back().map(String::as_str) == Some(line) {
            return;
        }
        self.history.push_back(line.to_owned());
        while self.history.len() > self.history_limit {
            self.history.pop_front();
        }
    }

    /// The executed lines, oldest first.
    pub fn history(&self) -> Vec<&str> {
        self.history.iter().map(String::as_str).collect()
    }

    /// Step back through the history (the Up key). Stays on the oldest line at the end.
    pub fn history_prev(&mut self) -> Option<&str> {
        let last = self.history.len().checked_sub(1)?;
        let index = match self.cursor {
            None => last,
            Some(i) => i.saturating_sub(1),
        };
        self.cursor = Some(index);
        self.history.get(index).map(String::as_str)
    }

    /// Step forward (the Down key); `None` means back at the empty line after the newest entry.
    pub fn history_next(&mut self) -> Option<&str> {
        let i = self.cursor?;
        if i + 1 < self.history.len() {
            self.cursor = Some(i + 1);
            self.history.get(i + 1).map(String::as_str)
        } else {
            self.cursor = None;
            None
        }
    }
}

/// Split a console line into arguments: whitespace separates, `"double quotes"` group (with
/// `\"` and `\\` escapes inside), and `//` at the start of a word starts a comment.
///
/// # Errors
/// [`Error::Invalid`] for a quote that is never closed.
pub fn tokenize(line: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            match c {
                '"' => in_quotes = false,
                '\\' if matches!(chars.peek(), Some('"' | '\\')) => {
                    current.extend(chars.next());
                }
                _ => current.push(c),
            }
        } else if c == '"' {
            in_quotes = true;
            started = true;
        } else if c.is_whitespace() {
            if started {
                tokens.push(std::mem::take(&mut current));
                started = false;
            }
        } else if c == '/' && !started && chars.peek() == Some(&'/') {
            break;
        } else {
            current.push(c);
            started = true;
        }
    }
    if in_quotes {
        return Err(Error::invalid("console line", "a quote is never closed"));
    }
    if started {
        tokens.push(current);
    }
    Ok(tokens)
}

/// The number of single-character edits between two names (for "did you mean").
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = if ca == cb {
                diagonal
            } else {
                1 + diagonal.min(above).min(row[j])
            };
            diagonal = above;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn console() -> Console {
        let mut c = Console::new();
        let specs = [
            CVarSpec::new(
                "r_vsync",
                CVarValue::Bool(true),
                "synchronize with the display",
            ),
            CVarSpec::new("r_fov", CVarValue::Float(80.0), "field of view in degrees")
                .range(30.0, 120.0),
            CVarSpec::new("sv_maxplayers", CVarValue::Int(4), "player limit").range(1.0, 16.0),
            CVarSpec::new("name", CVarValue::Str("Sam".into()), "player name"),
            CVarSpec::new("sv_version", CVarValue::Str("1".into()), "build").read_only(),
        ];
        for spec in specs {
            c.register_cvar(spec).unwrap();
        }
        c
    }

    fn err(c: &mut Console, line: &str) -> String {
        c.execute(line).unwrap_err().to_string()
    }

    #[test]
    fn the_tokenizer_handles_quotes_escapes_and_comments() {
        assert_eq!(
            tokenize("  say \"hello world\"  now // a comment").unwrap(),
            ["say", "hello world", "now"]
        );
        assert_eq!(
            tokenize(r#"echo "a \"quoted\" b""#).unwrap(),
            ["echo", "a \"quoted\" b"]
        );
        assert!(tokenize("").unwrap().is_empty());
        assert!(tokenize("   // only a comment").unwrap().is_empty());
        assert!(tokenize("echo \"unterminated").is_err());
    }

    #[test]
    fn cvars_start_at_their_defaults_with_their_types() {
        let c = console();
        assert_eq!(c.get_bool("r_vsync"), Some(true));
        assert_eq!(c.get_float("r_fov"), Some(80.0));
        assert_eq!(c.get_int("sv_maxplayers"), Some(4));
        assert_eq!(c.get_str("name"), Some("Sam"));
        assert_eq!(c.get_bool("name"), None, "wrong type");
        assert_eq!(c.get_int("missing"), None);
    }

    #[test]
    fn booleans_accept_the_usual_spellings_and_reject_the_rest() {
        let mut c = console();
        for (text, expected) in [("off", false), ("1", true), ("false", false), ("ON", true)] {
            c.execute(&format!("r_vsync {text}")).unwrap();
            assert_eq!(c.get_bool("r_vsync"), Some(expected), "{text}");
        }
        let message = err(&mut c, "r_vsync maybe");
        assert!(
            message.contains("r_vsync") && message.contains("maybe"),
            "{message}"
        );
        assert_eq!(
            c.get_bool("r_vsync"),
            Some(true),
            "a bad value changes nothing"
        );
    }

    #[test]
    fn numbers_are_parsed_by_type_and_clamped_to_their_range() {
        let mut c = console();
        let out = c.execute("r_fov 200").unwrap();
        assert_eq!(c.get_float("r_fov"), Some(120.0));
        assert!(out.contains("120"), "{out}");
        c.execute("sv_maxplayers 0").unwrap();
        assert_eq!(c.get_int("sv_maxplayers"), Some(1));
        assert!(err(&mut c, "sv_maxplayers 2.5").contains("sv_maxplayers"));
        assert!(err(&mut c, "r_fov wide").contains("wide"));
    }

    #[test]
    fn a_read_only_cvar_cannot_be_set() {
        let mut c = console();
        assert!(err(&mut c, "sv_version 2").contains("read-only"));
        assert_eq!(c.get_str("sv_version"), Some("1"));
    }

    #[test]
    fn naming_a_cvar_prints_it_and_naming_it_with_a_value_sets_it() {
        let mut c = console();
        assert_eq!(c.execute("r_fov").unwrap(), "r_fov = 80 (default 80)");
        assert_eq!(c.execute("r_fov 90").unwrap(), "r_fov = 90 (default 80)");
        assert_eq!(
            c.execute("set r_fov 100").unwrap(),
            "r_fov = 100 (default 80)"
        );
        assert_eq!(c.get_float("r_fov"), Some(100.0));
    }

    #[test]
    fn a_string_cvar_takes_the_rest_as_one_value_when_quoted() {
        let mut c = console();
        c.execute("name \"Serious Sam\"").unwrap();
        assert_eq!(c.get_str("name"), Some("Serious Sam"));
    }

    #[test]
    fn toggle_flips_booleans_only_and_reset_restores_the_default() {
        let mut c = console();
        c.execute("toggle r_vsync").unwrap();
        assert_eq!(c.get_bool("r_vsync"), Some(false));
        c.execute("toggle r_vsync").unwrap();
        assert_eq!(c.get_bool("r_vsync"), Some(true));
        assert!(err(&mut c, "toggle r_fov").contains("r_fov"));

        c.execute("r_fov 50").unwrap();
        c.execute("reset r_fov").unwrap();
        assert_eq!(c.get_float("r_fov"), Some(80.0));
        assert!(err(&mut c, "reset nothing").contains("nothing"));
    }

    #[test]
    fn registration_rejects_duplicates_and_bad_names() {
        let mut c = console();
        let dup = CVarSpec::new("r_fov", CVarValue::Float(1.0), "");
        assert!(c.register_cvar(dup).is_err());
        assert!(
            c.register_command("echo", "print", |_, a| Ok(a.join(" ")))
                .is_ok()
        );
        assert!(
            c.register_command("echo", "again", |_, _| Ok(String::new()))
                .is_err()
        );
        assert!(
            c.register_command("r_fov", "clash", |_, _| Ok(String::new()))
                .is_err()
        );
        assert!(
            c.register_command("help", "clash", |_, _| Ok(String::new()))
                .is_err()
        );
        for bad in ["", "two words", "semi;colon", "quo\"te"] {
            let spec = CVarSpec::new(bad, CVarValue::Bool(false), "");
            assert!(c.register_cvar(spec).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn commands_receive_their_arguments_and_can_change_cvars() {
        let mut c = console();
        c.register_command("echo", "print the arguments", |_, args| Ok(args.join(" ")))
            .unwrap();
        c.register_command("bump", "one more player", |cvars, _| {
            let now = cvars.get_int("sv_maxplayers").unwrap_or(0);
            cvars.set("sv_maxplayers", &(now + 1).to_string())?;
            Ok("bumped".into())
        })
        .unwrap();
        assert_eq!(c.execute("echo a \"b c\" d").unwrap(), "a b c d");
        assert_eq!(c.execute("bump").unwrap(), "bumped");
        assert_eq!(c.get_int("sv_maxplayers"), Some(5));
    }

    #[test]
    fn an_unknown_name_suggests_a_close_one() {
        let mut c = console();
        let message = err(&mut c, "r_vsinc 0");
        assert!(
            message.contains("r_vsinc") && message.contains("r_vsync"),
            "{message}"
        );
        let message = err(&mut c, "zzzzzz");
        assert!(
            message.contains("zzzzzz") && !message.contains("did you mean"),
            "{message}"
        );
    }

    #[test]
    fn an_empty_line_does_nothing() {
        let mut c = console();
        assert_eq!(c.execute("   ").unwrap(), "");
        assert_eq!(c.execute("// just a comment").unwrap(), "");
        assert!(c.history().is_empty());
    }

    #[test]
    fn help_and_cvarlist_describe_what_is_registered() {
        let mut c = console();
        c.register_command("echo", "print the arguments", |_, a| Ok(a.join(" ")))
            .unwrap();
        let all = c.execute("help").unwrap();
        for needle in ["echo", "print the arguments", "r_fov", "help", "cvarlist"] {
            assert!(all.contains(needle), "missing {needle:?} in\n{all}");
        }
        let one = c.execute("help r_fov").unwrap();
        assert!(one.contains("field of view in degrees"), "{one}");
        assert!(
            one.contains("30") && one.contains("120"),
            "range shown: {one}"
        );
        let list = c.execute("cvarlist").unwrap();
        let names: Vec<&str> = list.lines().map(|l| l.split(' ').next().unwrap()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
        assert_eq!(names.len(), 5);
    }

    #[test]
    fn history_records_lines_collapses_repeats_and_is_bounded() {
        let mut c = Console::with_history_limit(3);
        c.register_cvar(CVarSpec::new("a", CVarValue::Int(0), ""))
            .unwrap();
        for line in ["a 1", "a 1", "a 2", "bogus", "a 3"] {
            let _ = c.execute(line);
        }
        assert_eq!(c.history(), ["a 2", "bogus", "a 3"]);
    }

    #[test]
    fn history_navigation_walks_back_and_forward() {
        let mut c = console();
        for line in ["r_fov 40", "r_fov 50", "r_fov 60"] {
            c.execute(line).unwrap();
        }
        assert_eq!(c.history_prev(), Some("r_fov 60"));
        assert_eq!(c.history_prev(), Some("r_fov 50"));
        assert_eq!(c.history_prev(), Some("r_fov 40"));
        assert_eq!(c.history_prev(), Some("r_fov 40"), "stays at the oldest");
        assert_eq!(c.history_next(), Some("r_fov 50"));
        assert_eq!(c.history_next(), Some("r_fov 60"));
        assert_eq!(
            c.history_next(),
            None,
            "past the newest: back to the empty line"
        );
        c.history_prev();
        c.execute("r_fov 70").unwrap();
        assert_eq!(
            c.history_prev(),
            Some("r_fov 70"),
            "executing resets the cursor"
        );
    }

    #[test]
    fn completion_lists_sorted_candidates_and_their_common_prefix() {
        let mut c = console();
        c.register_command("echo", "print", |_, a| Ok(a.join(" ")))
            .unwrap();
        let r = c.complete("r_");
        assert_eq!(r.candidates, ["r_fov", "r_vsync"]);
        assert_eq!(r.common, "r_");
        let r = c.complete("r_v");
        assert_eq!(r.candidates, ["r_vsync"]);
        assert_eq!(r.common, "r_vsync");
        let r = c.complete("zzz");
        assert!(r.candidates.is_empty());
        assert_eq!(r.common, "zzz");
        assert!(c.complete("he").candidates.contains(&"help".to_string()));
        assert!(c.complete("ec").candidates.contains(&"echo".to_string()));
    }
}
