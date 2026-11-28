//! Headless tests of the app through `run_script` (no window, no renderer).

use std::time::{Duration, Instant};

use super::*;
use crate::play::TEST_ROOM_ID;
use ss2_platform_api::{GamepadId, ScriptStep, run_script};
use ss2_ui::ScreenId;

#[test]
fn without_a_file_or_overrides_the_platform_defaults_apply() {
    let s = effective_settings(None, &[]).unwrap();
    assert_eq!(
        s,
        Settings::for_platform(ss2_core::settings::PlatformClass::current())
    );
}

#[test]
fn overrides_are_applied_on_top_of_the_saved_settings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    std::fs::write(&path, "vsync = false\nmaster_volume = 0.3\n").unwrap();
    let overrides = ["fullscreen=1".to_owned(), "master_volume=0.9".to_owned()];
    let s = effective_settings(Some(&path), &overrides).unwrap();
    assert!(!s.vsync, "kept from the file");
    assert!(s.fullscreen, "set by the override");
    assert_eq!(s.master_volume, 0.9, "the override wins over the file");
}

#[test]
fn a_bad_override_is_an_error_naming_it() {
    let message = effective_settings(None, &["brightness=1".to_owned()]).unwrap_err();
    assert!(message.contains("brightness"), "{message}");
}

fn app() -> App {
    App::new(
        Settings::for_platform(ss2_core::settings::PlatformClass::current()),
        None,
        None,
        None,
        Levels::new(None),
    )
}

fn key(key: KeyCode, pressed: bool) -> ScriptStep {
    InputEvent::Key {
        key,
        pressed,
        repeat: false,
    }
    .into()
}

fn tap(k: KeyCode) -> [ScriptStep; 2] {
    [key(k, true), key(k, false)]
}

