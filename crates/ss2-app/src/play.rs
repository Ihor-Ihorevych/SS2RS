//! The play scene's parts that need no window (T03-037): where levels come from, loading one on a worker thread
//! with a progress bar, and the line of help drawn over the game.

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use ss2_physics::query::QueryWorld;
use ss2_ui::level_select::PickerEntry;
use ss2_ui::loading::{LoadingTracker, StageId};
use ss2_ui::{CANVAS_H, CANVAS_W, Color, DrawList, Quad, Rect, TextAlign, TextRun, Vec2, theme};
use ss2_vfs::Vfs;
use ss2_world::collision::build_static;
use ss2_world::scene::{LevelScene, LoadProgress, LoadStage};

use crate::level_session::GameData;

/// The id of the procedural room that is always listed (useful without game data and for tests).
pub(crate) const TEST_ROOM_ID: &str = "test_room";

/// The levels the picker offers: the install's, if there is one, and the test room.
pub(crate) struct Levels {
    game: Option<GameData>,
}

impl Levels {
    /// Levels from `game` (none when there is no install) plus the test room.
    pub(crate) fn new(game: Option<GameData>) -> Self {
        Self { game }
    }

    /// What the picker lists: the install's levels by group, then the test room.
    pub(crate) fn entries(&self) -> Vec<PickerEntry> {
        let mut out: Vec<PickerEntry> = self
            .game
            .iter()
            .flat_map(|g| g.levels())
            .map(|l| PickerEntry::new(&l.id, &l.group, display_name(&l.name)))
            .collect();
        out.push(PickerEntry::new(TEST_ROOM_ID, "development", "Test room"));
        out
    }

    /// Load the level `id` with its collision world.
    ///
    /// # Errors
    /// An unknown id, a world file that cannot be read, or broken collision data.
    pub(crate) fn load(
        &self,
        id: &str,
        progress: &mut dyn FnMut(LoadProgress),
    ) -> Result<(Arc<LevelScene>, QueryWorld), String> {
        let scene = if id == TEST_ROOM_ID {
            Arc::new(LevelScene::test_room())
        } else {
            let game = self
                .game
                .as_ref()
                .ok_or_else(|| "there is no game data".to_string())?;
            let level = game
                .find(id)
                .ok_or_else(|| format!("no level named '{id}'"))?;
            game.load(level, progress)?
        };
        progress(LoadProgress {
            stage: LoadStage::Collision,
            done: 0,
            total: 1,
        });
        let physics = build_static(&scene).map_err(|e| format!("collision of {id}: {e}"))?;
        progress(LoadProgress {
            stage: LoadStage::Collision,
            done: 1,
            total: 1,
        });
        Ok((scene, physics))
    }

    /// The mounted install, when there is game data (T04-049: the player's view-model weapon reads its `.ep`
    /// and `.mdl` files straight from it, outside any level).
    pub(crate) fn vfs(&self) -> Option<&dyn Vfs> {
        self.game.as_ref().map(GameData::vfs)
    }

    /// The display name of `id`.
    pub(crate) fn name_of(&self, id: &str) -> String {
        self.entries()
            .into_iter()
            .find(|e| e.id == id)
            .map(|e| if e.label.is_empty() { e.id } else { e.label })
            .unwrap_or_else(|| id.to_string())
    }
}

/// The text to show for a level `.nfo` `NAME`. Many are text keys written `TTRS:<key>=<text>`; until there is a
/// localization table (none is specified yet) the text after the first `=` is shown, which is what the data
/// suggests is the default text. Other names are shown as they are; an empty result falls back to the id later.
pub(crate) fn display_name(name: &str) -> &str {
    match name
        .strip_prefix("TTRS:")
        .and_then(|rest| rest.split_once('='))
    {
        Some((_, text)) => text.trim(),
        None => name.trim(),
    }
}

enum Message {
    Progress(LoadProgress),
    Done(Box<Result<(Arc<LevelScene>, QueryWorld), String>>),
}

