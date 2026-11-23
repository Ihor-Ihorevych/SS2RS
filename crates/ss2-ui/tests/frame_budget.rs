//! The UI frame budget (roadmap T10-023): how many draw calls a screen may cost, and **zero allocations per frame once it
//! is warm** (`docs/UI_BUDGET.md`).
//!
//! A counting allocator replaces the global one for this test binary. It is one test function on purpose: the counter is
//! process-wide, so another test running on another thread would be counted too.

use ss2_core::input::NavInput;
use ss2_core::settings::Settings;
use ss2_ui::{DrawList, Menu, UiInput};

/// The only `unsafe` in the crate's tests (docs/CODING_STANDARDS.md, "Unsafe"): a global allocator that forwards every call to the system
/// allocator and counts the allocations. It is one small module, each `unsafe` block says why it is sound, and it needs the approval of the
/// crate owner and a second maintainer like any other `unsafe` (named in the pull request). There is no safe way to count allocations.
#[allow(unsafe_code)]
mod counting {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Counting;

    static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

    // SAFETY: every method forwards its arguments unchanged to `System`, which upholds the `GlobalAlloc` contract; the only addition is a relaxed counter.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            // SAFETY: `layout` is the caller's, passed on as received.
            unsafe { System.alloc(layout) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            // SAFETY: `ptr` and `layout` come from an earlier call of this allocator, which forwarded it to `System`.
            unsafe { System.dealloc(ptr, layout) }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            // SAFETY: `layout` is the caller's, passed on as received.
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            // SAFETY: `ptr` and `layout` come from an earlier call of this allocator (so from `System`), and `new_size` is the caller's.
            unsafe { System.realloc(ptr, layout, new_size) }
        }
    }

    #[global_allocator]
    static COUNTING: Counting = Counting;

    /// How many allocations this process has made so far.
    pub fn allocations() -> u64 {
        ALLOCATIONS.load(Ordering::Relaxed)
    }
}

use counting::allocations;

/// The budget of one frame of one screen: quads and text runs (each text run is a draw call in the renderer).
const MAX_QUADS: usize = 24;
const MAX_TEXTS: usize = 32;

/// A menu on each of its screens: the main menu, then options (down, activate), then back and about (down, down, activate).
fn screens() -> Vec<(&'static str, Menu)> {
    let nav = |m: &mut Menu, steps: &[NavInput]| {
        for s in steps {
            m.handle(UiInput::Nav(*s));
        }
    };
    let main = Menu::new(Settings::default());
    let mut options = Menu::new(Settings::default());
    nav(&mut options, &[NavInput::Down, NavInput::Activate]);
    let mut about = Menu::new(Settings::default());
    nav(
        &mut about,
        &[NavInput::Down, NavInput::Down, NavInput::Activate],
    );
    vec![("main", main), ("options", options), ("about", about)]
}

#[test]
fn a_warm_frame_allocates_nothing_and_stays_inside_the_draw_call_budget() {
    // A guard for the measurements below: the counter works, so a zero means something. The allocating entry point does allocate.
    {
        let menu = Menu::new(Settings::default());
        let before = allocations();
        let list = menu.draw();
        assert!(
            allocations() > before,
            "the counting allocator did not see Menu::draw allocate"
        );
        drop(list);
    }

    let mut report = String::new();
    for (name, mut menu) in screens() {
        let mut list = DrawList::default();
        // Warm up: the first frames may grow the lists.
        for _ in 0..4 {
            menu.draw_into(&mut list);
        }
        let (quads, texts) = (list.quads.len(), list.texts.len());
        assert!(
            quads <= MAX_QUADS && texts <= MAX_TEXTS,
            "{name}: {quads} quads and {texts} text runs exceed {MAX_QUADS} and {MAX_TEXTS}"
        );
        assert!(
            quads + texts > 0,
            "{name}: an empty screen would make the allocation count meaningless"
        );

        let before = allocations();
        for _ in 0..200 {
            menu.draw_into(&mut list);
        }
        let per_200 = allocations() - before;
        // What the allocating entry point costs per frame, for the record (it is not budgeted).
        let before = allocations();
        for _ in 0..200 {
            drop(menu.draw());
        }
        let old_per_frame = (allocations() - before) / 200;
        report.push_str(&format!(
            "{name}: {quads} quads, {texts} text runs, {per_200} allocations in 200 warm frames of draw_into ({old_per_frame} per frame through draw())\n"
        ));
        assert_eq!(per_200, 0, "{name} allocated in a warm frame:\n{report}");

        // Moving the focus and changing a setting must not make a warm frame allocate either.
        menu.handle(UiInput::Nav(NavInput::Down));
        menu.draw_into(&mut list);
        let before = allocations();
        for _ in 0..50 {
            menu.draw_into(&mut list);
        }
        assert_eq!(allocations() - before, 0, "{name} after moving the focus");
    }

    // The same drawing through the allocating entry point still gives the same list.
    for (name, menu) in screens() {
        let mut reused = DrawList::default();
        menu.draw_into(&mut reused);
        assert_eq!(reused, menu.draw(), "{name}");
        // And a list that held another screen is cleared first.
        let mut other = Menu::new(Settings::default()).draw();
        let leftover = other.quads.len();
        menu.draw_into(&mut other);
        assert_eq!(
            other,
            menu.draw(),
            "{name}: a reused list must not keep the previous frame (it had {leftover} quads)"
        );
    }
    println!("{report}");
}
