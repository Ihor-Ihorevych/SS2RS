//! The **frame graph**: declare a frame's render passes and the resources they read and write; get back the **order** to run them in, the passes that can be
//! **skipped**, and a plan for **sharing memory** between short-lived resources (roadmap T04-002, `docs/FRAME_GRAPH.md`).
//!
//! Device-free: it schedules and plans, it does not draw. The renderer keeps what each pass *does* (a closure or an enum) and asks the compiled graph in what order
//! to run them ([`CompiledGraph::run`]) and which physical slot each transient resource lives in ([`AliasPlan`]).
//!
//! * A pass belongs to a [`Stage`] (shadow, depth pre-pass, opaque, transparent, post, UI, overlay). The run order is **by stage, then by the order the passes were
//!   declared in**, so modules can register their passes in any order and the frame still runs in the same sequence on every run.
//! * Dependencies come from that order: a reader depends on the last pass that wrote the resource (read after write), a writer on the passes that read the old
//!   contents (write after read) and on the previous writer (write after write). **A cycle cannot be built**, because every dependency points to an earlier pass.
//! * A pass is **culled** unless something live needs it: it has a side effect, writes an imported *output* (the swapchain), or writes something a live pass reads.
//! * Transient resources get a **lifetime** (first and last pass that use them) and are packed into **slots**: resources of the same kind whose lifetimes do not
//!   overlap share one.
//!
//! ```
//! use ss2_render_api::frame_graph::{FrameGraphBuilder, PassDesc, ResourceKind, Stage, TextureFormat};
//!
//! let mut g = FrameGraphBuilder::new();
//! let screen = g.import("swapchain", true);
//! let hdr = g.create("hdr", ResourceKind::Texture { width: 1280, height: 720, format: TextureFormat::Rgba16Float }).unwrap();
//! // Declared out of order on purpose: the stages put them right.
//! g.add_pass(PassDesc::new("tonemap", Stage::Post).reads(&[hdr]).writes(&[screen])).unwrap();
//! g.add_pass(PassDesc::new("opaque", Stage::Opaque).writes(&[hdr])).unwrap();
//! let graph = g.compile().unwrap();
//! let order: Vec<&str> = graph.passes().iter().map(|p| p.name.as_str()).collect();
//! assert_eq!(order, ["opaque", "tonemap"]);
//! ```

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The part of the frame a pass belongs to, in the order they run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Stage {
    /// Shadow maps.
    Shadow,
    /// The depth pre-pass.
    DepthPrepass,
    /// Opaque and alpha-tested geometry.
    Opaque,
    /// Blended geometry and particles.
    Transparent,
    /// Post-processing (tone mapping, bloom, anti-aliasing).
    Post,
    /// The 2D interface.
    Ui,
    /// Debug drawing and overlays.
    Overlay,
}

impl Stage {
    /// The name shown in dumps.
    pub fn name(self) -> &'static str {
        match self {
            Self::Shadow => "shadow",
            Self::DepthPrepass => "depth_prepass",
            Self::Opaque => "opaque",
            Self::Transparent => "transparent",
            Self::Post => "post",
            Self::Ui => "ui",
            Self::Overlay => "overlay",
        }
    }
}

/// The format of a texture resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureFormat {
    /// 8 bits per channel, 4 channels.
    Rgba8,
    /// 16-bit float, 4 channels.
    Rgba16Float,
    /// 16-bit float, 2 channels.
    Rg16Float,
    /// 32-bit float depth.
    Depth32,
    /// One 8-bit channel.
    R8,
}

impl TextureFormat {
    /// Bytes per pixel.
    pub fn bytes_per_pixel(self) -> u64 {
        match self {
            Self::Rgba8 | Self::Rg16Float | Self::Depth32 => 4,
            Self::Rgba16Float => 8,
            Self::R8 => 1,
        }
    }
}

/// What a transient resource is. Two resources can share memory only if their kinds are **equal**.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceKind {
    /// A 2D texture.
    Texture {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
        /// Pixel format.
        format: TextureFormat,
    },
    /// A buffer.
    Buffer {
        /// Size in bytes.
        size: u64,
    },
}

impl ResourceKind {
    /// The memory it needs, in bytes (saturating at `u64::MAX`; [`FrameGraphBuilder::create`] refuses sizes that do not fit).
    pub fn size_bytes(&self) -> u64 {
        match *self {
            Self::Texture {
                width,
                height,
                format,
            } => u64::from(width)
                .saturating_mul(u64::from(height))
                .saturating_mul(format.bytes_per_pixel()),
            Self::Buffer { size } => size,
        }
    }
}

/// A resource in a graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(u32);

impl ResourceId {
    /// The index in creation order.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A pass in a graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PassId(u32);

impl PassId {
    /// The index in declaration order.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// The most passes in a graph.
pub const MAX_PASSES: usize = 4096;
/// The most resources in a graph.
pub const MAX_RESOURCES: usize = 16_384;

/// Why a graph was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphError {
    /// A texture with a zero width or height, a buffer of zero bytes, or a size that does not fit in 64 bits.
    BadResourceSize(String),
    /// A pass mentions a resource that this graph does not have.
    UnknownResource {
        /// The pass.
        pass: String,
    },
    /// A pass reads a transient resource before any pass (in run order) has written it.
    ReadBeforeWrite {
        /// The pass.
        pass: String,
        /// The resource.
        resource: String,
    },
    /// More passes or resources than the limits.
    TooMany,
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadResourceSize(n) => {
                write!(f, "resource {n:?} has no size or a size that is too large")
            }
            Self::UnknownResource { pass } => {
                write!(f, "pass {pass:?} uses a resource that is not in this graph")
            }
            Self::ReadBeforeWrite { pass, resource } => write!(
                f,
                "pass {pass:?} reads {resource:?} before anything writes it"
            ),
            Self::TooMany => write!(
                f,
                "more than {MAX_PASSES} passes or {MAX_RESOURCES} resources"
            ),
        }
    }
}