/// A level being loaded on a worker thread.
pub(crate) struct Loading {
    /// What the screen calls it.
    pub(crate) name: String,
    rx: Receiver<Message>,
    tracker: LoadingTracker,
    stages: [StageId; 4],
    /// The last count reported for each stage (the tracker takes increments).
    reported: [u64; 4],
}

impl Loading {
    /// Start loading `id` from `levels` on a new thread.
    pub(crate) fn start(levels: Arc<Levels>, id: &str) -> Self {
        let (tx, rx) = channel();
        let name = levels.name_of(id);
        let worker_id = id.to_string();
        std::thread::Builder::new()
            .name(format!("load {id}"))
            .spawn(move || {
                let progress_tx = tx.clone();
                let result = levels.load(&worker_id, &mut |p| {
                    let _ = progress_tx.send(Message::Progress(p));
                });
                let _ = tx.send(Message::Done(Box::new(result)));
            })
            .map(drop)
            .unwrap_or_else(|e| tracing::error!("cannot start the loading thread: {e}"));
        let mut tracker = LoadingTracker::new();
        let mut stage = |name, weight| {
            tracker
                .add_stage(name, weight)
                .expect("the stage weights are positive and finite")
        };
        let stages = [
            stage("Reading the world", 1.0),
            stage("Reading models", 4.0),
            stage("Reading textures", 4.0),
            stage("Building collision", 1.0),
        ];
        Self {
            name,
            rx,
            tracker,
            stages,
            reported: [0; 4],
        }
    }

    fn index(s: LoadStage) -> usize {
        match s {
            LoadStage::World => 0,
            LoadStage::Models => 1,
            LoadStage::Textures => 2,
            LoadStage::Collision => 3,
        }
    }

    /// Take what the worker reported, advance the bar by `dt` seconds, and return the result once it is done.
    pub(crate) fn poll(
        &mut self,
        dt: f32,
    ) -> Option<Result<(Arc<LevelScene>, QueryWorld), String>> {
        let mut done = None;
        loop {
            match self.rx.try_recv() {
                Ok(Message::Progress(p)) => {
                    let i = Self::index(p.stage);
                    // Every stage before this one is complete.
                    for earlier in &self.stages[..i] {
                        self.tracker.finish(*earlier);
                    }
                    let stage = self.stages[i];
                    self.tracker.set_total(stage, u64::from(p.total.max(1)));
                    let done = u64::from(p.done);
                    self.tracker
                        .advance(stage, done.saturating_sub(self.reported[i]));
                    self.reported[i] = self.reported[i].max(done);
                    if p.total == 0 || p.done >= p.total {
                        self.tracker.finish(stage);
                    }
                }
                Ok(Message::Done(result)) => {
                    if result.is_ok() {
                        for stage in self.stages {
                            self.tracker.finish(stage);
                        }
                    }
                    done = Some(*result);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    done = Some(Err("the loading thread stopped without a result".into()));
                    break;
                }
            }
        }
        self.tracker.update(dt);
        done
    }

    /// The loading screen: the level's name, a bar and the current stage.
    pub(crate) fn draw(&self, out: &mut DrawList) {
        out.clear();
        let w = 900.0;
        let x = (CANVAS_W - w) / 2.0;
        let y = CANVAS_H * 0.55;
        out.texts.push(TextRun {
            pos: Vec2 {
                x: CANVAS_W / 2.0,
                y: y - 140.0,
            },
            text: Cow::Owned(format!("Loading {}", self.name)),
            size: 52.0,
            color: theme::TEXT,
            align: TextAlign::Center,
        });
        out.quads.push(Quad {
            rect: Rect { x, y, w, h: 24.0 },
            color: theme::TRACK,
        });
        out.quads.push(Quad {
            rect: Rect {
                x,
                y,
                w: w * self.tracker.displayed().clamp(0.0, 1.0),
                h: 24.0,
            },
            color: theme::FOCUSED,
        });
        if let Some(status) = self.tracker.status() {
            out.texts.push(TextRun {
                pos: Vec2 {
                    x: CANVAS_W / 2.0,
                    y: y + 48.0,
                },
                text: Cow::Owned(status),
                size: 30.0,
                color: HINT,
                align: TextAlign::Center,
            });
        }
    }

