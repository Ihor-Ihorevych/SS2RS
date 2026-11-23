//! The navigation model (roadmap T10-005): **which control has focus, and how mouse, keyboard and gamepad move it**, the same way for every
//! device.
//!
//! A screen describes its controls as a [`FocusGraph`]: nodes with a rectangle (canvas units), an enabled flag and, optionally, explicit links to
//! the node that up, down, left or right should go to. [`Focus`] holds the current node and turns [`FocusInput`] into [`FocusEvent`]s:
//!
//! * **Navigation** (`NavInput::Up/Down/Left/Right`): an explicit link wins; otherwise the nearest **enabled** node in that direction, chosen by
//!   distance along the axis plus **twice** the offset across it (so a node straight ahead beats a nearer one far off to the side); ties go to the lower
//!   id, so the choice never depends on the order nodes were added. With [`Wrap::Around`] going past the last node in a direction continues from the
//!   opposite side of the same row or column.
//! * **Activate** and **Back** are events for the screen to act on; the focus model does not know what a control does.
//! * **Pointer**: moving over an enabled node focuses it (hover is focus, as in `Menu`); pressing focuses it and remembers it; releasing over the **same** node
//!   activates it. A press that ends elsewhere does nothing.
//!
//! Nothing here knows about a device: a touch screen maps to the pointer inputs, a gamepad to `NavInput`, and a screen is written once. Pure and
//! deterministic (no clocks, no hash maps). [`Menu`](crate::Menu) still has its own list focus; moving it onto this model is follow-up T10-005b.

use ss2_core::input::NavInput;

use crate::geometry::{Rect, Vec2};

/// The most nodes a graph holds.
pub const MAX_NODES: usize = 4096;

/// A control's identity within a screen. Chosen by the screen; unique in a graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FocusId(pub u32);

/// A direction of navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    /// Toward smaller y.
    Up,
    /// Toward larger y.
    Down,
    /// Toward smaller x.
    Left,
    /// Toward larger x.
    Right,
}

impl Dir {
    /// The direction of a navigation input, if it is one of the four.
    pub fn from_nav(input: NavInput) -> Option<Self> {
        match input {
            NavInput::Up => Some(Self::Up),
            NavInput::Down => Some(Self::Down),
            NavInput::Left => Some(Self::Left),
            NavInput::Right => Some(Self::Right),
            _ => None,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Up => 0,
            Self::Down => 1,
            Self::Left => 2,
            Self::Right => 3,
        }
    }

    /// (along, across): how far `to` is in this direction and how far off to the side, measured between centres.
    fn measure(self, from: Vec2, to: Vec2) -> (f32, f32) {
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        match self {
            Self::Up => (-dy, dx.abs()),
            Self::Down => (dy, dx.abs()),
            Self::Left => (-dx, dy.abs()),
            Self::Right => (dx, dy.abs()),
        }
    }
}

/// What happens at the edge of the screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Wrap {
    /// Nothing: the focus stays where it is.
    #[default]
    None,
    /// Continue from the opposite side of the same row or column.
    Around,
}

/// One control.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FocusNode {
    /// Its identity.
    pub id: FocusId,
    /// Where it is, in canvas units.
    pub rect: Rect,
    /// Whether it can have focus.
    pub enabled: bool,
    links: [Option<FocusId>; 4],
}

impl FocusNode {
    /// An enabled node with no explicit links.
    pub fn new(id: FocusId, rect: Rect) -> Self {
        Self {
            id,
            rect,
            enabled: true,
            links: [None; 4],
        }
    }

    fn center(&self) -> Vec2 {
        Vec2 {
            x: self.rect.x + self.rect.w / 2.0,
            y: self.rect.y + self.rect.h / 2.0,
        }
    }
}

/// Why a node could not be added.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphError {
    /// A node with this id exists.
    DuplicateId(FocusId),
    /// The rectangle has a side that is negative or not finite.
    BadRect,
    /// The graph holds [`MAX_NODES`] nodes.
    Full,
    /// No node has this id.
    UnknownId(FocusId),
}