impl std::error::Error for GraphError {}

/// A pass to add.
#[derive(Clone, Debug)]
pub struct PassDesc {
    name: String,
    stage: Stage,
    reads: Vec<ResourceId>,
    writes: Vec<ResourceId>,
    side_effect: bool,
}

impl PassDesc {
    /// A pass with no reads or writes yet.
    pub fn new(name: &str, stage: Stage) -> Self {
        Self {
            name: name.to_owned(),
            stage,
            reads: Vec::new(),
            writes: Vec::new(),
            side_effect: false,
        }
    }

    /// What it reads (a resource that it also writes, such as a depth buffer, is listed in both).
    pub fn reads(mut self, r: &[ResourceId]) -> Self {
        self.reads = r.to_vec();
        self
    }

    /// What it writes.
    pub fn writes(mut self, w: &[ResourceId]) -> Self {
        self.writes = w.to_vec();
        self
    }

    /// The pass must run even if nothing reads what it writes (a readback, a present).
    pub fn side_effect(mut self) -> Self {
        self.side_effect = true;
        self
    }
}

#[derive(Clone, Debug)]
struct ResourceEntry {
    name: String,
    /// `Some` for a transient resource the graph allocates; `None` for an imported one.
    kind: Option<ResourceKind>,
    /// For an imported resource: writing it keeps a pass alive (the swapchain).
    output: bool,
}

/// Collects passes and resources.
#[derive(Clone, Debug, Default)]
pub struct FrameGraphBuilder {
    resources: Vec<ResourceEntry>,
    passes: Vec<PassDesc>,
}

impl FrameGraphBuilder {
    /// An empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// A resource the graph allocates for this frame. It must be written before it is read.
    pub fn create(&mut self, name: &str, kind: ResourceKind) -> Result<ResourceId, GraphError> {
        let fits = match kind {
            ResourceKind::Texture {
                width,
                height,
                format,
            } => {
                width != 0
                    && height != 0
                    && u64::from(width)
                        .checked_mul(u64::from(height))
                        .and_then(|px| px.checked_mul(format.bytes_per_pixel()))
                        .is_some()
            }
            ResourceKind::Buffer { size } => size != 0,
        };
        if !fits {
            return Err(GraphError::BadResourceSize(name.to_owned()));
        }
        self.push_resource(ResourceEntry {
            name: name.to_owned(),
            kind: Some(kind),
            output: false,
        })
    }

    /// A resource that exists outside the graph (the swapchain image, a texture from the last frame). Reading it needs no writer; with `output` set, writing it keeps
    /// the writing pass alive.
    ///
    /// # Panics
    /// If the resource limit is already reached (use [`create`](Self::create) to get an error instead).
    pub fn import(&mut self, name: &str, output: bool) -> ResourceId {
        self.push_resource(ResourceEntry {
            name: name.to_owned(),
            kind: None,
            output,
        })
        .expect("too many frame graph resources")
    }

    fn push_resource(&mut self, e: ResourceEntry) -> Result<ResourceId, GraphError> {
        if self.resources.len() >= MAX_RESOURCES {
            return Err(GraphError::TooMany);
        }
        self.resources.push(e);
        Ok(ResourceId((self.resources.len() - 1) as u32))
    }

    /// Declare a pass.
    pub fn add_pass(&mut self, pass: PassDesc) -> Result<PassId, GraphError> {
        if self.passes.len() >= MAX_PASSES {
            return Err(GraphError::TooMany);
        }
        if pass
            .reads
            .iter()
            .chain(&pass.writes)
            .any(|r| r.index() >= self.resources.len())
        {
            return Err(GraphError::UnknownResource { pass: pass.name });
        }
        self.passes.push(pass);
        Ok(PassId((self.passes.len() - 1) as u32))
    }

