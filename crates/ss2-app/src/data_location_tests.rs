//! The first-run wizard from the first screen to the remembered choice, with the real wizard, the real install check and real folders,
//! and no window.

use std::fs;
use std::path::{Path, PathBuf};

use ss2_core::input::NavInput;
use ss2_testkit::fixtures::ZipBuilder;
use ss2_ui::UiInput;
use ss2_ui::data_wizard::{DataWizard, WizardCommand, WizardInput, WizardView};
use ss2_vfs::{Environment, LoadedChoice, Os, Startup, StartupSource, WizardReason, load_choice};

use crate::data_location::{Host, Step, forget, startup, store_path, unix_now, wizard_for};

const COMPLETE: [&str; 9] = [
    "All_PC_01.gro",
    "All_PC_02.gro",
    "All_PC_03.gro",
    "Movies_PC_01.gro",
    "Movies_PC_02.gro",
    "Patch_02_068.gro",
    "Patch_02_080_210639.gro",
    "Patch_02_090_593613.gro",
    "Patch_02_100_800000.gro",
];

/// The real folders, with the variables and the current folder of the test.
struct TestEnv {
    vars: Vec<(String, String)>,
    cwd: Option<PathBuf>,
}

impl TestEnv {
    fn none() -> Self {
        Self {
            vars: Vec::new(),
            cwd: None,
        }
    }
    fn in_dir(dir: &Path) -> Self {
        Self {
            vars: Vec::new(),
            cwd: Some(dir.to_path_buf()),
        }
    }
}

impl Environment for TestEnv {
    fn os(&self) -> Os {
        Os::current()
    }
    fn var(&self, name: &str) -> Option<String> {
        self.vars
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
    }
    fn current_dir(&self) -> Option<PathBuf> {
        self.cwd.clone()
    }
    fn exe_dir(&self) -> Option<PathBuf> {
        None
    }
    fn home_dir(&self) -> Option<PathBuf> {
        None
    }
    fn list_dir(&self, dir: &Path) -> Option<Vec<String>> {
        ss2_vfs::SystemEnvironment.list_dir(dir)
    }
    fn read_text(&self, _file: &Path) -> Option<String> {
        None
    }
    fn registry_string(&self, _key: &str, _value: &str) -> Option<String> {
        None
    }
}

fn make_install(dir: &Path, names: &[&str]) {
    fs::create_dir_all(dir).unwrap();
    for name in names {
        let bytes = ZipBuilder::new()
            .add("a.txt", b"x".to_vec())
            .finish()
            .unwrap();
        fs::write(dir.join(name), bytes).unwrap();
    }
}

fn press(w: &mut DataWizard, n: NavInput) -> Option<WizardCommand> {
    w.handle(WizardInput::Ui(UiInput::Nav(n)))
}

/// Press a key and, as the window would, run any command it gives; returns the last step.
fn drive(w: &mut DataWizard, host: &Host, n: NavInput) -> Step {
    let mut step = Step::Continue;
    let mut next = press(w, n);
    while let Some(command) = next.take() {
        step = host.run(w, command);
    }
    step
}

fn start(w: &mut DataWizard, host: &Host) {
    let command = w.start();
    assert_eq!(host.run(w, command), Step::Continue);
}

struct Rig {
    tmp: tempfile::TempDir,
}

impl Rig {
    fn new() -> Self {
        Self {
            tmp: tempfile::tempdir().unwrap(),
        }
    }
    fn store(&self) -> PathBuf {
        store_path(&self.tmp.path().join("config"))
    }
    fn dir(&self, name: &str) -> PathBuf {
        self.tmp.path().join(name)
    }
}

