//! Cheat and developer commands for the console (roadmap T09-028).
//!
//! The commands do not touch the game. Each one is checked and turned into a [`CheatRequest`]
//! that the game loop drains with [`Cheats::take_requests`] and applies **as a command of the
//! simulation**, so that a co-op host and its clients stay in step (a client must never change
//! state by itself). The sticky switches (`god`, `noclip`, `notarget`) are kept in [`Cheats`] and
//! read through [`Cheats::flags`].
//!
//! Every command refuses to run unless the console variable `sv_cheats` is `1` (it is `0` by
//! default), the usual protection against cheating by accident in a shared game.

use std::cell::RefCell;
use std::rc::Rc;

use ss2_core::console::{CVarSpec, CVarValue, CVars, Console};
use ss2_core::{Error, Result};

/// The most of one item `give` hands out at once.
pub const MAX_GIVE_AMOUNT: u32 = 9_999;
/// The most enemies one `spawn` creates.
pub const MAX_SPAWN_COUNT: u32 = 64;
/// The longest item or class name accepted.
const MAX_NAME_LEN: usize = 64;

/// The sticky cheat switches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CheatFlags {
    /// The player cannot be hurt.
    pub god: bool,
    /// The player flies through walls.
    pub noclip: bool,
    /// Enemies do not notice the player.
    pub notarget: bool,
}

/// What a cheat command asks the game to do. Names are lower case and made of letters, digits,
/// `_` and `.`; whether an item or class exists is for the game to decide when it applies them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheatRequest {
    /// Set the god mode switch.
    SetGod(bool),
    /// Set the noclip switch.
    SetNoclip(bool),
    /// Set the notarget switch.
    SetNotarget(bool),
    /// Give the player `amount` of an item (a weapon, ammunition, armour, `all`).
    Give {
        /// The item name.
        item: String,
        /// How many, 1 to [`MAX_GIVE_AMOUNT`].
        amount: u32,
    },
    /// Create `count` enemies or objects of a class in front of the player.
    Spawn {
        /// The class name.
        class: String,
        /// How many, 1 to [`MAX_SPAWN_COUNT`].
        count: u32,
    },
    /// Kill every enemy of the level.
    KillAll,
    /// Finish the level at once.
    SkipLevel,
}

#[derive(Default)]
struct Shared {
    flags: CheatFlags,
    queue: Vec<CheatRequest>,
}

/// The cheat commands registered on a console, and what they asked for.
///
/// Cloning gives another handle to the same state.
#[derive(Clone, Default)]
pub struct Cheats {
    shared: Rc<RefCell<Shared>>,
}

fn bool_arg(text: &str) -> Result<bool> {
    match text.to_ascii_lowercase().as_str() {
        "1" | "on" | "true" | "yes" => Ok(true),
        "0" | "off" | "false" | "no" => Ok(false),
        _ => Err(Error::invalid(
            "cheat",
            format!("{text:?} is not on or off"),
        )),
    }
}

fn name_arg(what: &'static str, text: &str) -> Result<String> {
    let name = text.to_ascii_lowercase();
    let valid = !name.is_empty()
        && name.len() <= MAX_NAME_LEN
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
    if valid {
        Ok(name)
    } else {
        Err(Error::invalid(
            "cheat",
            format!("{text:?} is not a valid {what} name (letters, digits, `_` and `.`)"),
        ))
    }
}

fn count_arg(what: &'static str, text: &str, max: u32) -> Result<u32> {
    match text.parse::<u32>() {
        Ok(n) if (1..=max).contains(&n) => Ok(n),
        _ => Err(Error::invalid(
            "cheat",
            format!("the {what} must be a whole number from 1 to {max}, not {text:?}"),
        )),
    }
}

fn require_cheats(cvars: &CVars) -> Result<()> {
    if cvars.get_bool("sv_cheats").unwrap_or(false) {
        Ok(())
    } else {
        Err(Error::invalid(
            "cheat",
            "cheats are disabled (set sv_cheats 1 to allow them)",
        ))
    }
}

fn no_arguments(name: &str, args: &[String]) -> Result<()> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(Error::invalid(
            "cheat",
            format!("{name} takes no arguments"),
        ))
    }
}