/// The controls of a screen.
///
/// ```
/// use ss2_core::input::NavInput;
/// use ss2_ui::focus::{Focus, FocusEvent, FocusGraph, FocusId, FocusInput, FocusNode};
/// use ss2_ui::Rect;
///
/// let mut graph = FocusGraph::new();
/// for (i, y) in [100.0, 200.0, 300.0].into_iter().enumerate() {
///     graph.add(FocusNode::new(FocusId(i as u32), Rect { x: 0.0, y, w: 100.0, h: 50.0 })).unwrap();
/// }
/// let mut focus = Focus::default();
/// focus.handle(&graph, FocusInput::Nav(NavInput::Down)); // nothing focused yet: the first control
/// assert_eq!(focus.current(), Some(FocusId(0)));
/// assert!(matches!(focus.handle(&graph, FocusInput::Nav(NavInput::Down)), FocusEvent::Moved { to: FocusId(1), .. }));
/// assert_eq!(focus.handle(&graph, FocusInput::Nav(NavInput::Activate)), FocusEvent::Activate(FocusId(1)));
/// ```
#[derive(Clone, Debug, Default)]
pub struct FocusGraph {
    nodes: Vec<FocusNode>,
}

impl FocusGraph {
    /// An empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Remove every node (keeps the allocation, so a frame loop can rebuild the graph without allocating).
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    /// How many nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Add a node.
    pub fn add(&mut self, node: FocusNode) -> Result<(), GraphError> {
        let r = node.rect;
        let finite = r.x.is_finite() && r.y.is_finite() && r.w.is_finite() && r.h.is_finite();
        if !finite || r.w < 0.0 || r.h < 0.0 {
            return Err(GraphError::BadRect);
        }
        if self.nodes.len() >= MAX_NODES {
            return Err(GraphError::Full);
        }
        if self.nodes.iter().any(|n| n.id == node.id) {
            return Err(GraphError::DuplicateId(node.id));
        }
        self.nodes.push(node);
        Ok(())
    }

    fn index_of(&self, id: FocusId) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// The node with this id.
    pub fn get(&self, id: FocusId) -> Option<&FocusNode> {
        self.index_of(id).map(|i| &self.nodes[i])
    }

    /// Enable or disable a node.
    pub fn set_enabled(&mut self, id: FocusId, enabled: bool) -> Result<(), GraphError> {
        let i = self.index_of(id).ok_or(GraphError::UnknownId(id))?;
        self.nodes[i].enabled = enabled;
        Ok(())
    }

    /// Make `from` go to `to` when `dir` is pressed, whatever the layout says. `to` need not exist yet; a link to a missing or
    /// disabled node is ignored at navigation time and the layout decides.
    pub fn link(&mut self, from: FocusId, dir: Dir, to: FocusId) -> Result<(), GraphError> {
        let i = self.index_of(from).ok_or(GraphError::UnknownId(from))?;
        self.nodes[i].links[dir.index()] = Some(to);
        Ok(())
    }

    /// The enabled node under `point` (the one added last if several overlap).
    pub fn hit(&self, point: Vec2) -> Option<FocusId> {
        self.nodes
            .iter()
            .rev()
            .find(|n| n.enabled && n.rect.contains(point))
            .map(|n| n.id)
    }

