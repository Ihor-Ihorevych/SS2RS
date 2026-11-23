//! The bookkeeping of a **GPU resource manager** (roadmap T04-003, `docs/GPU_RESOURCES.md`): a **staging ring**, a per-frame **upload budget** with a priority queue
//! for streamed assets, and **handles whose destruction waits for the GPU**.
//!
//! Device-free: these are the decisions and the arithmetic, not the buffers. The wgpu side (T04-003b) creates a real staging buffer of
//! [`StagingRing::capacity`] bytes and copies to the offsets the ring hands out; this module guarantees that those ranges never overlap while the GPU may still read them.
//!
//! * [`StagingRing`]: a ring allocator over one big upload buffer. Allocations belong to a **frame**; when the GPU reports a frame finished, [`retire`](StagingRing::retire)
//!   frees everything up to it.
//! * [`UploadScheduler`]: streamed assets queue their uploads; each frame [`plan`](UploadScheduler::plan) takes as many as fit in a **byte budget**, **highest priority
//!   first, oldest first**, and never skips ahead of a request that does not fit (no starvation of big uploads).
//! * [`ResourceTable`]: generational [`ResourceHandle`]s. [`remove`](ResourceTable::remove) hides a resource at once and hands it back from
//!   [`collect`](ResourceTable::collect) only after the frame that last used it has finished, so a buffer is never freed under a pass that is still reading it.

use std::collections::VecDeque;

/// Why a request was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceError {
    /// A size of zero.
    Empty,
    /// Bigger than the ring (or than the largest single upload allowed).
    TooLarge,
    /// An upload with this id is already waiting.
    Duplicate,
    /// A bad configuration value.
    BadConfig(&'static str),
}

impl std::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("a size of zero"),
            Self::TooLarge => f.write_str("larger than the staging ring or the upload limit"),
            Self::Duplicate => f.write_str("an upload with this id is already waiting"),
            Self::BadConfig(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for ResourceError {}

/// A piece of the staging buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StagingRange {
    /// Where it starts, a multiple of the ring's alignment.
    pub offset: u64,
    /// How many bytes it covers (the size that was asked for).
    pub size: u64,
}

#[derive(Clone, Copy, Debug)]
struct Allocation {
    frame: u64,
    start: u64,
    end: u64,
}

/// A ring allocator for the staging buffer.
///
/// ```
/// use ss2_render_api::gpu_resources::StagingRing;
///
/// let mut ring = StagingRing::new(1024, 16).unwrap();
/// let a = ring.alloc(600, 1).unwrap();
/// assert_eq!(a.offset, 0);
/// assert!(ring.alloc(600, 1).is_none(), "full until the GPU is done with frame 1");
/// ring.retire(1);
/// assert!(ring.alloc(600, 2).is_some());
/// ```
#[derive(Clone, Debug)]
pub struct StagingRing {
    capacity: u64,
    align: u64,
    live: VecDeque<Allocation>,
}

impl StagingRing {
    /// A ring of `capacity` bytes whose allocations start at multiples of `align` (a power of two, at most the capacity).
    pub fn new(capacity: u64, align: u64) -> Result<Self, ResourceError> {
        if capacity == 0 {
            return Err(ResourceError::BadConfig("the ring needs a capacity"));
        }
        if !align.is_power_of_two() || align > capacity {
            return Err(ResourceError::BadConfig(
                "the alignment must be a power of two no larger than the capacity",
            ));
        }
        Ok(Self {
            capacity,
            align,
            live: VecDeque::new(),
        })
    }

    /// The size of the staging buffer.
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    fn align_up(&self, x: u64) -> u64 {
        x.div_ceil(self.align) * self.align
    }

    /// Bytes inside live allocations, as asked for (not counting the padding between them or what a wrap leaves unused).
    pub fn used(&self) -> u64 {
        self.live.iter().map(|a| a.end - a.start).sum()
    }

    /// How many allocations are live.
    pub fn live_allocations(&self) -> usize {
        self.live.len()
    }

