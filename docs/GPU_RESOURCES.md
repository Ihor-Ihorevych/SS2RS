# GPU resource bookkeeping

`ss2_render_api::gpu_resources` (T04-003) is the **bookkeeping** of a GPU resource manager: where in the staging buffer an upload goes, how many bytes of uploads a frame may spend and in what order, and when a resource may really be destroyed. It is device-free: it never touches a GPU. The wgpu side (follow-up T04-003b) creates the staging buffer, records the copies and destroys the buffers; this module guarantees that what it does is safe.

## The staging ring

`StagingRing::new(capacity, align)` models one big upload buffer used as a ring. `alloc(size, frame)` returns a `StagingRange { offset, size }` (the offset is a multiple of the alignment) or `None`; `retire(completed_frame)` frees everything the GPU has finished. So the CPU can write frame N's uploads while the GPU still reads frame N-1's, and **no live range ever overlaps another** (checked by a random-workload test over 30 rings of random size and alignment, 400 operations each).

- Allocations are in order; when there is no room after the newest one, the ring **wraps** to the start if the oldest allocation has left enough room. An allocation that fits exactly in the space before the oldest is allowed.
- The buffer may be filled completely: an allocation needs its own size to fit, not its size rounded up to the alignment. (My first version rounded and so refused a full-size allocation of a buffer whose capacity was not a multiple of the alignment; the random test found it.)
- `largest_free()` says what would fit now; `used()` and `live_allocations()` are for statistics; `reset()` is for a lost device.
- A full ring returns `None`: **the caller waits a frame and tries again**. The ring never blocks and never overwrites.

## The upload queue

`UploadScheduler` holds the uploads of streamed assets and chooses what to spend each frame's byte budget on. `plan(budget_bytes)` returns requests **by priority (highest first), then oldest, then lowest id**, and **stops at the first one that does not fit in what is left**. It never skips a big request to let a smaller, less urgent one through, so a big upload cannot be starved by a stream of small ones. The first request of a frame is taken even if it alone exceeds the budget, because an upload cannot be split here, so every request is eventually served. `submit` refuses a size of zero, a size over the configured maximum (which should not exceed the ring), and a second request with an id that is already waiting; `cancel` and `reprioritize` handle an asset that was unloaded or became urgent.

The budget arithmetic uses `checked_add`: a first version used a saturating sum, which for two requests of `u64::MAX` bytes compared equal to a budget of `u64::MAX` and let both through. A test of that case found it.

The ring and the queue compose: each frame the caller plans with `min(budget, ring.largest_free())`, allocates a ring range for each chosen request and puts back any that did not fit. A test runs 300 frames of random submissions with the GPU finishing a frame two frames late, checks no overlap, and then drains the queue: **every request is served exactly once**.

## Handles with deferred destruction

`ResourceTable<T>` hands out `ResourceHandle`s (an index and a generation). `remove(handle, last_used_frame)` makes the resource **invisible to new work at once** (`get` returns `None`) but keeps it in a graveyard; `collect(completed_frame)` returns the resources whose last frame the GPU has finished, in removal order, so the caller can really destroy them, and only then are their slots reused, **with a new generation**: a stale handle can never reach the slot's next tenant, and cannot remove it either (tested, including generation wrap-around).

## How it was tested

Eighteen unit tests and three doctests. **Twenty-six deliberate breakages (one of them a variant): twenty-one failed a test at once and four survived my first tests.** Two of those were real gaps (a wrapped allocation filling the space before the oldest exactly, and the id tie-break, which a stable sort had been hiding behind the insertion order) and now fail a test. Two were equivalent: a redundant "larger than the ring" check in `alloc` (the fit check already refuses it), which I removed, and the zero-capacity check, which the alignment check also catches; I kept it for its clearer message.

## Not covered (follow-up T04-003b)

- **No wgpu**: nothing creates the staging buffer, records `copy_buffer_to_texture`, calls `queue.write_buffer`, or reads a frame-completion signal from the device. Which frame the GPU has finished (`completed`) has to come from a fence or from `Queue::on_submitted_work_done`; that wiring does not exist.
- **No texture or buffer creation, pooling or budgets by memory type**, no memory-pressure handling on a phone, and no uploads split across frames (a request is atomic, so the ring must be at least as big as the largest asset: a big texture needs a bigger ring or a mip-by-mip split).
- **No asset streaming API on top** (requests keyed by asset path, load states, priority from camera distance). The queue is the mechanism; the policy waits for the asset system (T02-*, T05-*).
- **The budget is a number the caller chooses.** What a good per-frame budget is on a given GPU, and how to adapt it from frame times (`frame_timing`), is unmeasured.