    /// The enabled node nearest the top left: the first thing to focus. Rows are ordered by the top edge, then left to right.
    pub fn first(&self) -> Option<FocusId> {
        self.nodes
            .iter()
            .filter(|n| n.enabled)
            .min_by(|a, b| {
                (a.rect.y, a.rect.x, a.id)
                    .partial_cmp(&(b.rect.y, b.rect.x, b.id))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|n| n.id)
    }

    /// Where `dir` leads from `from`: an explicit link to an enabled node, else the best enabled node in that direction, else (with
    /// [`Wrap::Around`]) the best one from the opposite side, else `None`.
    pub fn neighbor(&self, from: FocusId, dir: Dir, wrap: Wrap) -> Option<FocusId> {
        let node = self.get(from)?;
        if let Some(link) = node.links[dir.index()]
            .and_then(|id| self.get(id))
            .filter(|n| n.enabled && n.id != from)
        {
            return Some(link.id);
        }
        let origin = node.center();
        // Score: along + 2 * across, the lowest wins; equal scores go to the lower id.
        let mut best: Option<(f32, FocusId)> = None;
        let mut opposite: Option<(f32, f32, FocusId)> = None; // (across, -distance behind, id)
        for other in self.nodes.iter().filter(|n| n.enabled && n.id != from) {
            let (along, across) = dir.measure(origin, other.center());
            if along > 0.0 {
                let score = along + 2.0 * across;
                if best.is_none_or(|(s, id)| score < s || (score == s && other.id < id)) {
                    best = Some((score, other.id));
                }
            } else if along < 0.0 && wrap == Wrap::Around {
                let key = (across, along, other.id); // the same column or row first, then the farthest behind
                if opposite.is_none_or(|(a, l, id)| (key.0, key.1, key.2) < (a, l, id)) {
                    opposite = Some(key);
                }
            }
        }
        best.map(|(_, id)| id).or(opposite.map(|(_, _, id)| id))
    }
}

/// An input to the focus model, from any device.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FocusInput {
    /// Keyboard or gamepad navigation.
    Nav(NavInput),
    /// The pointer (mouse or a touch) moved.
    PointerMoved(Vec2),
    /// The pointer button went down (or a finger touched).
    PointerPressed(Vec2),
    /// The pointer button came up (or the finger lifted).
    PointerReleased(Vec2),
}

/// What an input did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusEvent {
    /// Nothing changed (a move that is blocked, a release elsewhere, a press on empty space).
    None,
    /// The focus moved.
    Moved {
        /// Where it was.
        from: Option<FocusId>,
        /// Where it is now.
        to: FocusId,
    },
    /// The focused control was activated (confirm, or a release over the control that was pressed).
    Activate(FocusId),
    /// Back / cancel.
    Back,
}

/// Which control has focus, and what a press is waiting for.
#[derive(Clone, Copy, Debug, Default)]
pub struct Focus {
    current: Option<FocusId>,
    pressed: Option<FocusId>,
    wrap: Wrap,
}

impl Focus {
    /// A focus with no control focused and the given edge behaviour.
    pub fn new(wrap: Wrap) -> Self {
        Self {
            current: None,
            pressed: None,
            wrap,
        }
    }

    /// The focused control.
    pub fn current(&self) -> Option<FocusId> {
        self.current
    }

    /// Focus a control directly (a screen opening on a default button). `false` if it does not exist or is disabled.
    pub fn set(&mut self, graph: &FocusGraph, id: FocusId) -> bool {
        if graph.get(id).is_some_and(|n| n.enabled) {
            self.current = Some(id);
            true
        } else {
            false
        }
    }

    /// After the graph changed (a screen rebuilt, a control disabled): if the focused control is gone or disabled, focus the first enabled one
    /// (or nothing). Returns the new focus if it changed.
    pub fn refocus(&mut self, graph: &FocusGraph) -> Option<FocusId> {
        if self
            .pressed
            .is_some_and(|p| graph.get(p).is_none_or(|n| !n.enabled))
        {
            self.pressed = None;
        }
        if self
            .current
            .is_some_and(|c| graph.get(c).is_some_and(|n| n.enabled))
        {
            return None;
        }
        self.current = graph.first();
        self.current
    }

    fn moved_to(&mut self, to: FocusId) -> FocusEvent {
        let from = self.current;
        if from == Some(to) {
            return FocusEvent::None;
        }
        self.current = Some(to);
        FocusEvent::Moved { from, to }
    }