#[test]
fn first_run_find_check_accept_remember_and_the_next_start_goes_straight_in() {
    let rig = Rig::new();
    let install = rig.dir("Serious Sam 2");
    make_install(&install, &COMPLETE);
    let env = TestEnv::in_dir(&install);

    // First start: nothing remembered, so the wizard is needed.
    let Startup::NeedsWizard(reason) = startup(&env, None, &rig.store()) else {
        panic!("expected the wizard");
    };
    assert_eq!(reason, WizardReason::FirstRun);
    let mut w = wizard_for(&reason);
    let host = Host::new(&env, rig.store(), 1_700_000_123);
    start(&mut w, &host);
    assert_eq!(w.view(), WizardView::Candidates);
    assert_eq!(w.candidates().len(), 1);
    assert_eq!(w.candidates()[0].dir, install);
    assert_eq!(
        w.candidates()[0].detail,
        "current folder - patch 2.100 (build 800000)"
    );

    // Choose it: the check runs and the result is a clean pass.
    assert_eq!(drive(&mut w, &host, NavInput::Activate), Step::Continue);
    assert_eq!(w.view(), WizardView::Result);
    let report = w.report().unwrap();
    assert!(report.usable && report.warnings == 0, "{report:?}");
    assert_eq!(
        report.headline,
        "Serious Sam 2 found: the install looks complete"
    );

    // Use it: remembered with the time.
    assert_eq!(
        drive(&mut w, &host, NavInput::Activate),
        Step::Chosen(install.clone())
    );
    assert!(w.is_finished());
    let LoadedChoice::Found(choice) = load_choice(&rig.store()) else {
        panic!("nothing was remembered");
    };
    assert_eq!(
        (choice.dir.clone(), choice.validated_at),
        (install.clone(), 1_700_000_123)
    );

    // Next start, somewhere else entirely: the remembered folder is validated again and used.
    let elsewhere = TestEnv::none();
    let Startup::Ready {
        dir,
        source,
        report,
    } = startup(&elsewhere, None, &rig.store())
    else {
        panic!("expected Ready");
    };
    assert_eq!((dir, source), (install, StartupSource::Remembered));
    assert!(report.errors().is_empty());
}

#[test]
fn nothing_found_means_typing_the_path_and_a_wrong_folder_is_refused_with_advice() {
    let rig = Rig::new();
    let empty = rig.dir("empty");
    fs::create_dir_all(&empty).unwrap();
    let install = rig
        .dir("Spiele")
        .join("Serious Sam 2 \u{418}\u{433}\u{440}\u{44b}");
    make_install(&install, &COMPLETE);
    let env = TestEnv::none();
    let host = Host::new(&env, rig.store(), 5);
    let mut w = wizard_for(&WizardReason::FirstRun);
    start(&mut w, &host);
    assert!(w.candidates().is_empty());

    // "Enter a folder path", then a folder that has no game in it.
    assert_eq!(drive(&mut w, &host, NavInput::Activate), Step::Continue);
    assert_eq!(w.view(), WizardView::Manual);
    w.handle(WizardInput::Text(empty.display().to_string()));
    assert_eq!(drive(&mut w, &host, NavInput::Activate), Step::Continue);
    let report = w.report().unwrap();
    assert!(!report.usable, "{report:?}");
    assert!(
        report.lines.iter().any(|l| l.contains("no .gro archives")),
        "{:?}",
        report.lines
    );
    assert_eq!(w.view(), WizardView::Result);

    // The only ways on are Back and Quit: there is nothing to accept.
    assert_eq!(drive(&mut w, &host, NavInput::Back), Step::Continue);
    assert_eq!(w.view(), WizardView::Manual);
    assert!(!rig.store().exists(), "nothing was remembered");

    // Fix the typo: replace the text with the real folder, pasted the way Windows copies a path (in quotes), spaces and non-ASCII included.
    while !w.field().is_empty() {
        w.handle(WizardInput::Backspace);
    }
    w.handle(WizardInput::Text(format!("\"{}\"", install.display())));
    assert_eq!(drive(&mut w, &host, NavInput::Activate), Step::Continue);
    assert!(w.report().unwrap().usable);
    assert_eq!(
        drive(&mut w, &host, NavInput::Activate),
        Step::Chosen(install.clone())
    );
    assert_eq!(
        load_choice(&rig.store()),
        LoadedChoice::Found(ss2_vfs::RememberedChoice {
            dir: install,
            validated_at: 5
        })
    );
}