    /// The places the next allocation could go, as `(start, limit)`: it fits in a place when `start + size <= limit`. The first is the space after the
    /// newest allocation, the second the space at the start of the buffer if it has to wrap (empty when there is none).
    fn spaces(&self) -> [(u64, u64); 2] {
        let Some(newest) = self.live.back() else {
            return [(0, self.capacity), (0, 0)];
        };
        let tail = self.live.front().map_or(0, |a| a.start);
        let head = self.align_up(newest.end);
        if head > tail {
            // Not wrapped: room after the newest allocation to the end, and room before the oldest if we wrap.
            [(head, self.capacity), (0, tail)]
        } else {
            // Wrapped: only the gap between the newest allocation and the oldest.
            [(head, tail), (0, 0)]
        }
    }

    /// The biggest allocation that would succeed now.
    pub fn largest_free(&self) -> u64 {
        self.spaces()
            .iter()
            .map(|(start, limit)| limit.saturating_sub(*start))
            .max()
            .unwrap_or(0)
    }

    /// Take `size` bytes for work recorded in `frame`. `None` if the ring has no room (try again after [`retire`](Self::retire)) or the size is zero or
    /// larger than the whole ring. Frames must not go backwards between calls.
    pub fn alloc(&mut self, size: u64, frame: u64) -> Option<StagingRange> {
        if size == 0 {
            return None; // (a size over the capacity cannot fit either place and is refused below)
        }
        debug_assert!(
            self.live.back().is_none_or(|a| a.frame <= frame),
            "frames went backwards"
        );
        let [first, second] = self.spaces();
        let start = if first.0.saturating_add(size) <= first.1 {
            first.0
        } else if second.0 + size <= second.1 {
            second.0
        } else {
            return None;
        };
        self.live.push_back(Allocation {
            frame,
            start,
            end: start + size,
        });
        Some(StagingRange {
            offset: start,
            size,
        })
    }

    /// The GPU has finished every frame up to and including `completed`: free what they used.
    pub fn retire(&mut self, completed: u64) {
        while self.live.front().is_some_and(|a| a.frame <= completed) {
            self.live.pop_front();
        }
    }

    /// Forget everything (after a device loss).
    pub fn reset(&mut self) {
        self.live.clear();
    }
}

/// One upload that is waiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UploadRequest {
    /// Identifies the asset (the caller's id).
    pub id: u64,
    /// How many bytes to copy.
    pub bytes: u64,
    /// Higher goes first (the texture under the player's feet before a far prop).
    pub priority: u8,
    /// The frame it was submitted in; older goes first among equal priorities.
    pub submitted_frame: u64,
}

/// A queue of uploads and a per-frame budget.
///
/// ```
/// use ss2_render_api::gpu_resources::{UploadRequest, UploadScheduler};
///
/// let mut q = UploadScheduler::new(1 << 20).unwrap();
/// q.submit(UploadRequest { id: 1, bytes: 600_000, priority: 1, submitted_frame: 0 }).unwrap();
/// q.submit(UploadRequest { id: 2, bytes: 600_000, priority: 9, submitted_frame: 0 }).unwrap();
/// let first = q.plan(700_000);
/// assert_eq!(first.iter().map(|r| r.id).collect::<Vec<_>>(), [2]); // the urgent one; the other does not fit this frame
/// assert_eq!(q.plan(700_000).iter().map(|r| r.id).collect::<Vec<_>>(), [1]);
/// ```
#[derive(Clone, Debug)]
pub struct UploadScheduler {
    pending: Vec<UploadRequest>,
    max_request: u64,
}

impl UploadScheduler {
    /// A queue that refuses single uploads over `max_request` bytes (it should not exceed the staging ring).
    pub fn new(max_request: u64) -> Result<Self, ResourceError> {
        if max_request == 0 {
            return Err(ResourceError::BadConfig(
                "the largest upload must be positive",
            ));
        }
        Ok(Self {
            pending: Vec::new(),
            max_request,
        })
    }

    /// Queue an upload.
    pub fn submit(&mut self, req: UploadRequest) -> Result<(), ResourceError> {
        if req.bytes == 0 {
            return Err(ResourceError::Empty);
        }
        if req.bytes > self.max_request {
            return Err(ResourceError::TooLarge);
        }
        if self.pending.iter().any(|p| p.id == req.id) {
            return Err(ResourceError::Duplicate);
        }
        self.pending.push(req);
        Ok(())
    }

    /// Drop a waiting upload (the asset was unloaded before its turn). `true` if it was waiting.
    pub fn cancel(&mut self, id: u64) -> bool {
        let before = self.pending.len();
        self.pending.retain(|p| p.id != id);
        self.pending.len() != before
    }