impl Cheats {
    /// Declare `sv_cheats` and the commands `god`, `noclip`, `notarget`, `give`, `spawn`,
    /// `killall` and `skip` on `console`.
    ///
    /// # Errors
    /// [`Error::Invalid`] when one of the names is already used (registering twice).
    pub fn register(console: &mut Console) -> Result<Self> {
        let cheats = Self::default();
        console.register_cvar(CVarSpec::new(
            "sv_cheats",
            CVarValue::Bool(false),
            "allow cheat commands (god, noclip, give, spawn, ...)",
        ))?;
        type Make = fn(bool) -> CheatRequest;
        let switches: [(&str, &str, Make); 3] = [
            (
                "god",
                "god mode: take no damage ([on|off], toggles without a value)",
                CheatRequest::SetGod,
            ),
            (
                "noclip",
                "fly through walls ([on|off], toggles without a value)",
                CheatRequest::SetNoclip,
            ),
            (
                "notarget",
                "enemies ignore you ([on|off], toggles without a value)",
                CheatRequest::SetNotarget,
            ),
        ];
        for (name, help, make) in switches {
            let shared = Rc::clone(&cheats.shared);
            console.register_command(name, help, move |cvars, args| {
                require_cheats(cvars)?;
                if args.len() > 1 {
                    return Err(Error::invalid(
                        "cheat",
                        format!("{name} takes at most one value"),
                    ));
                }
                let mut state = shared.borrow_mut();
                let flag = match name {
                    "god" => &mut state.flags.god,
                    "noclip" => &mut state.flags.noclip,
                    _ => &mut state.flags.notarget,
                };
                let value = match args.first() {
                    Some(text) => bool_arg(text)?,
                    None => !*flag,
                };
                *flag = value;
                state.queue.push(make(value));
                Ok(format!("{name} {}", if value { "on" } else { "off" }))
            })?;
        }
        let shared = Rc::clone(&cheats.shared);
        console.register_command(
            "give",
            "give <item> [amount]: add an item to the player's inventory",
            move |cvars, args| {
                require_cheats(cvars)?;
                let (item, amount) = match args {
                    [item] => (name_arg("item", item)?, 1),
                    [item, amount] => (
                        name_arg("item", item)?,
                        count_arg("amount", amount, MAX_GIVE_AMOUNT)?,
                    ),
                    _ => return Err(Error::invalid("cheat", "usage: give <item> [amount]")),
                };
                let reply = format!("give {item} x{amount}");
                shared
                    .borrow_mut()
                    .queue
                    .push(CheatRequest::Give { item, amount });
                Ok(reply)
            },
        )?;
        let shared = Rc::clone(&cheats.shared);
        console.register_command(
            "spawn",
            "spawn <class> [count]: create enemies in front of the player",
            move |cvars, args| {
                require_cheats(cvars)?;
                let (class, count) = match args {
                    [class] => (name_arg("class", class)?, 1),
                    [class, count] => (
                        name_arg("class", class)?,
                        count_arg("count", count, MAX_SPAWN_COUNT)?,
                    ),
                    _ => return Err(Error::invalid("cheat", "usage: spawn <class> [count]")),
                };
                let reply = format!("spawn {class} x{count}");
                shared
                    .borrow_mut()
                    .queue
                    .push(CheatRequest::Spawn { class, count });
                Ok(reply)
            },
        )?;
        let shared = Rc::clone(&cheats.shared);
        console.register_command(
            "killall",
            "kill every enemy of the level",
            move |cvars, args| {
                require_cheats(cvars)?;
                no_arguments("killall", args)?;
                shared.borrow_mut().queue.push(CheatRequest::KillAll);
                Ok("kill all enemies".to_owned())
            },
        )?;
        let shared = Rc::clone(&cheats.shared);
        console.register_command("skip", "finish the level at once", move |cvars, args| {
            require_cheats(cvars)?;
            no_arguments("skip", args)?;
            shared.borrow_mut().queue.push(CheatRequest::SkipLevel);
            Ok("skip to the next level".to_owned())
        })?;
        Ok(cheats)
    }

    /// The sticky switches as the commands left them.
    pub fn flags(&self) -> CheatFlags {
        self.shared.borrow().flags
    }