    /// Schedule: order the passes, cull the dead ones, work out lifetimes and memory sharing.
    pub fn compile(&self) -> Result<CompiledGraph, GraphError> {
        // Program order: by stage, then by declaration.
        let mut order: Vec<usize> = (0..self.passes.len()).collect();
        order.sort_by_key(|i| (self.passes[*i].stage, *i));

        // Dependencies in program order. `raw` (read after write) decides what is alive; the others only order.
        let n = self.passes.len();
        let mut last_writer: Vec<Option<usize>> = vec![None; self.resources.len()];
        let mut readers_since_write: Vec<Vec<usize>> = vec![Vec::new(); self.resources.len()];
        let mut raw: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut deps: Vec<Vec<usize>> = vec![Vec::new(); n];
        for &p in &order {
            let pass = &self.passes[p];
            for r in &pass.reads {
                match last_writer[r.index()] {
                    Some(w) => {
                        push_unique(&mut raw[p], w);
                        push_unique(&mut deps[p], w);
                    }
                    None if self.resources[r.index()].kind.is_some() => {
                        return Err(GraphError::ReadBeforeWrite {
                            pass: pass.name.clone(),
                            resource: self.resources[r.index()].name.clone(),
                        });
                    }
                    None => {}
                }
                readers_since_write[r.index()].push(p);
            }
            for w in &pass.writes {
                if let Some(prev) = last_writer[w.index()] {
                    push_unique(&mut deps[p], prev);
                }
                for &reader in &readers_since_write[w.index()] {
                    if reader != p {
                        push_unique(&mut deps[p], reader);
                    }
                }
                last_writer[w.index()] = Some(p);
                readers_since_write[w.index()].clear();
            }
        }

        // Liveness, backwards through the read-after-write edges.
        let mut live = vec![false; n];
        let mut stack: Vec<usize> = Vec::new();
        for (i, pass) in self.passes.iter().enumerate() {
            let writes_output = pass.writes.iter().any(|w| self.resources[w.index()].output);
            if pass.side_effect || writes_output {
                live[i] = true;
                stack.push(i);
            }
        }
        while let Some(p) = stack.pop() {
            for &w in &raw[p] {
                if !live[w] {
                    live[w] = true;
                    stack.push(w);
                }
            }
        }

        let kept: Vec<usize> = order.iter().copied().filter(|p| live[*p]).collect();
        let culled: Vec<PassId> = order
            .iter()
            .copied()
            .filter(|p| !live[*p])
            .map(|p| PassId(p as u32))
            .collect();
        let position: BTreeMap<usize, usize> =
            kept.iter().enumerate().map(|(pos, p)| (*p, pos)).collect();

        // Lifetimes of the transient resources the live passes touch.
        let mut lifetimes: BTreeMap<ResourceId, Lifetime> = BTreeMap::new();
        for (pos, &p) in kept.iter().enumerate() {
            for r in self.passes[p].reads.iter().chain(&self.passes[p].writes) {
                if self.resources[r.index()].kind.is_some() {
                    lifetimes
                        .entry(*r)
                        .and_modify(|l| l.last = pos)
                        .or_insert(Lifetime {
                            first: pos,
                            last: pos,
                        });
                }
            }
        }
        let unused: Vec<ResourceId> = (0..self.resources.len())
            .map(|i| ResourceId(i as u32))
            .filter(|r| self.resources[r.index()].kind.is_some() && !lifetimes.contains_key(r))
            .collect();

        let compiled: Vec<CompiledPass> = kept
            .iter()
            .map(|&p| {
                let d = &self.passes[p];
                let mut depends_on: Vec<PassId> = deps[p]
                    .iter()
                    .filter(|w| live[**w])
                    .map(|w| PassId(*w as u32))
                    .collect();
                depends_on.sort_by_key(|id| position[&id.index()]);
                CompiledPass {
                    id: PassId(p as u32),
                    name: d.name.clone(),
                    stage: d.stage,
                    reads: d.reads.clone(),
                    writes: d.writes.clone(),
                    depends_on,
                }
            })
            .collect();

        let kinds: BTreeMap<ResourceId, ResourceKind> = lifetimes
            .keys()
            .filter_map(|r| self.resources[r.index()].kind.map(|k| (*r, k)))
            .collect();
        let plan = plan_aliasing(&lifetimes, &kinds);
        let names = self.resources.iter().map(|r| r.name.clone()).collect();
        Ok(CompiledGraph {
            passes: compiled,
            culled,
            lifetimes,
            unused,
            plan,
            resource_names: names,
        })
    }
}

fn push_unique(v: &mut Vec<usize>, x: usize) {
    if !v.contains(&x) {
        v.push(x);
    }
}

/// A transient resource's life: the positions in the compiled order of the first and the last pass that use it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lifetime {
    /// Position of the first pass.
    pub first: usize,
    /// Position of the last pass.
    pub last: usize,
}

/// A pass in run order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledPass {
    /// Its id from [`FrameGraphBuilder::add_pass`].
    pub id: PassId,
    /// Its name.
    pub name: String,
    /// Its stage.
    pub stage: Stage,
    /// What it reads.
    pub reads: Vec<ResourceId>,
    /// What it writes.
    pub writes: Vec<ResourceId>,
    /// The live passes that must run before it, in run order.
    pub depends_on: Vec<PassId>,
}

/// One piece of memory that several resources take turns in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    /// What it holds (every tenant has this kind).
    pub kind: ResourceKind,
    /// The resources that use it, in the order they do.
    pub tenants: Vec<ResourceId>,
}

/// Where each transient resource lives.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AliasPlan {
    /// The slots to allocate.
    pub slots: Vec<Slot>,
    /// For each transient resource that is used: the index of its slot.
    pub slot_of: BTreeMap<ResourceId, usize>,
    /// Memory needed if every resource had its own.
    pub bytes_without_sharing: u64,
    /// Memory needed with the slots.
    pub bytes_with_sharing: u64,
}

fn plan_aliasing(
    lifetimes: &BTreeMap<ResourceId, Lifetime>,
    kinds: &BTreeMap<ResourceId, ResourceKind>,
) -> AliasPlan {
    // Interval colouring per kind: by first use, give each resource the lowest slot whose last tenant is finished (strictly before this one starts).
    let mut by_start: Vec<(&ResourceId, &Lifetime)> = lifetimes.iter().collect();
    by_start.sort_by_key(|(id, life)| (life.first, **id));
    let mut plan = AliasPlan::default();
    let mut slot_end: Vec<usize> = Vec::new(); // the last position at which each slot is in use
    for (id, life) in by_start {
        let kind = kinds[id];
        plan.bytes_without_sharing = plan.bytes_without_sharing.saturating_add(kind.size_bytes());
        let free = plan
            .slots
            .iter()
            .enumerate()
            .position(|(i, s)| s.kind == kind && slot_end[i] < life.first);
        let slot = match free {
            Some(i) => i,
            None => {
                plan.slots.push(Slot {
                    kind,
                    tenants: Vec::new(),
                });
                slot_end.push(0);
                plan.bytes_with_sharing = plan.bytes_with_sharing.saturating_add(kind.size_bytes());
                plan.slots.len() - 1
            }
        };
        plan.slots[slot].tenants.push(*id);
        slot_end[slot] = life.last;
        plan.slot_of.insert(*id, slot);
    }
    plan
}

