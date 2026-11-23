//! Property tests for the navigation model (roadmap T10-005): random layouts and random input sequences against the invariants of
//! `ss2_ui::focus` (`docs/NAVIGATION.md`).

use proptest::prelude::*;
use ss2_core::input::NavInput;
use ss2_ui::Rect;
use ss2_ui::Vec2;
use ss2_ui::focus::{Dir, Focus, FocusEvent, FocusGraph, FocusId, FocusInput, FocusNode, Wrap};

fn rect_strategy() -> impl Strategy<Value = Rect> {
    (
        -500.0f32..2000.0,
        -500.0f32..1200.0,
        0.0f32..400.0,
        0.0f32..300.0,
    )
        .prop_map(|(x, y, w, h)| Rect { x, y, w, h })
}

/// A graph of 1 to 30 nodes with ids 0.. in insertion order and random rectangles; some disabled.
fn graph_strategy() -> impl Strategy<Value = FocusGraph> {
    prop::collection::vec((rect_strategy(), prop::bool::weighted(0.8)), 1..30).prop_map(|nodes| {
        let mut g = FocusGraph::new();
        for (i, (rect, enabled)) in nodes.into_iter().enumerate() {
            g.add(FocusNode::new(FocusId(i as u32), rect)).unwrap();
            g.set_enabled(FocusId(i as u32), enabled).unwrap();
        }
        g
    })
}

fn dir_strategy() -> impl Strategy<Value = Dir> {
    prop_oneof![
        Just(Dir::Up),
        Just(Dir::Down),
        Just(Dir::Left),
        Just(Dir::Right)
    ]
}

fn wrap_strategy() -> impl Strategy<Value = Wrap> {
    prop_oneof![Just(Wrap::None), Just(Wrap::Around)]
}

fn input_strategy() -> impl Strategy<Value = FocusInput> {
    let point = (-200.0f32..2200.0, -200.0f32..1400.0).prop_map(|(x, y)| Vec2 { x, y });
    prop_oneof![
        Just(FocusInput::Nav(NavInput::Up)),
        Just(FocusInput::Nav(NavInput::Down)),
        Just(FocusInput::Nav(NavInput::Left)),
        Just(FocusInput::Nav(NavInput::Right)),
        Just(FocusInput::Nav(NavInput::Activate)),
        Just(FocusInput::Nav(NavInput::Back)),
        point.clone().prop_map(FocusInput::PointerMoved),
        point.clone().prop_map(FocusInput::PointerPressed),
        point.prop_map(FocusInput::PointerReleased),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, failure_persistence: None, ..ProptestConfig::default() })]

    /// Navigation only ever leads to an existing, enabled node other than the one it started from.
    #[test]
    fn a_move_leads_to_a_different_enabled_node_or_nowhere(graph in graph_strategy(), from in 0u32..30, dir in dir_strategy(), wrap in wrap_strategy()) {
        if let Some(to) = graph.neighbor(FocusId(from), dir, wrap) {
            prop_assert_ne!(to, FocusId(from));
            let node = graph.get(to);
            prop_assert!(node.is_some_and(|n| n.enabled), "{to:?} is missing or disabled");
        }
    }

    /// Whatever the inputs, the focus is nothing or an enabled node of the graph, and an Activate names the focused node.
    #[test]
    fn the_focus_always_rests_on_an_enabled_node(graph in graph_strategy(), inputs in prop::collection::vec(input_strategy(), 1..60), wrap in wrap_strategy(), toggles in prop::collection::vec((0u32..30, any::<bool>()), 0..10)) {
        let mut graph = graph;
        let mut focus = Focus::new(wrap);
        for (step, input) in inputs.iter().enumerate() {
            // Now and then the screen changes under the focus.
            if let Some((id, on)) = toggles.get(step % 10).filter(|_| step % 7 == 0) {
                let _ = graph.set_enabled(FocusId(*id), *on);
            }
            let event = focus.handle(&graph, *input);
            if let Some(current) = focus.current() {
                prop_assert!(graph.get(current).is_some_and(|n| n.enabled), "step {step}: {current:?} is not an enabled node");
            }
            match event {
                FocusEvent::Moved { to, .. } => prop_assert_eq!(focus.current(), Some(to)),
                FocusEvent::Activate(id) => prop_assert!(graph.get(id).is_some_and(|n| n.enabled), "activated a node that cannot be"),
                FocusEvent::None | FocusEvent::Back => {}
            }
        }
    }

    /// On a regular grid the arrows do what they say: across a row, down a column, and back again.
    #[test]
    fn on_a_grid_the_arrows_walk_rows_and_columns_and_undo_each_other(cols in 1u32..8, rows in 1u32..8, col in 0u32..8, row in 0u32..8) {
        let (col, row) = (col % cols, row % rows);
        let mut g = FocusGraph::new();
        for r in 0..rows {
            for c in 0..cols {
                g.add(FocusNode::new(FocusId(r * cols + c), Rect { x: c as f32 * 150.0, y: r as f32 * 100.0, w: 100.0, h: 50.0 })).unwrap();
            }
        }
        let at = |c: u32, r: u32| FocusId(r * cols + c);
        // Walking right from the start of a row reaches its end, and no further without wrapping.
        let mut here = at(0, row);
        for c in 1..cols {
            here = g.neighbor(here, Dir::Right, Wrap::None).unwrap();
            prop_assert_eq!(here, at(c, row));
        }
        prop_assert_eq!(g.neighbor(here, Dir::Right, Wrap::None), None);
        // With wrapping, `cols` steps return to the start; the same for columns.
        let mut around = at(col, row);
        for _ in 0..cols {
            around = g.neighbor(around, Dir::Right, Wrap::Around).unwrap_or(around);
        }
        prop_assert_eq!(around, at(col, row));
        let mut around = at(col, row);
        for _ in 0..rows {
            around = g.neighbor(around, Dir::Down, Wrap::Around).unwrap_or(around);
        }
        prop_assert_eq!(around, at(col, row));
        // Down then up (and right then left) is where you started, when both exist.
        if let Some(down) = g.neighbor(at(col, row), Dir::Down, Wrap::None) {
            prop_assert_eq!(g.neighbor(down, Dir::Up, Wrap::None), Some(at(col, row)));
        }
        if let Some(right) = g.neighbor(at(col, row), Dir::Right, Wrap::None) {
            prop_assert_eq!(g.neighbor(right, Dir::Left, Wrap::None), Some(at(col, row)));
        }
    }

    /// The answer does not depend on the order the nodes were added in.
    #[test]
    fn insertion_order_changes_nothing(nodes in prop::collection::vec((rect_strategy(), prop::bool::weighted(0.8)), 2..20), rotate in 0usize..20, dir in dir_strategy(), wrap in wrap_strategy()) {
        let build = |order: &[usize]| {
            let mut g = FocusGraph::new();
            for &i in order {
                g.add(FocusNode::new(FocusId(i as u32), nodes[i].0)).unwrap();
                g.set_enabled(FocusId(i as u32), nodes[i].1).unwrap();
            }
            g
        };
        let straight: Vec<usize> = (0..nodes.len()).collect();
        let mut shuffled = straight.clone();
        shuffled.reverse();
        let r = rotate % shuffled.len();
        shuffled.rotate_left(r);
        let (a, b) = (build(&straight), build(&shuffled));
        for from in 0..nodes.len() as u32 {
            prop_assert_eq!(a.neighbor(FocusId(from), dir, wrap), b.neighbor(FocusId(from), dir, wrap), "from {}", from);
        }
        prop_assert_eq!(a.first(), b.first());
    }
}