    /// Change the priority of a waiting upload (the player turned around). `true` if it was waiting.
    pub fn reprioritize(&mut self, id: u64, priority: u8) -> bool {
        match self.pending.iter_mut().find(|p| p.id == id) {
            Some(p) => {
                p.priority = priority;
                true
            }
            None => false,
        }
    }

    /// How many uploads are waiting.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Whether nothing is waiting.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// The bytes waiting.
    pub fn pending_bytes(&self) -> u64 {
        self.pending.iter().map(|p| p.bytes).sum()
    }

    /// Choose this frame's uploads: highest priority first, then oldest, then lowest id, **stopping at the first request that does not fit in what is left of
    /// `budget_bytes`**. A request is never skipped to let a smaller, less urgent one through, so big uploads cannot starve. The one exception is the first
    /// request of the frame: it is taken even if it alone exceeds the budget (it cannot be split), so every upload is served eventually. The chosen
    /// requests leave the queue.
    pub fn plan(&mut self, budget_bytes: u64) -> Vec<UploadRequest> {
        self.pending.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then(a.submitted_frame.cmp(&b.submitted_frame))
                .then(a.id.cmp(&b.id))
        });
        let mut spent = 0u64;
        let mut taken = 0usize;
        for req in &self.pending {
            // `checked_add`: a saturated sum would compare equal to a budget of u64::MAX and let an oversize pair through.
            if taken > 0
                && spent
                    .checked_add(req.bytes)
                    .is_none_or(|total| total > budget_bytes)
            {
                break;
            }
            spent = spent.saturating_add(req.bytes);
            taken += 1;
        }
        self.pending.drain(..taken).collect()
    }
}

/// A handle to a resource: an index and a generation, so a stale handle never reaches a different resource that reused the slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResourceHandle {
    index: u32,
    generation: u32,
}

impl ResourceHandle {
    /// The slot index (for debugging).
    pub fn index(self) -> u32 {
        self.index
    }
}

#[derive(Debug)]
enum Slot<T> {
    Live(T),
    /// Removed; waiting for the GPU to finish with it.
    Pending,
    Free,
}

/// A table of resources with deferred destruction.
///
/// ```
/// use ss2_render_api::gpu_resources::ResourceTable;
///
/// let mut t: ResourceTable<&str> = ResourceTable::new();
/// let h = t.insert("mesh buffer");
/// t.remove(h, 7);                       // last used by frame 7
/// assert!(t.get(h).is_none());          // gone for new work at once...
/// assert!(t.collect(6).is_empty());     // ...but frame 7 may still be reading it
/// assert_eq!(t.collect(7), ["mesh buffer"]); // now it can really be freed
/// ```
#[derive(Debug)]
pub struct ResourceTable<T> {
    slots: Vec<(u32, Slot<T>)>,
    free: Vec<u32>,
    graveyard: VecDeque<(u64, u32, T)>,
}