/// A scheduled frame.
#[derive(Clone, Debug)]
pub struct CompiledGraph {
    passes: Vec<CompiledPass>,
    culled: Vec<PassId>,
    lifetimes: BTreeMap<ResourceId, Lifetime>,
    unused: Vec<ResourceId>,
    plan: AliasPlan,
    resource_names: Vec<String>,
}

impl CompiledGraph {
    /// The live passes in the order to run them.
    pub fn passes(&self) -> &[CompiledPass] {
        &self.passes
    }

    /// Call `f` for each live pass in run order.
    pub fn run(&self, mut f: impl FnMut(&CompiledPass)) {
        for p in &self.passes {
            f(p);
        }
    }

    /// The passes that were dropped because nothing live needs them, in program order.
    pub fn culled(&self) -> &[PassId] {
        &self.culled
    }

    /// The life of a transient resource, or `None` if no live pass uses it.
    pub fn lifetime(&self, r: ResourceId) -> Option<Lifetime> {
        self.lifetimes.get(&r).copied()
    }

    /// Transient resources no live pass uses (nothing to allocate).
    pub fn unused_resources(&self) -> &[ResourceId] {
        &self.unused
    }

    /// Where the transient resources live.
    pub fn alias_plan(&self) -> &AliasPlan {
        &self.plan
    }

    /// The name a resource was created with.
    pub fn resource_name(&self, r: ResourceId) -> Option<&str> {
        self.resource_names.get(r.index()).map(String::as_str)
    }