    /// Take (and forget) the requests the commands queued since the last call, oldest first.
    pub fn take_requests(&self) -> Vec<CheatRequest> {
        std::mem::take(&mut self.shared.borrow_mut().queue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_core::console::Console;

    fn console() -> (Console, Cheats) {
        let mut console = Console::new();
        let cheats = Cheats::register(&mut console).unwrap();
        (console, cheats)
    }

    fn enabled() -> (Console, Cheats) {
        let (mut console, cheats) = console();
        console.execute("sv_cheats 1").unwrap();
        (console, cheats)
    }

    #[test]
    fn every_command_is_refused_while_sv_cheats_is_off() {
        let (mut console, cheats) = console();
        for line in [
            "god",
            "noclip",
            "notarget",
            "give shotgun",
            "spawn walker",
            "killall",
            "skip",
        ] {
            let err = console.execute(line).unwrap_err().to_string();
            assert!(err.contains("sv_cheats"), "{line}: {err}");
        }
        assert!(cheats.take_requests().is_empty());
        assert_eq!(cheats.flags(), CheatFlags::default());
    }

    #[test]
    fn sv_cheats_is_a_boolean_variable_that_starts_off() {
        let (mut console, _cheats) = console();
        assert_eq!(console.get_bool("sv_cheats"), Some(false));
        console.execute("sv_cheats 1").unwrap();
        assert_eq!(console.get_bool("sv_cheats"), Some(true));
        assert!(console.execute("sv_cheats maybe").is_err());
    }

    #[test]
    fn god_noclip_and_notarget_toggle_or_take_an_explicit_value() {
        let (mut console, cheats) = enabled();
        assert!(console.execute("god").unwrap().contains("on"));
        assert!(cheats.flags().god);
        assert!(
            console.execute("god").unwrap().contains("off"),
            "a second call toggles back"
        );
        assert!(!cheats.flags().god);
        console.execute("god 1").unwrap();
        console.execute("god 1").unwrap();
        assert!(cheats.flags().god, "an explicit 1 never toggles");
        console.execute("noclip on").unwrap();
        console.execute("notarget true").unwrap();
        let flags = cheats.flags();
        assert!(flags.god && flags.noclip && flags.notarget);
        console.execute("noclip 0").unwrap();
        assert!(!cheats.flags().noclip && cheats.flags().notarget);
        assert!(console.execute("god banana").is_err());
        assert!(console.execute("god 1 2").is_err());
    }

    #[test]
    fn switching_a_flag_is_also_a_request_so_the_game_can_apply_it() {
        let (mut console, cheats) = enabled();
        console.execute("god").unwrap();
        console.execute("noclip 0").unwrap();
        assert_eq!(
            cheats.take_requests(),
            [CheatRequest::SetGod(true), CheatRequest::SetNoclip(false)]
        );
        assert!(cheats.take_requests().is_empty(), "taking drains the queue");
    }

    #[test]
    fn give_takes_an_item_and_an_optional_amount() {
        let (mut console, cheats) = enabled();
        console.execute("give shotgun").unwrap();
        console.execute("give cannonballs 25").unwrap();
        console.execute("give ALL").unwrap();
        assert_eq!(
            cheats.take_requests(),
            [
                CheatRequest::Give {
                    item: "shotgun".into(),
                    amount: 1
                },
                CheatRequest::Give {
                    item: "cannonballs".into(),
                    amount: 25
                },
                CheatRequest::Give {
                    item: "all".into(),
                    amount: 1
                },
            ],
            "item names are lower-cased"
        );
    }

    #[test]
    fn give_rejects_a_missing_item_a_bad_or_absurd_amount() {
        let (mut console, cheats) = enabled();
        for line in [
            "give",
            "give shotgun many",
            "give shotgun 0",
            "give shotgun -3",
            "give shotgun 100000",
            "give a b c",
            "give sho!tgun",
        ] {
            assert!(console.execute(line).is_err(), "{line}");
        }
        assert!(
            cheats.take_requests().is_empty(),
            "a refused command queues nothing"
        );
    }

    #[test]
    fn spawn_takes_a_class_and_a_bounded_count() {
        let (mut console, cheats) = enabled();
        console.execute("spawn Walker").unwrap();
        console.execute("spawn beast 12").unwrap();
        assert_eq!(
            cheats.take_requests(),
            [
                CheatRequest::Spawn {
                    class: "walker".into(),
                    count: 1
                },
                CheatRequest::Spawn {
                    class: "beast".into(),
                    count: 12
                },
            ]
        );
        for line in [
            "spawn",
            "spawn walker 0",
            "spawn walker 65",
            "spawn walker x",
            "spawn a b c",
        ] {
            assert!(console.execute(line).is_err(), "{line}");
        }
    }

    #[test]
    fn killall_and_skip_take_no_arguments() {
        let (mut console, cheats) = enabled();
        console.execute("killall").unwrap();
        console.execute("skip").unwrap();
        assert_eq!(
            cheats.take_requests(),
            [CheatRequest::KillAll, CheatRequest::SkipLevel]
        );
        assert!(console.execute("killall now").is_err());
        assert!(console.execute("skip 2").is_err());
        assert!(cheats.take_requests().is_empty());
    }

    #[test]
    fn the_replies_say_what_was_requested() {
        let (mut console, _cheats) = enabled();
        assert_eq!(
            console.execute("give shotgun 3").unwrap(),
            "give shotgun x3"
        );
        assert_eq!(
            console.execute("spawn walker 2").unwrap(),
            "spawn walker x2"
        );
        assert_eq!(console.execute("killall").unwrap(), "kill all enemies");
        assert_eq!(console.execute("skip").unwrap(), "skip to the next level");
    }

    #[test]
    fn registering_twice_on_one_console_is_an_error_not_a_panic() {
        let (mut console, _cheats) = console();
        assert!(Cheats::register(&mut console).is_err());
    }

    #[test]
    fn the_commands_show_up_in_help_and_completion() {
        let (console, _cheats) = console();
        let completion = console.complete("no");
        let names: Vec<_> = completion.candidates.iter().map(String::as_str).collect();
        assert!(
            names.contains(&"noclip") && names.contains(&"notarget"),
            "{names:?}"
        );
    }

    #[test]
    fn turning_cheats_off_again_blocks_the_commands_but_keeps_the_switch_state() {
        let (mut console, cheats) = enabled();
        console.execute("god 1").unwrap();
        console.execute("sv_cheats 0").unwrap();
        assert!(console.execute("god 0").is_err());
        assert!(
            cheats.flags().god,
            "only the owner of the game decides what to do with it"
        );
    }
}