#[test]
fn an_old_patch_level_can_still_be_accepted_and_says_what_is_old() {
    let rig = Rig::new();
    let install = rig.dir("old");
    make_install(
        &install,
        &[
            "All_PC_01.gro",
            "All_PC_02.gro",
            "All_PC_03.gro",
            "Movies_PC_01.gro",
            "Movies_PC_02.gro",
            "Patch_02_068.gro",
        ],
    );
    let env = TestEnv::in_dir(&install);
    let host = Host::new(&env, rig.store(), 9);
    let mut w = wizard_for(&WizardReason::FirstRun);
    start(&mut w, &host);
    assert_eq!(w.candidates()[0].detail, "current folder - patch 2.068");
    drive(&mut w, &host, NavInput::Activate);
    let report = w.report().unwrap().clone();
    assert!(report.usable);
    assert_eq!(report.warnings, 1);
    assert_eq!(report.headline, "Usable, with 1 warning");
    assert!(
        report.lines.iter().any(|l| l.contains("older than 2.100")),
        "{:?}",
        report.lines
    );
    assert_eq!(
        drive(&mut w, &host, NavInput::Activate),
        Step::Chosen(install)
    );
}

#[test]
fn a_folder_missing_a_base_archive_cannot_be_accepted() {
    let rig = Rig::new();
    let install = rig.dir("broken");
    make_install(&install, &COMPLETE[1..]);
    // It is not even offered: discovery wants All_PC_01.gro. Typing it is possible and ends in a refusal.
    let env = TestEnv::in_dir(&install);
    let host = Host::new(&env, rig.store(), 9);
    let mut w = wizard_for(&WizardReason::FirstRun);
    start(&mut w, &host);
    assert!(w.candidates().is_empty());
    drive(&mut w, &host, NavInput::Activate);
    w.handle(WizardInput::Text(install.display().to_string()));
    drive(&mut w, &host, NavInput::Activate);
    let report = w.report().unwrap();
    assert!(!report.usable);
    assert!(
        report
            .lines
            .iter()
            .any(|l| l == "ERROR: All_PC_01.gro is missing"),
        "{:?}",
        report.lines
    );
    assert_eq!(
        drive(&mut w, &host, NavInput::Activate),
        Step::Continue,
        "the first button only goes back"
    );
    assert!(!rig.store().exists());
}

#[test]
fn a_remembered_folder_that_vanished_brings_the_wizard_back_with_the_reason_and_a_new_choice_replaces_it()
 {
    let rig = Rig::new();
    let old = rig.dir("old install");
    let new = rig.dir("new install");
    make_install(&old, &COMPLETE);
    make_install(&new, &COMPLETE);
    ss2_vfs::save_choice(
        &rig.store(),
        &ss2_vfs::RememberedChoice {
            dir: old.clone(),
            validated_at: 1,
        },
    )
    .unwrap();
    assert!(matches!(
        startup(&TestEnv::none(), None, &rig.store()),
        Startup::Ready { .. }
    ));
    fs::remove_dir_all(&old).unwrap();

    let env = TestEnv::in_dir(&new);
    let Startup::NeedsWizard(reason) = startup(&env, None, &rig.store()) else {
        panic!("expected the wizard");
    };
    assert!(
        matches!(reason, WizardReason::RememberedUnusable(_)),
        "{reason:?}"
    );
    let mut w = wizard_for(&reason);
    let host = Host::new(&env, rig.store(), 2);
    start(&mut w, &host);
    let message = w
        .draw()
        .texts
        .iter()
        .map(|t| t.text.to_string())
        .collect::<Vec<_>>()
        .join("|");
    assert!(message.contains("no longer works"), "{message}");
    assert_eq!(
        w.candidates().len(),
        1,
        "the vanished folder is not offered, the new one is"
    );
    assert_eq!(w.candidates()[0].dir, new);
    drive(&mut w, &host, NavInput::Activate);
    assert_eq!(
        drive(&mut w, &host, NavInput::Activate),
        Step::Chosen(new.clone())
    );
    assert_eq!(
        load_choice(&rig.store()),
        LoadedChoice::Found(ss2_vfs::RememberedChoice {
            dir: new,
            validated_at: 2
        })
    );
}