    /// A text dump for a console command or a log: one line per pass, then the memory plan.
    pub fn to_text(&self) -> String {
        let name = |r: &ResourceId| {
            self.resource_names
                .get(r.index())
                .map_or("?", String::as_str)
        };
        let mut s = String::new();
        for (i, p) in self.passes.iter().enumerate() {
            let reads: Vec<&str> = p.reads.iter().map(name).collect();
            let writes: Vec<&str> = p.writes.iter().map(name).collect();
            let _ = writeln!(
                s,
                "{i:>2} {:<13} {:<14} reads [{}] writes [{}]",
                p.stage.name(),
                p.name,
                reads.join(", "),
                writes.join(", ")
            );
        }
        let _ = writeln!(
            s,
            "culled {} pass(es); {} slot(s), {} bytes ({} without sharing)",
            self.culled.len(),
            self.plan.slots.len(),
            self.plan.bytes_with_sharing,
            self.plan.bytes_without_sharing
        );
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HD: ResourceKind = ResourceKind::Texture {
        width: 1280,
        height: 720,
        format: TextureFormat::Rgba16Float,
    };
    const DEPTH: ResourceKind = ResourceKind::Texture {
        width: 1280,
        height: 720,
        format: TextureFormat::Depth32,
    };

    fn names(g: &CompiledGraph) -> Vec<&str> {
        g.passes().iter().map(|p| p.name.as_str()).collect()
    }

    /// A realistic frame: shadows, depth, opaque, particles, post, UI, and a debug pass nobody asked for.
    fn frame(order_reversed: bool) -> (CompiledGraph, [ResourceId; 4]) {
        let mut g = FrameGraphBuilder::new();
        let screen = g.import("swapchain", true);
        let shadow = g
            .create(
                "shadow_map",
                ResourceKind::Texture {
                    width: 2048,
                    height: 2048,
                    format: TextureFormat::Depth32,
                },
            )
            .unwrap();
        let depth = g.create("depth", DEPTH).unwrap();
        let hdr = g.create("hdr", HD).unwrap();
        let bloom = g.create("bloom", HD).unwrap();
        let _unused = g
            .create(
                "ssao",
                ResourceKind::Texture {
                    width: 640,
                    height: 360,
                    format: TextureFormat::R8,
                },
            )
            .unwrap();
        let mut passes = vec![
            PassDesc::new("shadow", Stage::Shadow).writes(&[shadow]),
            PassDesc::new("prepass", Stage::DepthPrepass).writes(&[depth]),
            PassDesc::new("opaque", Stage::Opaque)
                .reads(&[shadow, depth])
                .writes(&[hdr, depth]),
            PassDesc::new("particles", Stage::Transparent)
                .reads(&[depth, hdr])
                .writes(&[hdr]),
            PassDesc::new("bloom", Stage::Post)
                .reads(&[hdr])
                .writes(&[bloom]),
            PassDesc::new("tonemap", Stage::Post)
                .reads(&[hdr, bloom])
                .writes(&[screen]),
            PassDesc::new("ui", Stage::Ui)
                .reads(&[screen])
                .writes(&[screen]),
            PassDesc::new("debug_heat", Stage::Overlay)
                .reads(&[depth])
                .writes(&[bloom]), // writes something nobody reads afterwards
        ];
        if order_reversed {
            // Declared from the last stage to the first; the sort is stable, so the passes of one stage keep their order (bloom before tonemap).
            passes.sort_by_key(|p| std::cmp::Reverse(p.stage));
        }
        for p in passes {
            g.add_pass(p).unwrap();
        }
        (g.compile().unwrap(), [shadow, depth, hdr, bloom])
    }

    // ---- ordering ---------------------------------------------------------------------------------

    #[test]
    fn passes_run_by_stage_whatever_order_they_were_declared_in() {
        let (a, _) = frame(false);
        assert_eq!(
            names(&a),
            [
                "shadow",
                "prepass",
                "opaque",
                "particles",
                "bloom",
                "tonemap",
                "ui"
            ]
        );
        // Declared from the last stage to the first, the stages still put the frame in the same order.
        let (b, _) = frame(true);
        assert_eq!(names(&b), names(&a));
        assert_eq!(b.to_text(), a.to_text());
    }

    #[test]
    fn within_a_stage_the_declaration_order_decides() {
        let mut g = FrameGraphBuilder::new();
        let screen = g.import("screen", true);
        for n in ["c", "a", "b"] {
            g.add_pass(PassDesc::new(n, Stage::Post).writes(&[screen]))
                .unwrap();
        }
        assert_eq!(names(&g.compile().unwrap()), ["c", "a", "b"]);
    }

    #[test]
    fn a_pass_that_reads_what_a_later_declared_pass_of_its_own_stage_writes_is_refused() {
        // Within a stage the declaration order is the order: declare producers first.
        let mut g = FrameGraphBuilder::new();
        let screen = g.import("screen", true);
        let t = g.create("t", HD).unwrap();
        g.add_pass(
            PassDesc::new("consumer", Stage::Post)
                .reads(&[t])
                .writes(&[screen]),
        )
        .unwrap();
        g.add_pass(PassDesc::new("producer", Stage::Post).writes(&[t]))
            .unwrap();
        assert!(matches!(
            g.compile(),
            Err(GraphError::ReadBeforeWrite { .. })
        ));
    }

    #[test]
    fn dependencies_are_read_after_write_write_after_read_and_write_after_write() {
        let mut g = FrameGraphBuilder::new();
        let screen = g.import("screen", true);
        let t = g.create("t", HD).unwrap();
        let w1 = g
            .add_pass(PassDesc::new("w1", Stage::Opaque).writes(&[t]))
            .unwrap();
        let r1 = g
            .add_pass(
                PassDesc::new("r1", Stage::Transparent)
                    .reads(&[t])
                    .writes(&[screen]),
            )
            .unwrap();
        let w2 = g
            .add_pass(PassDesc::new("w2", Stage::Post).writes(&[t]))
            .unwrap(); // overwrites after r1 read it
        let r2 = g
            .add_pass(PassDesc::new("r2", Stage::Ui).reads(&[t]).writes(&[screen]))
            .unwrap();
        let c = g.compile().unwrap();
        let dep = |id: PassId| {
            c.passes()
                .iter()
                .find(|p| p.id == id)
                .unwrap()
                .depends_on
                .clone()
        };
        assert_eq!(dep(w1), []);
        assert_eq!(dep(r1), [w1], "read after write");
        assert_eq!(dep(w2), [w1, r1], "write after write and write after read");
        assert_eq!(
            dep(r2),
            [r1, w2],
            "read after the write of t, and write after the write of the screen"
        );
    }

    // ---- errors -----------------------------------------------------------------------------------

    #[test]
    fn reading_a_transient_resource_before_it_is_written_is_refused() {
        let mut g = FrameGraphBuilder::new();
        let t = g.create("t", HD).unwrap();
        let screen = g.import("screen", true);
        g.add_pass(
            PassDesc::new("early", Stage::Opaque)
                .reads(&[t])
                .writes(&[screen]),
        )
        .unwrap();
        g.add_pass(PassDesc::new("late", Stage::Post).writes(&[t]))
            .unwrap(); // writes it, but in a later stage
        let err = g.compile().unwrap_err();
        assert_eq!(
            err,
            GraphError::ReadBeforeWrite {
                pass: "early".into(),
                resource: "t".into()
            }
        );
        assert!(err.to_string().contains("early") && err.to_string().contains("\"t\""));
    }

    #[test]
    fn an_imported_resource_can_be_read_without_a_writer() {
        let mut g = FrameGraphBuilder::new();
        let history = g.import("last_frame", false);
        let screen = g.import("screen", true);
        g.add_pass(
            PassDesc::new("taa", Stage::Post)
                .reads(&[history])
                .writes(&[screen]),
        )
        .unwrap();
        assert_eq!(names(&g.compile().unwrap()), ["taa"]);
    }

    #[test]
    fn empty_resources_and_foreign_ids_and_too_many_things_are_refused() {
        let mut g = FrameGraphBuilder::new();
        for bad in [
            ResourceKind::Texture {
                width: 0,
                height: 4,
                format: TextureFormat::R8,
            },
            ResourceKind::Texture {
                width: 4,
                height: 0,
                format: TextureFormat::R8,
            },
            ResourceKind::Buffer { size: 0 },
        ] {
            assert!(matches!(
                g.create("x", bad),
                Err(GraphError::BadResourceSize(_))
            ));
        }
        let huge = ResourceKind::Texture {
            width: u32::MAX,
            height: u32::MAX,
            format: TextureFormat::Rgba16Float,
        };
        assert!(
            matches!(g.create("huge", huge), Err(GraphError::BadResourceSize(_))),
            "a size that does not fit in 64 bits"
        );
        let ok_big = ResourceKind::Texture {
            width: 65_536,
            height: 65_536,
            format: TextureFormat::Rgba16Float,
        };
        assert!(g.create("big but fine", ok_big).is_ok());
        let mut g = FrameGraphBuilder::new();
        let mut other = FrameGraphBuilder::new();
        let foreign = other.create("a", HD).unwrap();
        let _ = other.create("b", HD).unwrap();
        let far = other.create("c", HD).unwrap();
        assert!(matches!(
            g.add_pass(PassDesc::new("p", Stage::Post).reads(&[far])),
            Err(GraphError::UnknownResource { .. })
        ));
        assert!(
            matches!(
                g.add_pass(PassDesc::new("p", Stage::Post).writes(&[foreign])),
                Err(GraphError::UnknownResource { .. })
            ),
            "the graph has no resources at all"
        );
        // Limits.
        let mut g = FrameGraphBuilder::new();
        for _ in 0..MAX_PASSES {
            g.add_pass(PassDesc::new("p", Stage::Post)).unwrap();
        }
        assert_eq!(
            g.add_pass(PassDesc::new("one more", Stage::Post))
                .unwrap_err(),
            GraphError::TooMany
        );
        let mut g = FrameGraphBuilder::new();
        for _ in 0..MAX_RESOURCES {
            g.create("r", HD).unwrap();
        }
        assert_eq!(g.create("one more", HD).unwrap_err(), GraphError::TooMany);
        assert!(GraphError::TooMany.to_string().contains("4096"));
    }

    // ---- culling ----------------------------------------------------------------------------------

    #[test]
    fn a_pass_nobody_needs_is_culled_and_so_is_what_only_it_needed() {
        let (g, [_, depth, hdr, bloom]) = frame(false);
        assert_eq!(
            g.culled().len(),
            1,
            "debug_heat writes bloom after tonemap and nothing reads it"
        );
        let _ = (depth, hdr, bloom);
        assert!(!names(&g).contains(&"debug_heat"));
        // The unused texture is not allocated.
        assert_eq!(g.unused_resources().len(), 1);
        assert_eq!(g.resource_name(g.unused_resources()[0]), Some("ssao"));
        assert_eq!(g.resource_name(ResourceId(999)), None);
    }

    #[test]
    fn a_chain_feeding_only_a_dead_pass_is_dead_too() {
        let mut g = FrameGraphBuilder::new();
        let screen = g.import("screen", true);
        let a = g.create("a", HD).unwrap();
        let b = g.create("b", HD).unwrap();
        g.add_pass(PassDesc::new("make_a", Stage::Opaque).writes(&[a]))
            .unwrap();
        g.add_pass(
            PassDesc::new("make_b", Stage::Transparent)
                .reads(&[a])
                .writes(&[b]),
        )
        .unwrap(); // nobody reads b
        g.add_pass(PassDesc::new("present", Stage::Post).writes(&[screen]))
            .unwrap();
        let c = g.compile().unwrap();
        assert_eq!(names(&c), ["present"]);
        assert_eq!(c.culled().len(), 2);
        assert_eq!(c.unused_resources().len(), 2);
        assert!(c.lifetime(a).is_none());
    }

    #[test]
    fn side_effect_passes_are_kept_and_keep_their_inputs_alive() {
        let mut g = FrameGraphBuilder::new();
        let t = g.create("t", HD).unwrap();
        g.add_pass(PassDesc::new("make", Stage::Opaque).writes(&[t]))
            .unwrap();
        g.add_pass(
            PassDesc::new("readback", Stage::Post)
                .reads(&[t])
                .side_effect(),
        )
        .unwrap();
        g.add_pass(PassDesc::new("pointless", Stage::Post).reads(&[t]))
            .unwrap();
        let c = g.compile().unwrap();
        assert_eq!(names(&c), ["make", "readback"]);
    }

    #[test]
    fn a_graph_with_no_output_runs_nothing() {
        let mut g = FrameGraphBuilder::new();
        let t = g.create("t", HD).unwrap();
        g.add_pass(PassDesc::new("make", Stage::Opaque).writes(&[t]))
            .unwrap();
        let c = g.compile().unwrap();
        assert!(c.passes().is_empty());
        assert_eq!(c.culled().len(), 1);
        assert_eq!(
            FrameGraphBuilder::new().compile().unwrap().passes().len(),
            0
        );
    }

    #[test]
    fn writing_an_imported_non_output_does_not_keep_a_pass_alive() {
        let mut g = FrameGraphBuilder::new();
        let history = g.import("history", false);
        g.add_pass(PassDesc::new("save", Stage::Post).writes(&[history]))
            .unwrap();
        assert!(g.compile().unwrap().passes().is_empty());
    }

    #[test]
    fn a_read_write_pass_keeps_the_earlier_writer_alive() {
        // depth is written by the prepass and then read and written by opaque: the prepass is needed through the read.
        let (g, [_, depth, ..]) = frame(false);
        assert!(names(&g).contains(&"prepass"));
        let l = g.lifetime(depth).unwrap();
        assert_eq!(l.first, 1, "written by the prepass, the second live pass");
    }

    // ---- lifetimes and sharing -----------------------------------------------------------------------

    #[test]
    fn lifetimes_span_the_first_to_the_last_live_pass_that_uses_a_resource() {
        let (g, [shadow, depth, hdr, bloom]) = frame(false);
        // Run order: 0 shadow, 1 prepass, 2 opaque, 3 particles, 4 bloom, 5 tonemap, 6 ui.
        assert_eq!(g.lifetime(shadow), Some(Lifetime { first: 0, last: 2 }));
        assert_eq!(g.lifetime(depth), Some(Lifetime { first: 1, last: 3 }));
        assert_eq!(g.lifetime(hdr), Some(Lifetime { first: 2, last: 5 }));
        assert_eq!(g.lifetime(bloom), Some(Lifetime { first: 4, last: 5 }));
    }

    #[test]
    fn resources_of_one_kind_with_disjoint_lives_share_a_slot_and_others_do_not() {
        let mut g = FrameGraphBuilder::new();
        let screen = g.import("screen", true);
        let a = g.create("a", HD).unwrap();
        let b = g.create("b", HD).unwrap();
        let c = g.create("c", HD).unwrap();
        let d = g.create("d", DEPTH).unwrap();
        // a lives over passes 0..=1, b over 2..=3 (after a), c over 1..=2 (overlaps both), d is a different kind over 0..=3.
        g.add_pass(PassDesc::new("p0", Stage::Shadow).writes(&[a, d]))
            .unwrap();
        g.add_pass(
            PassDesc::new("p1", Stage::DepthPrepass)
                .reads(&[a])
                .writes(&[c]),
        )
        .unwrap();
        g.add_pass(PassDesc::new("p2", Stage::Opaque).reads(&[c]).writes(&[b]))
            .unwrap();
        g.add_pass(
            PassDesc::new("p3", Stage::Post)
                .reads(&[b, d])
                .writes(&[screen]),
        )
        .unwrap();
        let g = g.compile().unwrap();
        let plan = g.alias_plan();
        assert_eq!(
            plan.slot_of[&a], plan.slot_of[&b],
            "a is finished before b starts"
        );
        assert_ne!(plan.slot_of[&c], plan.slot_of[&a]);
        assert_ne!(plan.slot_of[&c], plan.slot_of[&b]);
        assert_ne!(
            plan.slot_of[&d], plan.slot_of[&a],
            "a different kind never shares"
        );
        assert_eq!(plan.slots.len(), 3);
        let hd = HD.size_bytes();
        assert_eq!(plan.bytes_without_sharing, 3 * hd + DEPTH.size_bytes());
        assert_eq!(plan.bytes_with_sharing, 2 * hd + DEPTH.size_bytes());
        assert_eq!(plan.slots[plan.slot_of[&a]].tenants, [a, b]);
    }

    #[test]
    fn a_resource_whose_life_ends_where_another_begins_does_not_share_with_it() {
        // One pass reads x and writes y: both are in use during that pass, so they must not be the same memory.
        let mut g = FrameGraphBuilder::new();
        let screen = g.import("screen", true);
        let x = g.create("x", HD).unwrap();
        let y = g.create("y", HD).unwrap();
        g.add_pass(PassDesc::new("make_x", Stage::Opaque).writes(&[x]))
            .unwrap();
        g.add_pass(
            PassDesc::new("x_to_y", Stage::Transparent)
                .reads(&[x])
                .writes(&[y]),
        )
        .unwrap();
        g.add_pass(
            PassDesc::new("out", Stage::Post)
                .reads(&[y])
                .writes(&[screen]),
        )
        .unwrap();
        let g = g.compile().unwrap();
        assert_ne!(g.alias_plan().slot_of[&x], g.alias_plan().slot_of[&y]);
    }

    #[test]
    fn sharing_is_optimal_and_never_overlaps_on_random_graphs() {
        // A chain of passes, each writing a fresh resource that a later pass reads: lifetimes are arbitrary intervals. The number of slots per kind must equal the
        // largest number of resources of that kind alive at once, and no slot may hold two resources alive together.
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        let mut rnd = move |m: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % m
        };
        for _ in 0..40 {
            let passes = 4 + rnd(20) as usize;
            let mut g = FrameGraphBuilder::new();
            let screen = g.import("screen", true);
            let kinds = [HD, DEPTH, ResourceKind::Buffer { size: 4096 }];
            // The kind each resource was created with, kept here so the checks do not rely on the plan's own idea of it.
            let mut kind_of: BTreeMap<ResourceId, ResourceKind> = BTreeMap::new();
            let mut pending: Vec<ResourceId> = Vec::new();
            for p in 0..passes {
                // Read some earlier resources (this ends their lives)...
                let mut reads: Vec<ResourceId> =
                    pending.iter().copied().filter(|_| rnd(3) == 0).collect();
                pending.retain(|r| !reads.contains(r));
                // ...and make a new one of a random kind.
                let kind = kinds[rnd(3) as usize];
                let r = g.create(&format!("r{p}"), kind).unwrap();
                kind_of.insert(r, kind);
                let mut writes = vec![r];
                if p == passes - 1 {
                    // The last pass also reads everything left over and writes the output, so what feeds it is live.
                    reads.extend(pending.iter().copied());
                    writes.push(screen);
                }
                pending.push(r);
                g.add_pass(
                    PassDesc::new(&format!("p{p}"), Stage::Opaque)
                        .reads(&reads)
                        .writes(&writes),
                )
                .unwrap();
            }
            let c = g.compile().unwrap();
            let plan = c.alias_plan();
            // Whatever is made but never read (and is not in the last pass) is culled with its pass only if nothing needs it; compute from the compiled lifetimes.
            for slot in &plan.slots {
                let mut lives: Vec<Lifetime> = slot
                    .tenants
                    .iter()
                    .map(|t| c.lifetime(*t).unwrap())
                    .collect();
                lives.sort_by_key(|l| l.first);
                for w in lives.windows(2) {
                    assert!(
                        w[0].last < w[1].first,
                        "two tenants of one slot are alive together"
                    );
                }
                assert!(
                    slot.tenants.iter().all(|t| kind_of[t] == slot.kind),
                    "a slot holds only resources of its own kind"
                );
            }
            // Optimality: slots of a kind == the largest overlap of that kind.
            for kind in kinds {
                let lives: Vec<Lifetime> = plan
                    .slot_of
                    .keys()
                    .filter(|r| kind_of[*r] == kind)
                    .map(|r| c.lifetime(*r).unwrap())
                    .collect();
                let max_overlap = (0..c.passes().len())
                    .map(|pos| {
                        lives
                            .iter()
                            .filter(|l| l.first <= pos && pos <= l.last)
                            .count()
                    })
                    .max()
                    .unwrap_or(0);
                let slots = plan.slots.iter().filter(|s| s.kind == kind).count();
                assert_eq!(slots, max_overlap, "{kind:?}");
            }
        }
    }

