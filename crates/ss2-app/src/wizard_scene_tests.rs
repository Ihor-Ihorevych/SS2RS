//! The wizard scene with canned answers: the workers, the order of the screens, quitting, and the drawn screen.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ss2_core::input::NavInput;
use ss2_render::{RenderError, Renderer};
use ss2_ui::data_wizard::{
    DataWizard, WizardCandidate, WizardCommand, WizardInput, WizardReport, WizardView,
};
use ss2_ui::{UiInput, theme};

use super::{Answerer, Opener, Outcome, WizardScene};
use crate::data_location::Answer;
use crate::play::Levels;

const INSTALL: &str = "C:/Games/Serious Sam 2";

/// The commands the scene sent to its workers and the folders it opened.
#[derive(Default)]
struct Log {
    commands: Vec<WizardCommand>,
    opened: Vec<PathBuf>,
}

/// An answerer that finds one install, finds it usable and saves the choice, slowly enough that the "busy" screens are seen.
fn canned(log: &Arc<Mutex<Log>>) -> (Answerer, Opener) {
    let l = log.clone();
    let answerer: Answerer = Arc::new(move |command: WizardCommand| {
        std::thread::sleep(Duration::from_millis(20));
        l.lock().unwrap().commands.push(command.clone());
        match command {
            WizardCommand::Search => Answer::Candidates(vec![WizardCandidate {
                dir: PathBuf::from(INSTALL),
                detail: "the current folder - patch 2.100".into(),
            }]),
            WizardCommand::Validate(dir) => Answer::Report(WizardReport {
                dir,
                usable: true,
                warnings: 0,
                headline: "Serious Sam 2 found: the install looks complete".into(),
                lines: vec!["Archives: 14 found, 14 readable".into()],
            }),
            WizardCommand::Accept(dir) => Answer::Chosen {
                dir,
                save_error: None,
            },
            WizardCommand::Quit => Answer::Quit,
        }
    });
    let l = log.clone();
    let opener: Opener = Arc::new(move |dir: &Path| {
        l.lock().unwrap().opened.push(dir.to_path_buf());
        Levels::new(None)
    });
    (answerer, opener)
}

