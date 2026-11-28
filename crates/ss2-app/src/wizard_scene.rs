//! The first-run wizard as a scene of the window (T10-019b, `docs/FIRST_RUN.md`).
//!
//! `main` shows it when [`crate::data_location::startup`] finds no usable install. The scene feeds input to the
//! [`DataWizard`], runs every command it gives on a worker thread (searching and checking read the disk for a
//! moment, and the window keeps drawing the "searching" or "checking" screen meanwhile), hands the answers back,
//! and once a folder is accepted (and remembered by the host) opens it for the level picker, again on a worker.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use ss2_ui::DrawList;
use ss2_ui::data_wizard::{DataWizard, WizardCommand, WizardInput, WizardView};

use crate::data_location::{Answer, Host, Step, unix_now};
use crate::level_session::GameData;
use crate::play::Levels;

/// Does the work of a command (normally [`Host::answer`]); runs on a worker thread.
pub(crate) type Answerer = Arc<dyn Fn(WizardCommand) -> Answer + Send + Sync>;
/// Opens the chosen folder for the level picker; runs on a worker thread.
pub(crate) type Opener = Arc<dyn Fn(&Path) -> Levels + Send + Sync>;

/// What the app does after a frame of the wizard.
pub(crate) enum Outcome {
    /// Keep showing the wizard.
    Continue,
    /// The folder is chosen and opened: go to the menu with these levels.
    Ready(Levels),
    /// Leave the application.
    Quit,
}

/// The work the scene is waiting for.
enum Pending {
    None,
    Answer(Receiver<Answer>),
    Open(Receiver<Levels>),
}

/// The wizard scene.
pub(crate) struct WizardScene {
    wizard: DataWizard,
    answerer: Answerer,
    opener: Opener,
    pending: Pending,
    quit: bool,
}

impl WizardScene {
    /// A scene that starts searching at once. `answerer` does the commands and `opener` opens the chosen folder.
    pub(crate) fn new(wizard: DataWizard, answerer: Answerer, opener: Opener) -> Self {
        let mut scene = Self {
            wizard,
            answerer,
            opener,
            pending: Pending::None,
            quit: false,
        };
        let first = scene.wizard.start();
        scene.command(first);
        scene
    }

    /// The scene for the real machine: the host searches with `ss2_vfs::SystemEnvironment` and remembers the choice in `store`;
    /// the chosen folder is mounted with [`GameData::open`] (when that fails, which the check makes unlikely, only the test room is
    /// offered and the reason is logged).
    pub(crate) fn for_system(wizard: DataWizard, store: PathBuf) -> Self {
        let answerer: Answerer = Arc::new(move |command| {
            Host::new(&ss2_vfs::SystemEnvironment, store.clone(), unix_now()).answer(command)
        });
        let opener: Opener = Arc::new(|dir: &Path| {
            tracing::info!("game data folder {} (chosen in the wizard)", dir.display());
            Levels::new(
                GameData::open(dir)
                    .inspect_err(|e| tracing::warn!("cannot open the chosen folder: {e}"))
                    .ok(),
            )
        });
        Self::new(wizard, answerer, opener)
    }

    /// The wizard (for tests).
    #[cfg(test)]
    pub(crate) fn wizard(&self) -> &DataWizard {
        &self.wizard
    }

    /// Whether the player is typing a path (the keyboard then types instead of navigating).
    pub(crate) fn typing(&self) -> bool {
        self.wizard.view() == WizardView::Manual
    }

    /// Whether a worker is busy.
    pub(crate) fn busy(&self) -> bool {
        !matches!(self.pending, Pending::None)
    }

    /// Feed one input event.
    pub(crate) fn handle(&mut self, input: WizardInput) {
        if self.busy() {
            return;
        }
        if let Some(command) = self.wizard.handle(input) {
            self.command(command);
        }
    }

    fn command(&mut self, command: WizardCommand) {
        if command == WizardCommand::Quit {
            self.quit = true;
            return;
        }
        let (tx, rx) = channel();
        let answerer = self.answerer.clone();
        let spawned = std::thread::Builder::new()
            .name("ss2-data-wizard".into())
            .spawn(move || {
                let _ = tx.send(answerer(command));
            });
        match spawned {
            Ok(_) => self.pending = Pending::Answer(rx),
            Err(e) => {
                tracing::error!("cannot start the wizard's worker thread: {e}");
                self.quit = true;
            }
        }
    }

    fn open(&mut self, dir: PathBuf) {
        let (tx, rx) = channel();
        let opener = self.opener.clone();
        let spawned = std::thread::Builder::new()
            .name("ss2-data-open".into())
            .spawn(move || {
                let _ = tx.send(opener(&dir));
            });
        match spawned {
            Ok(_) => self.pending = Pending::Open(rx),
            Err(e) => {
                tracing::error!("cannot start the thread that opens the game data: {e}");
                self.quit = true;
            }
        }
    }

    /// Collect finished work; call once a frame.
    pub(crate) fn poll(&mut self) -> Outcome {
        if self.quit {
            return Outcome::Quit;
        }
        match std::mem::replace(&mut self.pending, Pending::None) {
            Pending::None => {}
            Pending::Answer(rx) => match rx.try_recv() {
                Ok(answer) => match answer.apply(&mut self.wizard) {
                    Step::Continue => {}
                    Step::Chosen(dir) => self.open(dir),
                    Step::Quit => return Outcome::Quit,
                },
                Err(TryRecvError::Empty) => self.pending = Pending::Answer(rx),
                Err(TryRecvError::Disconnected) => {
                    tracing::error!("the wizard's worker thread stopped without an answer");
                    return Outcome::Quit;
                }
            },
            Pending::Open(rx) => match rx.try_recv() {
                Ok(levels) => return Outcome::Ready(levels),
                Err(TryRecvError::Empty) => self.pending = Pending::Open(rx),
                Err(TryRecvError::Disconnected) => {
                    tracing::error!("the thread that opens the game data stopped without a result");
                    return Outcome::Ready(Levels::new(None));
                }
            },
        }
        Outcome::Continue
    }

    /// Replace the contents of `list` with the wizard's screen.
    pub(crate) fn draw_into(&self, list: &mut DrawList) {
        self.wizard.draw_into(list);
    }
}

#[cfg(test)]
#[path = "wizard_scene_tests.rs"]
mod tests;