    #[test]
    fn a_pass_that_reads_and_writes_one_resource_does_not_depend_on_itself() {
        let (g, _) = frame(false);
        for p in g.passes() {
            assert!(
                !p.depends_on.contains(&p.id),
                "{} depends on itself",
                p.name
            );
        }
        let opaque = g.passes().iter().find(|p| p.name == "opaque").unwrap();
        assert!(!opaque.depends_on.is_empty());
    }

    #[test]
    fn a_later_writer_depends_on_the_latest_readers_only_not_on_ones_before_an_earlier_write() {
        let mut g = FrameGraphBuilder::new();
        let screen = g.import("screen", true);
        let t = g.create("t", HD).unwrap();
        let w1 = g
            .add_pass(PassDesc::new("w1", Stage::Shadow).writes(&[t]))
            .unwrap();
        let r1 = g
            .add_pass(
                PassDesc::new("r1", Stage::DepthPrepass)
                    .reads(&[t])
                    .writes(&[screen]),
            )
            .unwrap();
        let w2 = g
            .add_pass(PassDesc::new("w2", Stage::Opaque).writes(&[t]))
            .unwrap();
        let r2 = g
            .add_pass(
                PassDesc::new("r2", Stage::Transparent)
                    .reads(&[t])
                    .writes(&[screen]),
            )
            .unwrap();
        let w3 = g
            .add_pass(PassDesc::new("w3", Stage::Post).writes(&[t]))
            .unwrap();
        g.add_pass(
            PassDesc::new("out", Stage::Ui)
                .reads(&[t])
                .writes(&[screen]),
        )
        .unwrap();
        let c = g.compile().unwrap();
        let deps = |id: PassId| {
            c.passes()
                .iter()
                .find(|p| p.id == id)
                .unwrap()
                .depends_on
                .clone()
        };
        assert_eq!(
            deps(w2),
            [w1, r1],
            "overwrites w1 and must wait for r1 to finish reading it"
        );
        // w3 follows w2 (write after write) and r2 (the reader of w2's contents); r1 read the older contents and is ordered through w2.
        assert_eq!(deps(w3), [w2, r2]);
    }