impl<T> Default for ResourceTable<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ResourceTable<T> {
    /// An empty table.
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            graveyard: VecDeque::new(),
        }
    }

    /// Store a resource.
    pub fn insert(&mut self, value: T) -> ResourceHandle {
        if let Some(index) = self.free.pop() {
            let (generation, slot) = &mut self.slots[index as usize];
            *slot = Slot::Live(value);
            return ResourceHandle {
                index,
                generation: *generation,
            };
        }
        self.slots.push((0, Slot::Live(value)));
        ResourceHandle {
            index: (self.slots.len() - 1) as u32,
            generation: 0,
        }
    }

    /// The resource, or `None` for a handle that was removed (or never existed).
    pub fn get(&self, h: ResourceHandle) -> Option<&T> {
        match self.slots.get(h.index as usize) {
            Some((generation, Slot::Live(v))) if *generation == h.generation => Some(v),
            _ => None,
        }
    }

    /// The resource, mutably.
    pub fn get_mut(&mut self, h: ResourceHandle) -> Option<&mut T> {
        match self.slots.get_mut(h.index as usize) {
            Some((generation, Slot::Live(v))) if *generation == h.generation => Some(v),
            _ => None,
        }
    }

    /// Hide a resource from new work now and destroy it once frame `last_used_frame` has finished. Returns `false` for a handle that is not live.
    pub fn remove(&mut self, h: ResourceHandle, last_used_frame: u64) -> bool {
        let Some((generation, slot)) = self.slots.get_mut(h.index as usize) else {
            return false;
        };
        if *generation != h.generation || !matches!(slot, Slot::Live(_)) {
            return false;
        }
        let Slot::Live(value) = std::mem::replace(slot, Slot::Pending) else {
            return false;
        };
        self.graveyard.push_back((last_used_frame, h.index, value));
        true
    }

    /// The GPU has finished every frame up to and including `completed`: return the resources that can now really be freed (in the order they were removed), and
    /// let their slots be reused with a new generation.
    pub fn collect(&mut self, completed: u64) -> Vec<T> {
        let mut out = Vec::new();
        let mut keep = VecDeque::new();
        while let Some((frame, index, value)) = self.graveyard.pop_front() {
            if frame <= completed {
                let (generation, slot) = &mut self.slots[index as usize];
                *generation = generation.wrapping_add(1);
                *slot = Slot::Free;
                self.free.push(index);
                out.push(value);
            } else {
                keep.push_back((frame, index, value));
            }
        }
        self.graveyard = keep;
        out
    }

    /// How many resources are live.
    pub fn len(&self) -> usize {
        self.slots
            .iter()
            .filter(|(_, s)| matches!(s, Slot::Live(_)))
            .count()
    }

    /// Whether no resource is live.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// How many are removed but not yet collected.
    pub fn pending_destroy(&self) -> usize {
        self.graveyard.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- the ring ---------------------------------------------------------------------------------

    fn ring(cap: u64, align: u64) -> StagingRing {
        StagingRing::new(cap, align).unwrap()
    }

    #[test]
    fn allocations_follow_each_other_aligned_and_never_overlap() {
        let mut r = ring(1024, 16);
        let a = r.alloc(10, 1).unwrap();
        let b = r.alloc(33, 1).unwrap();
        let c = r.alloc(16, 1).unwrap();
        assert_eq!(
            (a.offset, b.offset, c.offset),
            (0, 16, 64),
            "10 rounds to 16, 33 to 48"
        );
        assert_eq!(
            (a.size, b.size, c.size),
            (10, 33, 16),
            "the caller's size is returned, not the rounded one"
        );
        assert_eq!(
            r.used(),
            10 + 33 + 16,
            "the bytes asked for, without the padding"
        );
        assert_eq!(r.live_allocations(), 3);
    }

    #[test]
    fn a_full_ring_refuses_until_the_gpu_finishes_a_frame() {
        let mut r = ring(1024, 16);
        assert_eq!(
            r.alloc(1024, 1),
            Some(StagingRange {
                offset: 0,
                size: 1024
            }),
            "the whole ring is allowed"
        );
        assert!(r.alloc(1, 1).is_none());
        assert_eq!(r.largest_free(), 0);
        r.retire(0);
        assert!(r.alloc(1, 2).is_none(), "frame 0 was not frame 1");
        r.retire(1);
        assert_eq!(r.largest_free(), 1024);
        assert!(r.alloc(1, 2).is_some());
    }

    #[test]
    fn the_ring_wraps_into_the_space_a_retired_frame_left_at_the_start() {
        let mut r = ring(1000, 8);
        let a = r.alloc(400, 1).unwrap(); // 0..400
        let b = r.alloc(400, 2).unwrap(); // 400..800
        assert_eq!((a.offset, b.offset), (0, 400));
        assert!(
            r.alloc(300, 3).is_none(),
            "200 bytes at the end, nothing at the start yet"
        );
        r.retire(1);
        let c = r.alloc(300, 3).unwrap();
        assert_eq!(c.offset, 0, "wrapped to the start, which frame 1 freed");
        // Now wrapped: head at 304, tail at 400: 96 bytes between.
        assert_eq!(r.largest_free(), 96);
        assert!(r.alloc(97, 3).is_none());
        assert_eq!(r.alloc(96, 3).unwrap().offset, 304);
        r.retire(2);
        // Frame 2's block (400..800) is free: the gap after the wrapped data is now larger.
        assert!(r.alloc(300, 4).is_some());
    }

    #[test]
    fn a_wrapped_allocation_may_fill_the_space_before_the_oldest_exactly() {
        let mut r = ring(1000, 8);
        r.alloc(400, 1).unwrap(); // 0..400
        r.alloc(400, 2).unwrap(); // 400..800
        r.retire(1); // 0..400 is free, 800..1000 is too small
        assert_eq!(
            r.alloc(400, 3),
            Some(StagingRange {
                offset: 0,
                size: 400
            }),
            "exactly the 400 before the oldest"
        );
        assert_eq!(r.largest_free(), 0, "head meets tail: full");
        assert!(r.alloc(1, 3).is_none());
    }

    #[test]
    fn zero_oversize_and_bad_configurations_are_refused() {
        let mut r = ring(100, 4);
        assert!(r.alloc(0, 1).is_none());
        assert!(r.alloc(101, 1).is_none());
        assert!(r.alloc(100, 1).is_some());
        for (cap, align) in [(0, 1), (100, 0), (100, 3), (100, 128), (8, 16)] {
            assert!(
                matches!(
                    StagingRing::new(cap, align),
                    Err(ResourceError::BadConfig(_))
                ),
                "{cap} {align}"
            );
        }
        assert!(StagingRing::new(64, 64).is_ok());
        assert_eq!(r.capacity(), 100);
        r.reset();
        assert_eq!(r.live_allocations(), 0);
        assert_eq!(r.largest_free(), 100);
    }

    #[test]
    fn a_random_workload_never_hands_out_overlapping_or_out_of_range_memory() {
        let mut seed = 0x1234_5678_9ABC_DEF1u64;
        let mut rnd = move |m: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % m
        };
        for _ in 0..30 {
            let cap = 512 + rnd(4096);
            let align = 1u64 << rnd(5);
            let mut r = ring(cap, align);
            let mut live: Vec<(u64, u64, u64)> = Vec::new(); // (frame, start, end rounded)
            let mut frame = 0u64;
            let mut gpu_done = 0u64;
            for _ in 0..400 {
                match rnd(10) {
                    0 | 1 => frame += 1,
                    2 | 3 => {
                        gpu_done = (gpu_done + 1).min(frame);
                        r.retire(gpu_done);
                        live.retain(|(f, _, _)| *f > gpu_done);
                    }
                    _ => {
                        let size = 1 + rnd(cap / 3);
                        if let Some(range) = r.alloc(size, frame) {
                            let end = range.offset + size;
                            assert_eq!(range.offset % align, 0);
                            assert!(end <= cap, "range {range:?} leaves a ring of {cap}");
                            for (_, s, e) in &live {
                                assert!(
                                    end <= *s || range.offset >= *e,
                                    "{range:?} overlaps a live allocation {s}..{e}"
                                );
                            }
                            live.push((frame, range.offset, end));
                        }
                    }
                }
            }
            // Everything retired: the whole ring is available again.
            r.retire(u64::MAX);
            assert_eq!(r.largest_free(), cap);
            assert!(r.alloc(cap, frame + 1).is_some());
        }
    }

    // ---- the upload queue ------------------------------------------------------------------------------

    fn req(id: u64, bytes: u64, priority: u8, frame: u64) -> UploadRequest {
        UploadRequest {
            id,
            bytes,
            priority,
            submitted_frame: frame,
        }
    }

    fn ids(v: &[UploadRequest]) -> Vec<u64> {
        v.iter().map(|r| r.id).collect()
    }

    #[test]
    fn uploads_go_by_priority_then_age_then_id() {
        let mut q = UploadScheduler::new(1000).unwrap();
        for r in [
            req(5, 10, 1, 3),
            req(2, 10, 9, 5),
            req(3, 10, 1, 1),
            req(4, 10, 1, 1),
            req(1, 10, 9, 2),
        ] {
            q.submit(r).unwrap();
        }
        assert_eq!(ids(&q.plan(1000)), [1, 2, 3, 4, 5]);
        assert!(q.is_empty());
    }

    #[test]
    fn the_budget_limits_a_frame_and_the_rest_waits_in_order() {
        let mut q = UploadScheduler::new(1000).unwrap();
        for i in 1..=6 {
            q.submit(req(i, 100, 0, i)).unwrap();
        }
        assert_eq!(q.pending_bytes(), 600);
        assert_eq!(ids(&q.plan(250)), [1, 2], "two fit in 250");
        assert_eq!((q.len(), q.pending_bytes()), (4, 400));
        assert_eq!(ids(&q.plan(300)), [3, 4, 5], "exactly 300 fits three");
        assert_eq!(
            ids(&q.plan(0)),
            [6],
            "the first request of a frame is served even with no budget"
        );
        assert!(q.plan(1000).is_empty());
    }

    #[test]
    fn a_request_that_does_not_fit_blocks_the_ones_behind_it_so_big_uploads_cannot_starve() {
        let mut q = UploadScheduler::new(10_000).unwrap();
        q.submit(req(1, 100, 5, 0)).unwrap();
        q.submit(req(2, 900, 4, 0)).unwrap(); // does not fit after the first in a 500 budget
        q.submit(req(3, 50, 1, 0)).unwrap(); // would fit, but is less urgent than 2
        assert_eq!(ids(&q.plan(500)), [1]);
        // Next frame the big one is first, so it is taken even though it alone exceeds the budget.
        assert_eq!(ids(&q.plan(500)), [2]);
        assert_eq!(ids(&q.plan(500)), [3]);
    }

    #[test]
    fn submit_refuses_empty_oversize_and_duplicate_uploads() {
        let mut q = UploadScheduler::new(1000).unwrap();
        assert_eq!(q.submit(req(1, 0, 0, 0)), Err(ResourceError::Empty));
        assert_eq!(q.submit(req(1, 1001, 0, 0)), Err(ResourceError::TooLarge));
        assert!(
            q.submit(req(1, 1000, 0, 0)).is_ok(),
            "exactly the limit is fine"
        );
        assert_eq!(q.submit(req(1, 5, 0, 0)), Err(ResourceError::Duplicate));
        assert!(matches!(
            UploadScheduler::new(0),
            Err(ResourceError::BadConfig(_))
        ));
        // A planned (served) id may be submitted again.
        q.plan(10_000);
        assert!(q.submit(req(1, 5, 0, 1)).is_ok());
        for e in [
            ResourceError::Empty,
            ResourceError::TooLarge,
            ResourceError::Duplicate,
            ResourceError::BadConfig("x"),
        ] {
            assert!(!e.to_string().is_empty());
        }
    }

    #[test]
    fn equal_priority_and_age_go_by_id_whatever_order_they_were_submitted_in() {
        let mut q = UploadScheduler::new(1000).unwrap();
        for id in [9, 4, 7, 3] {
            q.submit(req(id, 10, 2, 5)).unwrap();
        }
        assert_eq!(ids(&q.plan(1000)), [3, 4, 7, 9]);
    }

    #[test]
    fn cancel_and_reprioritize_change_what_goes_next() {
        let mut q = UploadScheduler::new(1000).unwrap();
        for i in 1..=3 {
            q.submit(req(i, 10, 1, 0)).unwrap();
        }
        assert!(q.cancel(1));
        assert!(!q.cancel(1) && !q.cancel(99));
        assert!(q.reprioritize(3, 200));
        assert!(!q.reprioritize(99, 1));
        assert_eq!(ids(&q.plan(1000)), [3, 2]);
    }

    #[test]
    fn a_huge_request_size_does_not_overflow_the_budget_arithmetic() {
        let mut q = UploadScheduler::new(u64::MAX).unwrap();
        q.submit(req(1, u64::MAX, 1, 0)).unwrap();
        q.submit(req(2, u64::MAX, 1, 1)).unwrap();
        assert_eq!(
            ids(&q.plan(u64::MAX)),
            [1],
            "the sum saturates and the second waits"
        );
        assert_eq!(ids(&q.plan(u64::MAX)), [2]);
    }

    #[test]
    fn the_ring_and_the_queue_together_serve_every_upload_without_overlap() {
        // Frames pass; the GPU finishes a frame two frames late; each frame the queue is planned within the ring's free space.
        let mut ring = ring(10_000, 16);
        let mut q = UploadScheduler::new(10_000).unwrap();
        let mut seed = 77u64;
        let mut rnd = move |m: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % m
        };
        let mut next_id = 0;
        let mut live: Vec<(u64, u64, u64)> = Vec::new();
        let mut served = 0;
        for frame in 1..=300u64 {
            for _ in 0..rnd(3) {
                next_id += 1;
                q.submit(req(next_id, 1 + rnd(3000), rnd(4) as u8, frame))
                    .unwrap();
            }
            if frame > 2 {
                ring.retire(frame - 2);
                live.retain(|(f, _, _)| *f > frame - 2);
            }
            // Take uploads while the ring has room: plan a budget, then give back what does not fit.
            let budget = ring.largest_free().min(4000);
            for r in q.plan(budget) {
                match ring.alloc(r.bytes, frame) {
                    Some(range) => {
                        let end = range.offset + r.bytes;
                        assert!(live.iter().all(|(_, s, e)| end <= *s || range.offset >= *e));
                        live.push((frame, range.offset, end));
                        served += 1;
                    }
                    None => q
                        .submit(UploadRequest {
                            submitted_frame: r.submitted_frame,
                            ..r
                        })
                        .unwrap(),
                }
            }
        }
        // Drain.
        for frame in 301..=600u64 {
            ring.retire(frame - 2);
            let budget = ring.largest_free();
            for r in q.plan(budget) {
                if ring.alloc(r.bytes, frame).is_some() {
                    served += 1;
                } else {
                    q.submit(r).unwrap();
                }
            }
        }
        assert!(q.is_empty(), "{} uploads never got through", q.len());
        assert_eq!(served, next_id, "every request was served exactly once");
    }

    // ---- handles ---------------------------------------------------------------------------------------------

    #[test]
    fn a_removed_handle_is_dead_at_once_and_a_reused_slot_does_not_revive_it() {
        let mut t: ResourceTable<u32> = ResourceTable::new();
        let a = t.insert(10);
        let b = t.insert(20);
        assert_eq!((t.get(a), t.get(b), t.len()), (Some(&10), Some(&20), 2));
        *t.get_mut(a).unwrap() += 1;
        assert_eq!(t.get(a), Some(&11));
        assert!(t.remove(a, 5));
        assert_eq!(t.get(a), None);
        assert_eq!(t.get_mut(a), None);
        assert_eq!((t.len(), t.pending_destroy()), (1, 1));
        assert!(!t.remove(a, 5), "already removed");
        // The slot is not reused until the frame is done...
        let c = t.insert(30);
        assert_ne!(c.index(), a.index());
        assert_eq!(t.collect(4), Vec::<u32>::new());
        assert_eq!(t.collect(5), [11]);
        // ...then it is, with a new generation: the old handle stays dead.
        let d = t.insert(40);
        assert_eq!(d.index(), a.index());
        assert_ne!(d, a);
        assert_eq!((t.get(a), t.get(d)), (None, Some(&40)));
        assert!(
            !t.remove(a, 9),
            "a stale handle cannot remove the new tenant"
        );
        assert_eq!(t.get(d), Some(&40));
    }

    #[test]
    fn collect_returns_resources_in_removal_order_and_only_when_their_frame_is_done() {
        let mut t: ResourceTable<&str> = ResourceTable::new();
        let h: Vec<ResourceHandle> = ["a", "b", "c", "d"]
            .into_iter()
            .map(|s| t.insert(s))
            .collect();
        t.remove(h[0], 3);
        t.remove(h[1], 5);
        t.remove(h[2], 3);
        assert_eq!(t.collect(2), Vec::<&str>::new());
        assert_eq!(t.collect(3), ["a", "c"]);
        assert_eq!(t.pending_destroy(), 1);
        assert_eq!(t.collect(100), ["b"]);
        assert_eq!((t.len(), t.pending_destroy()), (1, 0));
        assert!(t.get(h[3]).is_some());
        assert!(!t.is_empty());
    }

    #[test]
    fn handles_from_nowhere_are_dead() {
        let mut t: ResourceTable<u8> = ResourceTable::default();
        let mut other: ResourceTable<u8> = ResourceTable::new();
        let foreign = (0..5).map(|i| other.insert(i)).last().unwrap();
        assert!(t.get(foreign).is_none() && t.get_mut(foreign).is_none() && !t.remove(foreign, 0));
        assert!(t.is_empty());
    }

    #[test]
    fn generations_wrap_without_panicking() {
        let mut t: ResourceTable<u8> = ResourceTable::new();
        t.insert(0);
        t.slots[0].0 = u32::MAX;
        let h = ResourceHandle {
            index: 0,
            generation: u32::MAX,
        };
        assert!(t.remove(h, 0));
        assert_eq!(t.collect(0), [0]);
        let again = t.insert(1);
        assert_eq!(again.generation, 0, "wrapped");
        assert!(t.get(h).is_none());
    }
}