    /// Handle one input against the current graph.
    pub fn handle(&mut self, graph: &FocusGraph, input: FocusInput) -> FocusEvent {
        self.refocus_if_stale(graph);
        match input {
            FocusInput::Nav(NavInput::Activate) => {
                self.current.map_or(FocusEvent::None, FocusEvent::Activate)
            }
            FocusInput::Nav(NavInput::Back) => FocusEvent::Back,
            FocusInput::Nav(nav) => {
                let Some(dir) = Dir::from_nav(nav) else {
                    return FocusEvent::None;
                };
                match self.current {
                    None => graph
                        .first()
                        .map_or(FocusEvent::None, |first| self.moved_to(first)),
                    Some(from) => graph
                        .neighbor(from, dir, self.wrap)
                        .map_or(FocusEvent::None, |to| self.moved_to(to)),
                }
            }
            FocusInput::PointerMoved(p) => graph
                .hit(p)
                .map_or(FocusEvent::None, |id| self.moved_to(id)),
            FocusInput::PointerPressed(p) => {
                self.pressed = graph.hit(p);
                self.pressed
                    .map_or(FocusEvent::None, |id| self.moved_to(id))
            }
            FocusInput::PointerReleased(p) => {
                let pressed = self.pressed.take();
                match (pressed, graph.hit(p)) {
                    (Some(a), Some(b)) if a == b => {
                        self.current = Some(a);
                        FocusEvent::Activate(a)
                    }
                    _ => FocusEvent::None,
                }
            }
        }
    }