    #[test]
    fn dependencies_never_name_a_pass_that_was_culled() {
        let mut g = FrameGraphBuilder::new();
        let screen = g.import("screen", true);
        let t = g.create("t", HD).unwrap();
        let dead = g
            .add_pass(PassDesc::new("dead_writer", Stage::Opaque).writes(&[t]))
            .unwrap();
        let live = g
            .add_pass(PassDesc::new("live_writer", Stage::Post).writes(&[t, screen]))
            .unwrap();
        let c = g.compile().unwrap();
        assert_eq!(c.culled(), [dead]);
        assert!(
            c.passes()[0].depends_on.is_empty(),
            "the overwrite of a dead write is not a dependency"
        );
        assert_eq!(c.passes()[0].id, live);
    }

    #[test]
    fn different_kinds_never_share_even_when_their_lives_are_apart() {
        let mut g = FrameGraphBuilder::new();
        let screen = g.import("screen", true);
        let a = g.create("a", HD).unwrap();
        let e = g.create("e", ResourceKind::Buffer { size: 4096 }).unwrap();
        let smaller = g
            .create(
                "smaller",
                ResourceKind::Texture {
                    width: 640,
                    height: 360,
                    format: TextureFormat::Rgba16Float,
                },
            )
            .unwrap();
        g.add_pass(PassDesc::new("p0", Stage::Shadow).writes(&[a]))
            .unwrap();
        g.add_pass(
            PassDesc::new("p1", Stage::DepthPrepass)
                .reads(&[a])
                .writes(&[e]),
        )
        .unwrap();
        g.add_pass(
            PassDesc::new("p2", Stage::Opaque)
                .reads(&[e])
                .writes(&[smaller]),
        )
        .unwrap();
        g.add_pass(
            PassDesc::new("p3", Stage::Post)
                .reads(&[smaller])
                .writes(&[screen]),
        )
        .unwrap();
        let g = g.compile().unwrap();
        let plan = g.alias_plan();
        assert_eq!(plan.slots.len(), 3, "a, e and smaller are three kinds");
        assert_eq!(plan.bytes_with_sharing, plan.bytes_without_sharing);
    }