    /// The bar's displayed share, 0 to 1.
    #[cfg(test)]
    pub(crate) fn displayed(&self) -> f32 {
        self.tracker.displayed()
    }
}

const HINT: Color = Color([0.85, 0.85, 0.9, 1.0]);

/// The line of help over the game: the level, the mode and the keys.
pub(crate) fn draw_hud(out: &mut DrawList, level: &str, flying: bool) {
    out.clear();
    let mode = if flying { "FLY" } else { "WALK" };
    out.texts.push(TextRun {
        pos: Vec2 {
            x: 24.0,
            y: CANVAS_H - 56.0,
        },
        text: Cow::Owned(format!(
            "{level}  |  {mode}  |  WASD move, mouse look, Space jump, F fly, Shift fast, Esc pause"
        )),
        size: 26.0,
        color: HINT,
        align: TextAlign::Left,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn wait(loading: &mut Loading) -> Result<(Arc<LevelScene>, QueryWorld), String> {
        let until = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(result) = loading.poll(0.016) {
                return result;
            }
            assert!(Instant::now() < until, "loading never finished");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn without_game_data_only_the_test_room_is_listed_and_it_loads() {
        let levels = Arc::new(Levels::new(None));
        let entries = levels.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, TEST_ROOM_ID);
        assert_eq!(levels.name_of(TEST_ROOM_ID), "Test room");
        let mut loading = Loading::start(levels.clone(), TEST_ROOM_ID);
        assert_eq!(loading.name, "Test room");
        let (scene, _physics) = wait(&mut loading).unwrap();
        assert_eq!(scene.name, "test room");
        for _ in 0..200 {
            loading.poll(0.05);
        }
        assert!(loading.displayed() > 0.99, "the bar runs to the end");
        let mut list = DrawList::default();
        loading.draw(&mut list);
        assert!(list.texts.iter().any(|t| t.text.contains("Test room")));
        assert_eq!(list.quads.len(), 2, "the track and the bar");
    }

    #[test]
    fn an_unknown_level_fails_with_its_name() {
        let levels = Arc::new(Levels::new(None));
        let mut loading = Loading::start(levels, "1_1_jungle");
        let Err(err) = wait(&mut loading) else {
            panic!("an unknown level must not load");
        };
        assert!(err.contains("no game data"), "{err}");
    }

    #[test]
    fn the_install_levels_come_first_and_load() {
        let dir = ss2_testkit::require_game_data!();
        let levels = Arc::new(Levels::new(Some(GameData::open(&dir).unwrap())));
        let entries = levels.entries();
        assert_eq!(entries.len(), 58 + 1);
        assert_eq!(entries.last().map(|e| e.id.as_str()), Some(TEST_ROOM_ID));
        let mut loading = Loading::start(levels, "dm_dockingbay");
        let (scene, _physics) = wait(&mut loading).unwrap();
        assert!(!scene.instances.is_empty());
    }

    #[test]
    fn text_keys_show_their_default_text() {
        assert_eq!(display_name("TTRS:1_1_Mdigbo_Jungle.Name=JUNGLE"), "JUNGLE");
        assert_eq!(display_name("TTRS:a=b=c"), "b=c", "only the first = splits");
        assert_eq!(display_name("TTRS:no_text"), "TTRS:no_text");
        assert_eq!(display_name(" Docking Bay "), "Docking Bay");
        assert_eq!(
            display_name("TTRS:k="),
            "",
            "empty: the picker shows the id"
        );
    }

    #[test]
    fn the_hud_names_the_level_and_the_mode() {
        let mut list = DrawList::default();
        draw_hud(&mut list, "Jungle", true);
        assert!(list.texts[0].text.contains("Jungle") && list.texts[0].text.contains("FLY"));
        draw_hud(&mut list, "Jungle", false);
        assert_eq!(list.texts.len(), 1, "cleared first");
        assert!(list.texts[0].text.contains("WALK"));
    }
}