    fn refocus_if_stale(&mut self, graph: &FocusGraph) {
        if self
            .current
            .is_some_and(|c| graph.get(c).is_none_or(|n| !n.enabled))
        {
            self.current = None;
        }
        if self
            .pressed
            .is_some_and(|p| graph.get(p).is_none_or(|n| !n.enabled))
        {
            self.pressed = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32) -> Rect {
        Rect {
            x,
            y,
            w: 100.0,
            h: 50.0,
        }
    }

    /// A grid of `cols` by `rows` controls, ids in reading order (row * cols + col), 150 apart.
    fn grid(cols: u32, rows: u32) -> FocusGraph {
        let mut g = FocusGraph::new();
        for r in 0..rows {
            for c in 0..cols {
                g.add(FocusNode::new(
                    FocusId(r * cols + c),
                    rect(c as f32 * 150.0, r as f32 * 100.0),
                ))
                .unwrap();
            }
        }
        g
    }

    fn id(n: u32) -> FocusId {
        FocusId(n)
    }

    // ---- the graph --------------------------------------------------------------------------

    #[test]
    fn nodes_are_checked_when_added() {
        let mut g = FocusGraph::new();
        g.add(FocusNode::new(id(1), rect(0.0, 0.0))).unwrap();
        assert_eq!(
            g.add(FocusNode::new(id(1), rect(5.0, 5.0))),
            Err(GraphError::DuplicateId(id(1)))
        );
        for bad in [
            Rect {
                x: f32::NAN,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            },
            Rect {
                x: 0.0,
                y: f32::INFINITY,
                w: 1.0,
                h: 1.0,
            },
            Rect {
                x: 0.0,
                y: 0.0,
                w: -1.0,
                h: 1.0,
            },
            Rect {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: f32::NAN,
            },
        ] {
            assert_eq!(
                g.add(FocusNode::new(id(9), bad)),
                Err(GraphError::BadRect),
                "{bad:?}"
            );
        }
        assert_eq!(g.len(), 1);
        let mut full = FocusGraph::new();
        for i in 0..MAX_NODES as u32 {
            full.add(FocusNode::new(id(i), rect(i as f32, 0.0)))
                .unwrap();
        }
        assert_eq!(
            full.add(FocusNode::new(id(MAX_NODES as u32), rect(0.0, 0.0))),
            Err(GraphError::Full)
        );
        assert_eq!(
            g.set_enabled(id(77), false),
            Err(GraphError::UnknownId(id(77)))
        );
        assert_eq!(
            g.link(id(77), Dir::Up, id(1)),
            Err(GraphError::UnknownId(id(77)))
        );
        g.clear();
        assert!(g.is_empty());
    }

    #[test]
    fn a_zero_sized_node_is_allowed_and_a_pointer_cannot_hit_it() {
        let mut g = FocusGraph::new();
        g.add(FocusNode::new(
            id(1),
            Rect {
                x: 10.0,
                y: 10.0,
                w: 0.0,
                h: 0.0,
            },
        ))
        .unwrap();
        assert_eq!(g.hit(Vec2 { x: 10.0, y: 10.0 }), None);
        assert_eq!(
            g.first(),
            Some(id(1)),
            "it can still be reached by navigation"
        );
    }

    #[test]
    fn hit_finds_the_enabled_node_under_the_point_and_the_last_added_wins_an_overlap() {
        let mut g = FocusGraph::new();
        g.add(FocusNode::new(id(1), rect(0.0, 0.0))).unwrap();
        g.add(FocusNode::new(id(2), rect(50.0, 0.0))).unwrap(); // overlaps 1 from x = 50 to 100
        assert_eq!(g.hit(Vec2 { x: 20.0, y: 10.0 }), Some(id(1)));
        assert_eq!(
            g.hit(Vec2 { x: 75.0, y: 10.0 }),
            Some(id(2)),
            "the one drawn on top"
        );
        g.set_enabled(id(2), false).unwrap();
        assert_eq!(
            g.hit(Vec2 { x: 75.0, y: 10.0 }),
            Some(id(1)),
            "a disabled node is skipped"
        );
        assert_eq!(g.hit(Vec2 { x: 500.0, y: 10.0 }), None);
        assert_eq!(
            g.hit(Vec2 { x: 100.0, y: 10.0 }),
            None,
            "the right edge is outside"
        );
        assert_eq!(
            g.hit(Vec2 { x: 0.0, y: 0.0 }),
            Some(id(1)),
            "the left and top edges are inside"
        );
    }

    #[test]
    fn first_is_the_top_left_enabled_node_whatever_order_they_were_added_in() {
        let mut g = FocusGraph::new();
        g.add(FocusNode::new(id(3), rect(300.0, 0.0))).unwrap();
        g.add(FocusNode::new(id(1), rect(0.0, 100.0))).unwrap();
        g.add(FocusNode::new(id(2), rect(100.0, 0.0))).unwrap();
        assert_eq!(g.first(), Some(id(2)), "the top row, then the leftmost");
        g.set_enabled(id(2), false).unwrap();
        assert_eq!(g.first(), Some(id(3)));
        g.set_enabled(id(3), false).unwrap();
        g.set_enabled(id(1), false).unwrap();
        assert_eq!(g.first(), None);
        assert_eq!(FocusGraph::new().first(), None);
    }

    // ---- navigation -------------------------------------------------------------------------

    #[test]
    fn arrows_move_through_a_grid_and_stop_at_the_edge() {
        let g = grid(3, 3); // ids 0..9, 3 per row
        assert_eq!(g.neighbor(id(4), Dir::Right, Wrap::None), Some(id(5)));
        assert_eq!(g.neighbor(id(4), Dir::Left, Wrap::None), Some(id(3)));
        assert_eq!(g.neighbor(id(4), Dir::Up, Wrap::None), Some(id(1)));
        assert_eq!(g.neighbor(id(4), Dir::Down, Wrap::None), Some(id(7)));
        assert_eq!(g.neighbor(id(0), Dir::Left, Wrap::None), None);
        assert_eq!(g.neighbor(id(0), Dir::Up, Wrap::None), None);
        assert_eq!(g.neighbor(id(8), Dir::Right, Wrap::None), None);
        assert_eq!(g.neighbor(id(8), Dir::Down, Wrap::None), None);
        assert_eq!(
            g.neighbor(id(99), Dir::Down, Wrap::None),
            None,
            "an unknown id leads nowhere"
        );
    }

    #[test]
    fn a_node_straight_ahead_beats_a_nearer_one_off_to_the_side() {
        let mut g = FocusGraph::new();
        g.add(FocusNode::new(id(0), rect(0.0, 0.0))).unwrap();
        g.add(FocusNode::new(id(1), rect(170.0, 75.0))).unwrap(); // nearer along the axis but far to the side (415 at weight 2, 245 at weight 1; the node straight below is 300)
        g.add(FocusNode::new(id(2), rect(0.0, 300.0))).unwrap(); // straight down
        assert_eq!(g.neighbor(id(0), Dir::Down, Wrap::None), Some(id(2)));
        // ... but a node only slightly off beats a distant straight one.
        let mut h = FocusGraph::new();
        h.add(FocusNode::new(id(0), rect(0.0, 0.0))).unwrap();
        h.add(FocusNode::new(id(1), rect(20.0, 100.0))).unwrap();
        h.add(FocusNode::new(id(2), rect(0.0, 900.0))).unwrap();
        assert_eq!(h.neighbor(id(0), Dir::Down, Wrap::None), Some(id(1)));
    }

    #[test]
    fn disabled_nodes_are_skipped_and_the_next_one_is_found() {
        let mut g = grid(3, 1);
        g.set_enabled(id(1), false).unwrap();
        assert_eq!(g.neighbor(id(0), Dir::Right, Wrap::None), Some(id(2)));
        g.set_enabled(id(2), false).unwrap();
        assert_eq!(g.neighbor(id(0), Dir::Right, Wrap::None), None);
    }

    #[test]
    fn explicit_links_win_over_the_layout_but_only_if_the_target_can_take_focus() {
        let mut g = grid(3, 1);
        g.link(id(0), Dir::Right, id(2)).unwrap(); // skip over 1
        assert_eq!(g.neighbor(id(0), Dir::Right, Wrap::None), Some(id(2)));
        g.set_enabled(id(2), false).unwrap();
        assert_eq!(
            g.neighbor(id(0), Dir::Right, Wrap::None),
            Some(id(1)),
            "a link to a disabled node falls back to the layout"
        );
        g.link(id(0), Dir::Right, id(99)).unwrap();
        assert_eq!(
            g.neighbor(id(0), Dir::Right, Wrap::None),
            Some(id(1)),
            "so does a link to a node that does not exist"
        );
        g.link(id(0), Dir::Left, id(0)).unwrap();
        assert_eq!(
            g.neighbor(id(0), Dir::Left, Wrap::None),
            None,
            "a link to itself is ignored"
        );
        // A link also reaches against the geometry.
        g.link(id(0), Dir::Up, id(1)).unwrap();
        assert_eq!(g.neighbor(id(0), Dir::Up, Wrap::None), Some(id(1)));
    }

    #[test]
    fn wrapping_continues_from_the_opposite_side_of_the_same_row_or_column() {
        let g = grid(3, 3);
        assert_eq!(g.neighbor(id(2), Dir::Right, Wrap::Around), Some(id(0)));
        assert_eq!(g.neighbor(id(3), Dir::Left, Wrap::Around), Some(id(5)));
        assert_eq!(g.neighbor(id(7), Dir::Down, Wrap::Around), Some(id(1)));
        assert_eq!(g.neighbor(id(1), Dir::Up, Wrap::Around), Some(id(7)));
        // Where there is something ahead, wrapping changes nothing.
        assert_eq!(g.neighbor(id(0), Dir::Right, Wrap::Around), Some(id(1)));
        // A lone node wraps to nothing.
        let mut one = FocusGraph::new();
        one.add(FocusNode::new(id(0), rect(0.0, 0.0))).unwrap();
        assert_eq!(one.neighbor(id(0), Dir::Down, Wrap::Around), None);
    }

    #[test]
    fn wrapping_in_a_ragged_layout_picks_the_nearest_column_then_the_farthest_side() {
        let mut g = FocusGraph::new();
        g.add(FocusNode::new(id(0), rect(0.0, 0.0))).unwrap();
        g.add(FocusNode::new(id(1), rect(0.0, 100.0))).unwrap();
        g.add(FocusNode::new(id(2), rect(300.0, -200.0))).unwrap(); // a different column, and farther behind
        assert_eq!(
            g.neighbor(id(1), Dir::Down, Wrap::Around),
            Some(id(0)),
            "the same column, not the farther one in the other column"
        );
    }

    #[test]
    fn ties_go_to_the_lower_id_so_the_insertion_order_never_matters() {
        let mk = |order: &[u32]| {
            let mut g = FocusGraph::new();
            for &i in order {
                // 1 and 2 are mirror images either side of 0, equally good for Down.
                let r = match i {
                    0 => rect(100.0, 0.0),
                    1 => rect(0.0, 100.0),
                    _ => rect(200.0, 100.0),
                };
                g.add(FocusNode::new(id(i), r)).unwrap();
            }
            g
        };
        for order in [[0, 1, 2], [0, 2, 1], [2, 1, 0], [1, 0, 2]] {
            assert_eq!(
                mk(&order).neighbor(id(0), Dir::Down, Wrap::None),
                Some(id(1)),
                "{order:?}"
            );
        }
    }

    // ---- the Focus state ----------------------------------------------------------------------

    #[test]
    fn the_first_move_focuses_the_first_control_and_later_moves_follow_the_graph() {
        let g = grid(2, 2);
        let mut f = Focus::default();
        assert_eq!(
            f.handle(&g, FocusInput::Nav(NavInput::Right)),
            FocusEvent::Moved {
                from: None,
                to: id(0)
            }
        );
        assert_eq!(
            f.handle(&g, FocusInput::Nav(NavInput::Right)),
            FocusEvent::Moved {
                from: Some(id(0)),
                to: id(1)
            }
        );
        assert_eq!(
            f.handle(&g, FocusInput::Nav(NavInput::Right)),
            FocusEvent::None,
            "blocked at the edge"
        );
        assert_eq!(f.current(), Some(id(1)));
        assert_eq!(
            f.handle(&g, FocusInput::Nav(NavInput::Down)),
            FocusEvent::Moved {
                from: Some(id(1)),
                to: id(3)
            }
        );
        assert_eq!(
            f.handle(&g, FocusInput::Nav(NavInput::Activate)),
            FocusEvent::Activate(id(3))
        );
        assert_eq!(
            f.handle(&g, FocusInput::Nav(NavInput::Back)),
            FocusEvent::Back
        );
        let mut empty = Focus::default();
        assert_eq!(
            empty.handle(&FocusGraph::new(), FocusInput::Nav(NavInput::Down)),
            FocusEvent::None
        );
        assert_eq!(
            empty.handle(&FocusGraph::new(), FocusInput::Nav(NavInput::Activate)),
            FocusEvent::None,
            "nothing to activate"
        );
    }

    #[test]
    fn wrap_around_is_a_setting_of_the_focus() {
        let g = grid(3, 1);
        let mut f = Focus::new(Wrap::Around);
        f.set(&g, id(2));
        assert_eq!(
            f.handle(&g, FocusInput::Nav(NavInput::Right)),
            FocusEvent::Moved {
                from: Some(id(2)),
                to: id(0)
            }
        );
    }

    #[test]
    fn hovering_focuses_pressing_focuses_and_a_click_activates_only_on_the_same_control() {
        let g = grid(2, 1);
        let (a, b, nowhere) = (
            Vec2 { x: 10.0, y: 10.0 },
            Vec2 { x: 160.0, y: 10.0 },
            Vec2 { x: 900.0, y: 900.0 },
        );
        let mut f = Focus::default();
        assert_eq!(
            f.handle(&g, FocusInput::PointerMoved(a)),
            FocusEvent::Moved {
                from: None,
                to: id(0)
            }
        );
        assert_eq!(
            f.handle(&g, FocusInput::PointerMoved(a)),
            FocusEvent::None,
            "still over the same one"
        );
        assert_eq!(
            f.handle(&g, FocusInput::PointerMoved(nowhere)),
            FocusEvent::None,
            "leaving keeps the focus where it was"
        );
        assert_eq!(f.current(), Some(id(0)));
        // A click on b.
        assert_eq!(
            f.handle(&g, FocusInput::PointerPressed(b)),
            FocusEvent::Moved {
                from: Some(id(0)),
                to: id(1)
            }
        );
        assert_eq!(
            f.handle(&g, FocusInput::PointerReleased(b)),
            FocusEvent::Activate(id(1))
        );
        // Pressed on b, released on a: nothing.
        f.handle(&g, FocusInput::PointerPressed(b));
        assert_eq!(
            f.handle(&g, FocusInput::PointerReleased(a)),
            FocusEvent::None
        );
        assert_eq!(
            f.handle(&g, FocusInput::PointerReleased(a)),
            FocusEvent::None,
            "and a release with no press is nothing"
        );
        // Pressed on empty space, released on a control: nothing.
        f.handle(&g, FocusInput::PointerPressed(nowhere));
        assert_eq!(
            f.handle(&g, FocusInput::PointerReleased(a)),
            FocusEvent::None
        );
    }

    #[test]
    fn one_screen_works_the_same_with_mouse_keyboard_and_gamepad() {
        let g = grid(3, 1);
        let mut keyboard = Focus::default();
        let mut mouse = Focus::default();
        keyboard.handle(&g, FocusInput::Nav(NavInput::Right));
        keyboard.handle(&g, FocusInput::Nav(NavInput::Right));
        mouse.handle(&g, FocusInput::PointerMoved(Vec2 { x: 160.0, y: 10.0 }));
        assert_eq!(
            keyboard.current(),
            mouse.current(),
            "both are on the second control"
        );
        let click = Vec2 { x: 160.0, y: 10.0 };
        let by_keyboard = keyboard.handle(&g, FocusInput::Nav(NavInput::Activate));
        mouse.handle(&g, FocusInput::PointerPressed(click));
        let by_mouse = mouse.handle(&g, FocusInput::PointerReleased(click));
        assert_eq!(by_keyboard, by_mouse);
        assert_eq!(by_keyboard, FocusEvent::Activate(id(1)));
    }

    #[test]
    fn a_control_that_disappears_or_is_disabled_takes_the_focus_with_it_without_a_crash() {
        let mut g = grid(3, 1);
        let mut f = Focus::default();
        f.set(&g, id(1));
        g.set_enabled(id(1), false).unwrap();
        assert_eq!(
            f.handle(&g, FocusInput::Nav(NavInput::Activate)),
            FocusEvent::None,
            "a disabled control cannot be activated"
        );
        assert_eq!(
            f.handle(&g, FocusInput::Nav(NavInput::Right)),
            FocusEvent::Moved {
                from: None,
                to: id(0)
            },
            "navigation starts again from the first control"
        );
        // The screen was rebuilt without the focused control.
        let mut f = Focus::default();
        f.set(&g, id(2));
        g.clear();
        g.add(FocusNode::new(id(7), rect(0.0, 0.0))).unwrap();
        assert_eq!(f.refocus(&g), Some(id(7)));
        assert_eq!(f.refocus(&g), None, "already fine");
        g.clear();
        assert_eq!(f.refocus(&g), None);
        assert_eq!(f.current(), None);
    }

    #[test]
    fn a_press_on_a_control_that_is_then_disabled_does_not_activate_it() {
        let mut g = grid(2, 1);
        let mut f = Focus::default();
        let p = Vec2 { x: 10.0, y: 10.0 };
        f.handle(&g, FocusInput::PointerPressed(p));
        g.set_enabled(id(0), false).unwrap();
        assert_eq!(
            f.handle(&g, FocusInput::PointerReleased(p)),
            FocusEvent::None
        );
    }

    #[test]
    fn set_refuses_a_missing_or_disabled_control() {
        let mut g = grid(2, 1);
        let mut f = Focus::default();
        assert!(!f.set(&g, id(9)));
        g.set_enabled(id(1), false).unwrap();
        assert!(!f.set(&g, id(1)));
        assert_eq!(f.current(), None);
        assert!(f.set(&g, id(0)));
    }
}