    // ---- running and printing ----------------------------------------------------------------------------

    #[test]
    fn run_visits_the_live_passes_in_order() {
        let (g, _) = frame(false);
        let mut seen = Vec::new();
        g.run(|p| seen.push(p.name.clone()));
        assert_eq!(seen, names(&g));
    }

    #[test]
    fn the_text_dump_lists_passes_and_the_memory_plan() {
        let mut g = FrameGraphBuilder::new();
        let screen = g.import("screen", true);
        let t = g
            .create(
                "hdr",
                ResourceKind::Texture {
                    width: 2,
                    height: 2,
                    format: TextureFormat::Rgba8,
                },
            )
            .unwrap();
        g.add_pass(PassDesc::new("scene", Stage::Opaque).writes(&[t]))
            .unwrap();
        g.add_pass(
            PassDesc::new("tonemap", Stage::Post)
                .reads(&[t])
                .writes(&[screen]),
        )
        .unwrap();
        g.add_pass(PassDesc::new("junk", Stage::Ui)).unwrap();
        let text = g.compile().unwrap().to_text();
        assert_eq!(
            text,
            " 0 opaque        scene          reads [] writes [hdr]\n 1 post          tonemap        reads [hdr] writes [screen]\nculled 1 pass(es); 1 slot(s), 16 bytes (16 without sharing)\n"
        );
    }

    #[test]
    fn sizes_follow_the_format() {
        assert_eq!(HD.size_bytes(), 1280 * 720 * 8);
        assert_eq!(DEPTH.size_bytes(), 1280 * 720 * 4);
        assert_eq!(
            ResourceKind::Texture {
                width: 10,
                height: 10,
                format: TextureFormat::R8
            }
            .size_bytes(),
            100
        );
        assert_eq!(
            ResourceKind::Texture {
                width: 10,
                height: 10,
                format: TextureFormat::Rg16Float
            }
            .size_bytes(),
            400
        );
        assert_eq!(ResourceKind::Buffer { size: 77 }.size_bytes(), 77);
        assert_eq!(
            ResourceKind::Texture {
                width: u32::MAX,
                height: u32::MAX,
                format: TextureFormat::Rgba16Float
            }
            .size_bytes(),
            u64::MAX,
            "saturates"
        );
    }
}