/// Run frames until the level has loaded (the worker thread needs real time).
fn until_playing(a: &mut App) {
    let until = Instant::now() + Duration::from_secs(20);
    while !a.is_playing() {
        assert!(Instant::now() < until, "the level never finished loading");
        run_script(a, [ScriptStep::Frame]);
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn session(a: &App) -> &PlaySession {
    match &a.phase {
        Some(Phase::Playing(p)) => &p.session,
        _ => panic!("not playing"),
    }
}

#[test]
fn closing_the_window_exits() {
    let mut a = app();
    let report = run_script(
        &mut a,
        [
            ScriptStep::Frame,
            LifecycleEvent::CloseRequested.into(),
            ScriptStep::Frame,
        ],
    );
    assert!(report.exited);
    assert_eq!(report.frames, 1);
}

#[test]
fn new_game_opens_the_picker_and_back_returns_to_the_main_menu() {
    let mut a = app();
    run_script(&mut a, tap(KeyCode::Enter));
    assert_eq!(a.state.scene, Scene::Playing);
    assert!(matches!(a.phase, Some(Phase::Picking(_))));
    run_script(&mut a, tap(KeyCode::Escape));
    assert_eq!(a.state.scene, Scene::Menu);
    assert_eq!(a.menu.screen(), ScreenId::Main);
    assert!(a.phase.is_none());
}

#[test]
fn picking_the_test_room_loads_it_and_walking_moves_through_it() {
    let mut a = app();
    run_script(&mut a, tap(KeyCode::Enter)); // New Game
    run_script(&mut a, tap(KeyCode::Enter)); // the only entry: the test room
    assert!(matches!(a.phase, Some(Phase::Loading(_))));
    until_playing(&mut a);
    let start = session(&a).feet();
    // Hold W for half a second, looking a little to the right with the mouse.
    let mut steps = vec![key(KeyCode::W, true)];
    steps.push(InputEvent::MouseMotion { dx: 40.0, dy: 0.0 }.into());
    steps.extend(std::iter::repeat_n(ScriptStep::Frame, 30));
    steps.push(key(KeyCode::W, false));
    run_script(&mut a, steps);
    let moved = session(&a).feet() - start;
    assert!(moved.z < -1.0, "walked forward (-Z): {moved}");
    assert!(moved.x > 0.0, "turned right a little: {moved}");
    assert!(session(&a).grounded());
}

#[test]
fn escape_pauses_resume_continues_and_quit_to_menu_drops_the_level() {
    let mut a = app();
    a.start_level = Some(TEST_ROOM_ID.to_string());
    until_playing(&mut a);
    run_script(&mut a, [key(KeyCode::W, true)]);
    run_script(&mut a, tap(KeyCode::Escape));
    assert_eq!(a.state.scene, Scene::Menu);
    assert_eq!(a.menu.screen(), ScreenId::Pause);
    // Paused: frames do not move the walker, and the held W was let go.
    let paused_at = session(&a).feet();
    run_script(&mut a, std::iter::repeat_n(ScriptStep::Frame, 20));
    assert_eq!(session(&a).feet(), paused_at);
    // Resume (the first pause item).
    run_script(&mut a, tap(KeyCode::Enter));
    assert!(a.is_playing());
    run_script(&mut a, std::iter::repeat_n(ScriptStep::Frame, 20));
    assert_eq!(session(&a).feet(), paused_at, "W was released by the pause");
    // Pause again and quit to the menu: the level is gone.
    run_script(&mut a, tap(KeyCode::Escape));
    run_script(
        &mut a,
        [
            tap(KeyCode::ArrowDown),
            tap(KeyCode::ArrowDown),
            tap(KeyCode::Enter),
        ]
        .into_iter()
        .flatten(),
    );
    assert_eq!(a.state.scene, Scene::Menu);
    assert!(a.phase.is_none());
}

#[test]
fn f_switches_to_flying_and_the_gamepad_start_button_pauses() {
    let mut a = app();
    a.start_level = Some(TEST_ROOM_ID.to_string());
    until_playing(&mut a);
    run_script(
        &mut a,
        tap(KeyCode::F).into_iter().chain([ScriptStep::Frame]),
    );
    assert_eq!(session(&a).mode(), Mode::Fly);
    let mut steps = vec![key(KeyCode::Space, true)];
    steps.extend(std::iter::repeat_n(ScriptStep::Frame, 30));
    run_script(&mut a, steps);
    assert!(session(&a).camera().position.y > 3.0, "flew up");
    run_script(
        &mut a,
        [InputEvent::GamepadButton {
            pad: GamepadId(0),
            button: GamepadButton::Start,
            pressed: true,
        }
        .into()],
    );
    assert_eq!(a.state.scene, Scene::Menu);
}

#[test]
fn escape_while_loading_goes_back_to_the_picker() {
    let mut a = app();
    run_script(&mut a, tap(KeyCode::Enter));
    run_script(&mut a, tap(KeyCode::Enter));
    assert!(matches!(a.phase, Some(Phase::Loading(_))));
    run_script(&mut a, tap(KeyCode::Escape));
    assert!(matches!(a.phase, Some(Phase::Picking(_))));
}

#[test]
fn quit_from_the_main_menu_exits() {
    let mut a = app();
    let report = run_script(
        &mut a,
        [
            key(KeyCode::ArrowDown, true),
            key(KeyCode::ArrowDown, true),
            key(KeyCode::ArrowDown, true),
            key(KeyCode::Enter, true),
            ScriptStep::Frame,
        ],
    );
    assert!(report.exited);
}

#[test]
fn the_gamepad_navigates_menus_with_a_held_stick_that_repeats() {
    let mut a = app();
    let start = a.menu.focused();
    let pad = GamepadId(0);
    run_script(
        &mut a,
        [
            InputEvent::GamepadAxis {
                pad,
                axis: GamepadAxis::LeftStickY,
                value: -1.0,
            }
            .into(),
            ScriptStep::Frame,
        ],
    );
    assert_ne!(a.menu.focused(), start, "the stick moved the focus down");
    let after_one = a.menu.focused();
    run_script(&mut a, std::iter::repeat_n(ScriptStep::Frame, 30));
    assert_ne!(a.menu.focused(), after_one, "held, it repeats");
    let mut b = app();
    run_script(
        &mut b,
        [InputEvent::GamepadButton {
            pad,
            button: GamepadButton::South,
            pressed: true,
        }
        .into()],
    );
    assert!(
        matches!(b.phase, Some(Phase::Picking(_))),
        "South activates New Game"
    );
}

#[test]
fn smoke_frames_exit_after_the_count() {
    let mut a = App::new(
        Settings::for_platform(ss2_core::settings::PlatformClass::current()),
        None,
        None,
        Some(3),
        Levels::new(None),
    );
    let report = run_script(&mut a, std::iter::repeat_n(ScriptStep::Frame, 10));
    assert_eq!((report.frames, report.exited), (3, true));
}

#[test]
fn f12_without_a_screenshot_folder_does_nothing() {
    let mut a = app();
    run_script(&mut a, [key(KeyCode::F12, true), ScriptStep::Frame]);
    assert!(!a.capture_pending);
    assert_eq!(a.state.scene, Scene::Menu);
}

/// The first-run wizard in the window, with the real host (discovery on a fake machine, the real install check on a real folder,
/// the choice saved in a real file): keys, typed text, the workers, then the menu with the new levels.
#[test]
fn the_wizard_shows_first_takes_a_typed_folder_and_then_opens_the_menu() {
    use crate::data_location::{Host, store_path};
    use crate::wizard_scene::{Answerer, Opener};
    use ss2_ui::data_wizard::{DataWizard, WizardView};
    use std::sync::Mutex;

    let tmp = tempfile::tempdir().unwrap();
    let install = tmp.path().join("Serious Sam 2 (copy)");
    std::fs::create_dir_all(&install).unwrap();
    for name in [
        "All_PC_01.gro",
        "All_PC_02.gro",
        "All_PC_03.gro",
        "Movies_PC_01.gro",
        "Movies_PC_02.gro",
        "Patch_02_068.gro",
        "Patch_02_080_210639.gro",
        "Patch_02_090_593613.gro",
        "Patch_02_100_800000.gro",
    ] {
        let bytes = ss2_testkit::fixtures::ZipBuilder::new()
            .add("a.txt", b"x".to_vec())
            .finish()
            .unwrap();
        std::fs::write(install.join(name), bytes).unwrap();
    }
    let store = store_path(&tmp.path().join("config"));
    // Nothing to find on this machine: the player types the path.
    let env = Arc::new(ss2_vfs::FakeEnvironment::new(ss2_vfs::Os::current()));
    let host_store = store.clone();
    let answerer: Answerer =
        Arc::new(move |command| Host::new(&*env, host_store.clone(), 42).answer(command));
    let opened = Arc::new(Mutex::new(None));
    let o = opened.clone();
    let opener: Opener = Arc::new(move |dir: &Path| {
        *o.lock().unwrap() = Some(dir.to_path_buf());
        Levels::new(None)
    });
    let mut a = app();
    a.wizard = Some(WizardScene::new(DataWizard::new(None), answerer, opener));
    a.start_level = Some(TEST_ROOM_ID.to_string());

    let view = |a: &App| a.wizard.as_ref().map(|w| w.wizard().view());
    let frames_until = |a: &mut App, done: &dyn Fn(&App) -> bool| {
        let until = Instant::now() + Duration::from_secs(20);
        while !done(a) {
            assert!(Instant::now() < until, "the wizard's worker never answered");
            run_script(a, [ScriptStep::Frame]);
            std::thread::sleep(Duration::from_millis(2));
        }
    };
    frames_until(&mut a, &|a| view(a) == Some(WizardView::Candidates));
    assert!(a.phase.is_none(), "--level waits for the wizard");
    // Nothing found: the first row is "Enter a folder path". Space opens it, and its space is not typed.
    let path = install.display().to_string();
    let mut steps: Vec<ScriptStep> = vec![
        key(KeyCode::Space, true),
        InputEvent::Text(" ".into()).into(),
    ];
    // The S key while the field has the focus does not move the focus as it does in the menus (its text would be typed).
    steps.push(key(KeyCode::S, true));
    for ch in path.chars() {
        steps.push(InputEvent::Text(ch.to_string()).into());
    }
    steps.push(key(KeyCode::Enter, true));
    run_script(&mut a, steps);
    assert_eq!(view(&a), Some(WizardView::Validating));
    frames_until(&mut a, &|a| view(a) == Some(WizardView::Result));
    run_script(&mut a, tap(KeyCode::Enter)); // Use this folder
    frames_until(&mut a, &|a| a.wizard.is_none());
    assert_eq!(opened.lock().unwrap().as_deref(), Some(install.as_path()));
    match ss2_vfs::load_choice(&store) {
        ss2_vfs::LoadedChoice::Found(choice) => assert_eq!(choice.dir, install),
        other => panic!("the choice was not saved: {other:?}"),
    }
    // --level starts in the frame that closed the wizard.
    assert!(matches!(
        a.phase,
        Some(Phase::Loading(_) | Phase::Playing(_))
    ));
    assert_eq!(a.state.scene, Scene::Playing);
}

#[test]
fn quitting_the_wizard_closes_the_window() {
    use crate::data_location::Answer;
    use crate::wizard_scene::{Answerer, Opener};
    use ss2_ui::data_wizard::DataWizard;

    let answerer: Answerer = Arc::new(|_| Answer::Candidates(Vec::new()));
    let opener: Opener = Arc::new(|_| Levels::new(None));
    let mut a = app();
    a.wizard = Some(WizardScene::new(DataWizard::new(None), answerer, opener));
    let until = Instant::now() + Duration::from_secs(10);
    while a.wizard.as_ref().is_some_and(WizardScene::busy) {
        assert!(Instant::now() < until);
        run_script(&mut a, [ScriptStep::Frame]);
        std::thread::sleep(Duration::from_millis(2));
    }
    let report = run_script(
        &mut a,
        [
            tap(KeyCode::Escape),
            tap(KeyCode::Escape),
            [ScriptStep::Frame, ScriptStep::Frame],
        ]
        .into_iter()
        .flatten(),
    );
    assert!(report.exited);
}
