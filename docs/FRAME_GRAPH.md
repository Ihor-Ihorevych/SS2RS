# The frame graph

`ss2_render_api::frame_graph` (T04-002) is the **scheduler** of a frame: declare the passes and the resources they read and write, compile, and get the **order to run them in**, the passes that can be **skipped**, and a plan for **sharing memory** between short-lived resources. It is device-free. It decides; it does not draw. The renderer keeps what each pass *does* and asks the compiled graph when and with what memory.

```rust,ignore
let mut g = FrameGraphBuilder::new();
let screen = g.import("swapchain", true);                     // outside the graph; writing it keeps a pass alive
let hdr = g.create("hdr", ResourceKind::Texture { width: 1280, height: 720, format: TextureFormat::Rgba16Float })?;
g.add_pass(PassDesc::new("tonemap", Stage::Post).reads(&[hdr]).writes(&[screen]))?;
g.add_pass(PassDesc::new("opaque", Stage::Opaque).writes(&[hdr]))?;   // declared second, runs first
let graph = g.compile()?;
graph.run(|pass| { /* record the pass */ });
```

## Order

A pass has a `Stage`: `Shadow`, `DepthPrepass`, `Opaque`, `Transparent`, `Post`, `Ui`, `Overlay` (the enum is `non_exhaustive`, so a stage can be inserted later). The run order is **by stage, then by declaration order**. Modules may therefore register their passes in any order and the frame runs in the same sequence on every run (tested: a realistic frame of seven live passes declared last stage first compiles to an identical dump).

**Inside one stage the declaration order is the order**: a pass that reads what a later-declared pass of the same stage writes is refused (`ReadBeforeWrite`). I chose that over reordering inside a stage because a resource has no versions here: a later writer could mean "produce it first" or "overwrite it afterwards", and guessing would hide mistakes.

**Dependencies** come from that order: a reader depends on the last pass that wrote the resource (read after write), a writer on the passes that read the old contents (write after read) and on the previous writer (write after write). Every dependency points to an earlier pass, so **a cycle cannot be built**. A pass that both reads and writes a resource (a depth buffer under test) lists it in both; it does not depend on itself. `CompiledPass::depends_on` lists the live passes it must follow.

## Culling

A pass runs only if something live needs it. Live: a pass marked `side_effect()` (a readback), a pass that writes an imported resource marked `output` (the swapchain), and, backwards through **read-after-write** edges only, the passes that write what a live pass reads. A chain feeding only a dead pass is dead (tested), a graph with no output runs nothing, and writing an imported resource that is not an output does not keep a pass alive. Write-after-write and write-after-read edges only order; they do not make a pass live. `culled()` lists the dropped passes and `unused_resources()` the transient resources nothing live uses (nothing is allocated for them).

## Memory sharing

Each transient resource used by a live pass has a **lifetime**: the positions in the compiled order of the first and last pass that touch it. `alias_plan()` packs them into **slots**: resources of the **same kind** (same texture size and format, or the same buffer size) whose lifetimes do not overlap share one slot. Lifetimes that merely touch (one pass reads `x` and writes `y`) **do not** share, because both are in use during that pass. The algorithm is greedy by first use with the lowest free slot, which for intervals gives the fewest slots possible; a test on 40 random graphs checks that the number of slots of each kind **equals the largest number of resources of that kind alive at once** and that no slot ever holds two resources alive together. `bytes_with_sharing` against `bytes_without_sharing` says what it saved (sizes saturate rather than overflow, and `create` refuses a texture whose size does not fit in 64 bits).

Resources of different kinds never share, even though a real GPU could alias a large texture and a smaller one. That would need the allocator's alignment rules; it is a follow-up.

## Debugging

`CompiledGraph::to_text()` gives one line per pass and a summary of the memory plan, for a console command or a log:

```
 0 opaque        scene          reads [] writes [hdr]
 1 post          tonemap        reads [hdr] writes [screen]
culled 1 pass(es); 1 slot(s), 16 bytes (16 without sharing)
```

## How it was tested

Twenty-four unit tests and a doctest: stage order against declaration order, ties by declaration, the three kinds of dependency checked on a small graph and the exact edges of a write after two readers, errors (read before write, empty and overflowing sizes, foreign resource ids, the 4,096-pass and 16,384-resource limits), culling in six situations, lifetimes on a realistic frame, sharing in four situations and the random optimality test, the dump compared as an exact string. **Twenty-eight deliberate breakages: twenty-four failed a test at once and four survived my first tests**, all four real gaps: a pass depending on itself, a writer depending on readers from before an earlier write, a dependency naming a culled pass, and a slot accepting a resource of another kind. The last one was a **bug in my own test**, not only a gap: the helper that recovered each resource's kind did so from the plan, so the check could never fail. I made the test keep its own record of the kinds. All four are now killed.

## Not covered (follow-up T04-002b)

- **Nothing executes it on a GPU**: no wgpu encoder, no creation of the textures and buffers from the `AliasPlan`, no pass implementations (the renderer does not exist yet, T04-026). The graph says what; it has never driven a real pass.
- **No resource reuse across frames** (a pool keyed by kind so the plan's slots persist), no handling of a window resize (every transient is rebuilt from the sizes given at `create`), and no multisampled or mip-mapped or array resources: `ResourceKind` has only a 2D texture and a buffer.
- **No sharing between different kinds**, no per-pass timestamp hook (T04-019b will attach one per `CompiledPass`) and no multi-threaded recording.
- **Barriers**: wgpu inserts them, so the graph does not compute any; if the renderer ever moves below wgpu, `depends_on` is the information to start from.