#[test]
fn a_damaged_config_file_is_survivable_and_overwritten_by_the_new_choice() {
    let rig = Rig::new();
    let install = rig.dir("sam");
    make_install(&install, &COMPLETE);
    fs::create_dir_all(rig.store().parent().unwrap()).unwrap();
    fs::write(rig.store(), "path = {{{ not toml").unwrap();
    let env = TestEnv::in_dir(&install);
    let Startup::NeedsWizard(reason) = startup(&env, None, &rig.store()) else {
        panic!("expected the wizard");
    };
    assert!(
        matches!(reason, WizardReason::ChoiceDamaged(_)),
        "{reason:?}"
    );
    let mut w = wizard_for(&reason);
    let host = Host::new(&env, rig.store(), 3);
    start(&mut w, &host);
    assert_eq!(w.candidates().len(), 1);
    drive(&mut w, &host, NavInput::Activate);
    assert_eq!(
        drive(&mut w, &host, NavInput::Activate),
        Step::Chosen(install.clone())
    );
    assert!(matches!(load_choice(&rig.store()), LoadedChoice::Found(c) if c.dir == install));
}

#[test]
fn a_choice_that_cannot_be_saved_still_goes_on_and_tells_the_player() {
    let rig = Rig::new();
    let install = rig.dir("sam");
    make_install(&install, &COMPLETE);
    let blocker = rig.dir("blocker");
    fs::write(&blocker, "a file where the config folder should be").unwrap();
    let env = TestEnv::in_dir(&install);
    let host = Host::new(&env, store_path(&blocker), 4);
    let mut w = wizard_for(&WizardReason::FirstRun);
    start(&mut w, &host);
    drive(&mut w, &host, NavInput::Activate);
    assert_eq!(
        drive(&mut w, &host, NavInput::Activate),
        Step::Chosen(install)
    );
    let notice = w.notice().expect("a notice");
    assert!(
        notice.contains("could not be saved") && notice.contains("asked again"),
        "{notice}"
    );
}

#[test]
fn the_remembered_folder_is_offered_first_with_every_source_that_named_it() {
    let rig = Rig::new();
    let install = rig.dir("sam");
    make_install(&install, &COMPLETE);
    ss2_vfs::save_choice(
        &rig.store(),
        &ss2_vfs::RememberedChoice {
            dir: install.clone(),
            validated_at: 1,
        },
    )
    .unwrap();
    // The same folder is also the current folder: one row, two sources, the remembered one first.
    let env = TestEnv::in_dir(&install);
    let host = Host::new(&env, rig.store(), 2);
    let mut w = wizard_for(&WizardReason::FirstRun);
    start(&mut w, &host);
    assert_eq!(w.candidates().len(), 1);
    assert_eq!(
        w.candidates()[0].detail,
        "remembered choice, current folder - patch 2.100 (build 800000)"
    );
}

#[test]
fn search_again_finds_a_folder_that_appeared_meanwhile_and_quit_quits() {
    let rig = Rig::new();
    let install = rig.dir("late");
    let env = TestEnv::in_dir(&install);
    let host = Host::new(&env, rig.store(), 6);
    let mut w = wizard_for(&WizardReason::FirstRun);
    start(&mut w, &host);
    assert!(w.candidates().is_empty());
    make_install(&install, &COMPLETE);
    // Down to "Search again" (row 1 of the three actions).
    press(&mut w, NavInput::Down);
    assert_eq!(drive(&mut w, &host, NavInput::Activate), Step::Continue);
    assert_eq!(w.view(), WizardView::Candidates);
    assert_eq!(w.candidates().len(), 1);
    // Escape twice leaves.
    assert_eq!(drive(&mut w, &host, NavInput::Back), Step::Continue);
    assert_eq!(drive(&mut w, &host, NavInput::Back), Step::Quit);
}