/// Poll until the workers are done; returns the outcome of the last poll.
fn settle(scene: &mut WizardScene) -> Outcome {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let outcome = scene.poll();
        if !matches!(outcome, Outcome::Continue) || !scene.busy() {
            return outcome;
        }
        assert!(Instant::now() < until, "the worker never answered");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn nav(n: NavInput) -> WizardInput {
    WizardInput::Ui(UiInput::Nav(n))
}

#[test]
fn search_check_accept_and_open_run_on_workers_while_the_screens_show() {
    let log = Arc::new(Mutex::new(Log::default()));
    let (answerer, opener) = canned(&log);
    let mut scene = WizardScene::new(DataWizard::new(None), answerer, opener);
    // Searching started at once, and the first poll does not wait for it.
    assert!(scene.busy());
    assert!(matches!(scene.poll(), Outcome::Continue));
    assert_eq!(scene.wizard().view(), WizardView::Searching);
    // Input while a worker runs is ignored.
    scene.handle(nav(NavInput::Back));
    assert!(matches!(settle(&mut scene), Outcome::Continue));
    assert_eq!(scene.wizard().view(), WizardView::Candidates);
    assert_eq!(
        scene.wizard().focus(),
        0,
        "the ignored Back did not move to Quit"
    );

    scene.handle(nav(NavInput::Activate));
    assert_eq!(scene.wizard().view(), WizardView::Validating);
    assert!(scene.busy());
    assert!(matches!(settle(&mut scene), Outcome::Continue));
    assert_eq!(scene.wizard().view(), WizardView::Result);

    scene.handle(nav(NavInput::Activate)); // Use this folder
    let Outcome::Ready(_) = settle(&mut scene) else {
        panic!("expected the folder to be opened");
    };
    assert_eq!(scene.wizard().view(), WizardView::Done);
    let log = log.lock().unwrap();
    assert_eq!(
        log.commands,
        [
            WizardCommand::Search,
            WizardCommand::Validate(PathBuf::from(INSTALL)),
            WizardCommand::Accept(PathBuf::from(INSTALL)),
        ]
    );
    assert_eq!(log.opened, [PathBuf::from(INSTALL)]);
}

#[test]
fn a_typed_path_is_checked_and_quit_needs_no_worker() {
    let log = Arc::new(Mutex::new(Log::default()));
    let (answerer, opener) = canned(&log);
    let mut scene = WizardScene::new(DataWizard::new(None), answerer, opener);
    settle(&mut scene);
    // The list has one folder, then "Enter a folder path".
    scene.handle(nav(NavInput::Down));
    scene.handle(nav(NavInput::Activate));
    assert!(scene.typing());
    scene.handle(WizardInput::Text("\"D:/SS2 copy\"".into()));
    scene.handle(nav(NavInput::Activate));
    assert!(!scene.typing());
    settle(&mut scene);
    assert_eq!(
        scene.wizard().report().map(|r| r.dir.clone()),
        Some(PathBuf::from("D:/SS2 copy"))
    );
    // Back to the path screen, back to the list, then Escape twice quits (the first only points at Quit).
    scene.handle(nav(NavInput::Back));
    scene.handle(nav(NavInput::Back));
    scene.handle(nav(NavInput::Back));
    assert!(matches!(scene.poll(), Outcome::Continue));
    scene.handle(nav(NavInput::Back));
    assert!(!scene.busy(), "quitting starts no worker");
    assert!(matches!(scene.poll(), Outcome::Quit));
    assert_eq!(
        log.lock().unwrap().commands.len(),
        2,
        "search and one check"
    );
}

#[test]
fn a_worker_that_dies_quits_instead_of_hanging() {
    let answerer: Answerer = Arc::new(|_| panic!("the disk exploded"));
    let opener: Opener = Arc::new(|_| Levels::new(None));
    let mut scene = WizardScene::new(DataWizard::new(None), answerer, opener);
    assert!(matches!(settle(&mut scene), Outcome::Quit));
}

/// The wizard screen as the window draws it, rendered offscreen. With `SS2_WIZARD_SHOT` set to a folder the images are saved there
/// to be looked at (they are not compared: the goldens of the screens themselves are `ss2-ui`'s).
#[test]
fn the_wizard_screens_render_offscreen() {
    let mut renderer = match Renderer::new_offscreen((640, 360)) {
        Ok(r) => r,
        Err(RenderError::NoAdapter(reason)) => {
            eprintln!("SKIPPED (no GPU adapter): {reason}");
            return;
        }
        Err(e) => panic!("{e}"),
    };
    let log = Arc::new(Mutex::new(Log::default()));
    let (answerer, opener) = canned(&log);
    let mut scene = WizardScene::new(
        DataWizard::new(Some(
            "Choose the folder where Serious Sam 2 is installed".into(),
        )),
        answerer,
        opener,
    );
    let mut list = ss2_ui::DrawList::default();
    let bg: Vec<u8> = theme::BACKGROUND
        .0
        .iter()
        .map(|c| (c * 255.0).round() as u8)
        .collect();
    let mut shoot = |scene: &WizardScene, name: &str| {
        scene.draw_into(&mut list);
        let image = renderer.render_to_image(&list, theme::BACKGROUND).unwrap();
        let drawn = image
            .pixels
            .chunks(4)
            .filter(|p| p.iter().zip(&bg).any(|(a, b)| a.abs_diff(*b) > 8))
            .count();
        assert!(
            drawn > 640 * 360 / 100,
            "{name}: something is drawn ({drawn} pixels)"
        );
        if let Some(dir) = std::env::var_os("SS2_WIZARD_SHOT") {
            crate::screenshot::save_png(&Path::new(&dir).join(format!("{name}.png")), &image)
                .unwrap();
        }
        drawn
    };
    let searching = shoot(&scene, "wizard-searching");
    settle(&mut scene);
    let candidates = shoot(&scene, "wizard-candidates");
    assert!(candidates > searching, "the list draws rows and buttons");
    scene.handle(nav(NavInput::Down));
    scene.handle(nav(NavInput::Activate));
    scene.handle(WizardInput::Text("C:/Games/Serious Sam 2".into()));
    shoot(&scene, "wizard-manual");
}
