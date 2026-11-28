//! The app's half of the first-run wizard (roadmap T10-019, `docs/FIRST_RUN.md`): it answers the commands of
//! [`ss2_ui::data_wizard::DataWizard`] with `ss2_vfs` (discovery, the install check, the remembered choice).
//!
//! `main` asks [`startup`] whether the wizard is needed (T10-019b); the window's wizard scene (`crate::wizard_scene`) then has every
//! command answered by [`Host::answer`] on a worker thread and applies the [`Answer`] on the main thread. `Host::run` (tests only) does both at
//! once, which is how the whole loop is tested without a window.

use std::path::{Path, PathBuf};

use ss2_ui::data_wizard::{DataWizard, WizardCandidate, WizardCommand, WizardReport};
use ss2_vfs::{
    Discovery, Environment, LoadedChoice, RememberedChoice, Startup, ValidationSummary,
    WizardReason, check_install, choice_file, discover, forget_choice, load_choice,
    resolve_startup, save_choice, summarize,
};

/// Where the remembered choice lives: next to `settings.toml`, in the config folder.
pub fn store_path(config_dir: &Path) -> PathBuf {
    choice_file(config_dir)
}

/// Decide at start-up whether the game data folder is known and still good (see [`ss2_vfs::resolve_startup`]). `explicit` is `--data`.
pub fn startup(env: &dyn Environment, explicit: Option<&Path>, store: &Path) -> Startup {
    resolve_startup(env, explicit, store)
}

/// Forget the remembered choice (`--forget-data`), so that the wizard is shown unless `--data` or `SS2_DATA_DIR` names a folder.
/// No file is not an error.
///
/// # Errors
/// The message when the file exists and cannot be removed.
pub fn forget(store: &Path) -> Result<(), String> {
    forget_choice(store).map_err(|e| e.to_string())
}

/// The wizard to show for a start-up that needs it, with the reason as its message.
pub fn wizard_for(reason: &WizardReason) -> DataWizard {
    DataWizard::new(Some(reason.message()))
}

/// What the wizard's candidate list shows for what discovery found.
pub fn wizard_candidates(found: &Discovery) -> Vec<WizardCandidate> {
    found
        .candidates
        .iter()
        .map(|c| {
            let sources: Vec<&str> = c.sources.iter().map(|s| s.label()).collect();
            let mut detail = sources.join(", ");
            match &c.patch {
                Some(level) => detail.push_str(&format!(" - patch {level}")),
                None => detail.push_str(" - no patch"),
            }
            WizardCandidate {
                dir: c.dir.clone(),
                detail,
            }
        })
        .collect()
}

/// What the wizard's result screen shows for a summary.
pub fn wizard_report(summary: &ValidationSummary) -> WizardReport {
    WizardReport {
        dir: summary.dir.clone(),
        usable: summary.usable,
        warnings: summary.warnings,
        headline: summary.headline.clone(),
        lines: summary.lines.clone(),
    }
}

/// What the app does after a command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// Show the wizard as it is now.
    Continue,
    /// The player chose this folder; it is remembered (unless saving failed, which the wizard's notice says): go on.
    Chosen(PathBuf),
    /// Leave the application.
    Quit,
}

/// Answers the wizard's commands.
pub struct Host<'a> {
    env: &'a dyn Environment,
    store: PathBuf,
    now: u64,
}

impl<'a> Host<'a> {
    /// A host that searches through `env`, remembers in `store` and stamps choices with `now` (seconds since 1970).
    pub fn new(env: &'a dyn Environment, store: PathBuf, now: u64) -> Self {
        Self { env, store, now }
    }

    /// The folder remembered from before, if the file holds one.
    fn remembered(&self) -> Option<PathBuf> {
        match load_choice(&self.store) {
            LoadedChoice::Found(choice) => Some(choice.dir),
            LoadedChoice::Missing | LoadedChoice::Damaged(_) => None,
        }
    }

    /// Do what `command` asks and hand the answer to `wizard`: [`Host::answer`] then [`Answer::apply`]. Searching and checking read the
    /// disk and take as long as that takes (the check opens every archive: about a quarter of a second for the retail install), so a
    /// window calls [`Host::answer`] on a worker thread instead and keeps drawing the wizard's "searching" or "checking" screen.
    #[cfg(test)]
    pub fn run(&self, wizard: &mut DataWizard, command: WizardCommand) -> Step {
        self.answer(command).apply(wizard)
    }

    /// Do the work `command` asks for (searching, checking, saving the choice) without touching the wizard, so that it can run on
    /// another thread.
    pub fn answer(&self, command: WizardCommand) -> Answer {
        match command {
            WizardCommand::Search => {
                let found = discover(self.env, self.remembered().as_deref());
                Answer::Candidates(wizard_candidates(&found))
            }
            WizardCommand::Validate(dir) => {
                Answer::Report(wizard_report(&summarize(&check_install(&dir))))
            }
            WizardCommand::Accept(dir) => {
                let choice = RememberedChoice {
                    dir: dir.clone(),
                    validated_at: self.now,
                };
                let save_error = save_choice(&self.store, &choice)
                    .err()
                    .map(|e| e.to_string());
                Answer::Chosen { dir, save_error }
            }
            WizardCommand::Quit => Answer::Quit,
        }
    }
}

/// The result of [`Host::answer`], to be handed to the wizard with [`Answer::apply`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// What the search found.
    Candidates(Vec<WizardCandidate>),
    /// The result of checking a folder.
    Report(WizardReport),
    /// The player chose this folder; `save_error` says why it could not be remembered, if it could not.
    Chosen {
        /// The folder.
        dir: PathBuf,
        /// Why saving the choice failed.
        save_error: Option<String>,
    },
    /// Leave the application.
    Quit,
}

impl Answer {
    /// Hand the answer to `wizard` and say what the app does next.
    pub fn apply(self, wizard: &mut DataWizard) -> Step {
        match self {
            Answer::Candidates(found) => {
                wizard.set_candidates(found);
                Step::Continue
            }
            Answer::Report(report) => {
                wizard.set_report(report);
                Step::Continue
            }
            Answer::Chosen { dir, save_error } => {
                if let Some(e) = save_error {
                    wizard.set_notice(format!(
                        "Your choice could not be saved ({e}); you will be asked again next time"
                    ));
                }
                Step::Chosen(dir)
            }
            Answer::Quit => Step::Quit,
        }
    }
}

/// The current time as seconds since 1970 (0 if the clock is before that).
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
