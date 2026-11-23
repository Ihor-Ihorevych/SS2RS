//! The app's half of the first-run wizard (roadmap T10-019, `docs/FIRST_RUN.md`): it answers the commands of
//! [`ss2_ui::data_wizard::DataWizard`] with `ss2_vfs` (discovery, the install check, the remembered choice).
//!
//! **Not connected to the window yet.** `main` does not call it, because the window is being moved onto the platform layer (T03-033) and
//! the placeholder app uses no game data; the hook for T10-019b is [`startup`] (is the wizard needed?) followed by [`Host::run`] for every
//! command the wizard gives. Everything here works without a window, which is how it is tested.

use std::path::{Path, PathBuf};

use ss2_ui::data_wizard::{DataWizard, WizardCandidate, WizardCommand, WizardReport};
use ss2_vfs::{
    Discovery, Environment, LoadedChoice, RememberedChoice, Startup, ValidationSummary,
    WizardReason, check_install, choice_file, discover, load_choice, resolve_startup, save_choice,
    summarize,
};

/// Where the remembered choice lives: next to `settings.toml`, in the config folder.
pub fn store_path(config_dir: &Path) -> PathBuf {
    choice_file(config_dir)
}

/// Decide at start-up whether the game data folder is known and still good (see [`ss2_vfs::resolve_startup`]). `explicit` is `--data`.
pub fn startup(env: &dyn Environment, explicit: Option<&Path>, store: &Path) -> Startup {
    resolve_startup(env, explicit, store)
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

    /// Do what `command` asks and hand the answer to `wizard`. Searching and checking read the disk and take as long as that takes
    /// (the check opens every archive: about a quarter of a second for the retail install), so the window should show the wizard's
    /// "searching" or "checking" screen once before calling this.
    pub fn run(&self, wizard: &mut DataWizard, command: WizardCommand) -> Step {
        match command {
            WizardCommand::Search => {
                let found = discover(self.env, self.remembered().as_deref());
                wizard.set_candidates(wizard_candidates(&found));
                Step::Continue
            }
            WizardCommand::Validate(dir) => {
                let summary = summarize(&check_install(&dir));
                wizard.set_report(wizard_report(&summary));
                Step::Continue
            }
            WizardCommand::Accept(dir) => {
                let choice = RememberedChoice {
                    dir: dir.clone(),
                    validated_at: self.now,
                };
                if let Err(e) = save_choice(&self.store, &choice) {
                    wizard.set_notice(format!(
                        "Your choice could not be saved ({e}); you will be asked again next time"
                    ));
                }
                Step::Chosen(dir)
            }
            WizardCommand::Quit => Step::Quit,
        }
    }
}

/// The current time as seconds since 1970 (0 if the clock is before that).
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