#[test]
fn the_variable_and_the_command_line_skip_the_wizard_and_a_bad_one_shows_it_with_the_reason() {
    let rig = Rig::new();
    let install = rig.dir("sam");
    make_install(&install, &COMPLETE);
    let mut env = TestEnv::none();
    env.vars
        .push(("SS2_DATA_DIR".into(), install.display().to_string()));
    assert!(matches!(
        startup(&env, None, &rig.store()),
        Startup::Ready {
            source: StartupSource::Environment,
            ..
        }
    ));
    let bad = rig.dir("typo");
    let Startup::NeedsWizard(reason) = startup(&env, Some(&bad), &rig.store()) else {
        panic!("expected the wizard");
    };
    assert!(reason.message().contains("--data"), "{}", reason.message());
}

/// Skips without `SS2_DATA_DIR`: the player's own install through the whole wizard.
#[test]
fn the_real_install_goes_through_the_wizard_and_is_remembered() {
    let data = ss2_testkit::require_game_data!();
    let rig = Rig::new();
    let mut env = TestEnv::none();
    env.vars
        .push(("SS2_DATA_DIR".into(), data.display().to_string()));
    let host = Host::new(&env, rig.store(), unix_now());
    let mut w = wizard_for(&WizardReason::FirstRun);
    start(&mut w, &host);
    let found: Vec<_> = w
        .candidates()
        .iter()
        .map(|c| (c.dir.clone(), c.detail.clone()))
        .collect();
    eprintln!("candidates: {found:?}");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, "SS2_DATA_DIR - patch 2.100 (build 800000)");
    let started = std::time::Instant::now();
    drive(&mut w, &host, NavInput::Activate);
    eprintln!("the wizard's check took {:?}", started.elapsed());
    let report = w.report().unwrap();
    assert!(report.usable && report.warnings == 0, "{report:?}");
    assert!(matches!(
        drive(&mut w, &host, NavInput::Activate),
        Step::Chosen(_)
    ));
    let started = std::time::Instant::now();
    let next = startup(&TestEnv::none(), None, &rig.store());
    eprintln!(
        "the next start validated the remembered choice in {:?}",
        started.elapsed()
    );
    assert!(
        matches!(
            next,
            Startup::Ready {
                source: StartupSource::Remembered,
                ..
            }
        ),
        "{next:?}"
    );
}

#[test]
fn forget_data_drops_the_remembered_folder_so_the_wizard_shows_again() {
    let rig = Rig::new();
    let install = rig.dir("Serious Sam 2");
    make_install(&install, &COMPLETE);
    let env = TestEnv::none();
    let host = Host::new(&env, rig.store(), 7);
    let mut w = wizard_for(&WizardReason::FirstRun);
    assert_eq!(
        host.run(&mut w, WizardCommand::Accept(install.clone())),
        Step::Chosen(install.clone())
    );
    assert!(matches!(
        startup(&env, None, &rig.store()),
        Startup::Ready {
            source: StartupSource::Remembered,
            ..
        }
    ));
    forget(&rig.store()).unwrap();
    assert_eq!(
        startup(&env, None, &rig.store()),
        Startup::NeedsWizard(WizardReason::FirstRun)
    );
    // Forgetting twice (no file) is fine, and --data still wins over a forgotten choice.
    forget(&rig.store()).unwrap();
    assert!(matches!(
        startup(&env, Some(&install), &rig.store()),
        Startup::Ready {
            source: StartupSource::Explicit,
            ..
        }
    ));
}

#[test]
fn answer_then_apply_is_what_run_does() {
    use crate::data_location::Answer;
    let rig = Rig::new();
    let env = TestEnv::none();
    let host = Host::new(
        &env,
        rig.dir("no-such-folder")
            .join("x")
            .join("data_location.toml"),
        7,
    );
    let mut w = wizard_for(&WizardReason::FirstRun);
    let _ = w.start();
    let found = host.answer(WizardCommand::Search);
    assert_eq!(found, Answer::Candidates(Vec::new()));
    assert_eq!(found.apply(&mut w), Step::Continue);
    assert_eq!(w.view(), WizardView::Candidates);
    assert_eq!(host.answer(WizardCommand::Quit).apply(&mut w), Step::Quit);
}
