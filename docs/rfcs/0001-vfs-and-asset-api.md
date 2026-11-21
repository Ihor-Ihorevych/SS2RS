# RFC 0001: `Vfs`, `AssetLoader` and `AssetManager` API (contract v0)

- **Number:** 0001 (`docs/rfcs/0001-vfs-and-asset-api.md`)
- **Authors:** @lxstyo
- **Status:** in review
- **Approvers:** owner of `ss2-vfs` / `ss2-assets` **plus one** other maintainer
- **Review SLA:** 3 working days after "in review"
- **Roadmap tasks:** T00-014 (this contract); unblocks T02-001..005 (`ss2-vfs`), T02-009/010 (`ss2-assets`) and every consumer that loads game data (T04, T06, T07, T09, T10)

## Summary

Freeze the public API between "where bytes come from" and "who needs assets": a read-only `Vfs` trait with a normalized `VfsPath`, a per-format `AssetLoader` trait, and an `AssetManager` that hands out typed, cloneable `Handle<T>`s. Ship a `v0` stub with a working in-memory `MemoryVfs` and a small synchronous `AssetManager`, so consumers code and test against the final signatures now while T02 builds the real mount stack and async manager behind them.

## Motivation

Roadmap rule 2 ("contracts before code"): renderer, audio, world and gameplay all load assets, but the archive reader (T02-001) and the CTSEMETA-based loaders (T02-006+) are weeks away. Without a frozen API each consumer would invent its own, or block. Contracts also keep the clean-room boundary simple: format crates implement `AssetLoader` from a spec, nothing else.

## Detailed design

### `ss2-vfs`

```rust
pub struct VfsPath(/* normalized */);          // "models/player.mdl"
pub trait Vfs: Send + Sync {
    fn stat(&self, path: &VfsPath) -> Result<Stat>;             // kind + size
    fn open(&self, path: &VfsPath) -> Result<Box<dyn ReadSeek>>; // streaming
    fn list(&self, dir: &VfsPath) -> Result<Vec<DirEntry>>;      // sorted, de-duplicated
    fn read(&self, path: &VfsPath) -> Result<Vec<u8>> { /* default via open */ }
    fn exists(&self, path: &VfsPath) -> bool { /* default via stat */ }
}
pub struct MemoryVfs;                           // mock / test double
```

- **`VfsPath` is canonical**: both `/` and `\` accepted, `.` and empty segments dropped, lower-cased (the game is case-insensitive), no leading/trailing `/`. `==`, `Ord`, `Hash` therefore mean "same file". It **rejects** `..`, `:` and control characters, so a path can never escape the VFS or smuggle a host path (drive letters, URLs).
- **Read-only.** Writing (saves, config) is `ss2-platform`'s job (T00-016), not the game-data VFS.
- **`&self` + `Send + Sync`**: many threads read concurrently; implementations do their own locking.
- **Errors** are `ss2_core::Error`: `NotFound` (missing, or wrong kind), `Io`, `Invalid` (corrupt archive). Never panic.
- **`open` returns `Box<dyn ReadSeek>`** (`Read + Seek + Send`). This is the seam T02-005 needs: memory-mapped, buffered, or (Phase 2) Android content-URI backends all fit behind it without touching loaders. `read` is a convenience; backends may override it.
- **`list` is sorted and merged across layers** so overrides (`Patch_*` over `All_PC_*`, mods) and directory merges (T02-002) are invisible to callers and deterministic.
- Mount/layer management is **not** part of the trait. Constructing and layering a concrete VFS is the app's job (T02-002/003); consumers only see `Arc<dyn Vfs>`.

### `ss2-assets`

```rust
pub trait AssetLoader: Send + Sync + 'static {
    type Asset: Send + Sync + 'static;
    fn extensions(&self) -> &[&str];
    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<Self::Asset>;
}
pub struct LoadContext<'a>;   // path(), vfs(), read(), load_dependency::<D>(path)
pub struct Handle<T>;         // Clone; path(), state(), get() -> Option<Arc<T>>, error(), dependencies(), same_asset()
pub enum LoadState { Loading, Ready, Failed }   // #[non_exhaustive]
pub struct AssetManager;      // new(Arc<dyn Vfs>), register_loader(L), load::<T>(&str) -> Result<Handle<T>>
```

- **Typed handles, type-erased registry.** The manager maps extension → loader; `load::<T>` checks that the loader produces `T`. A mismatch is a caller error (`Err`), caught on first use instead of a bad downcast later.
- **Two kinds of failure, kept apart.** *Caller mistakes* (bad path, no loader, wrong `T`) return `Err` from `load`. *Data problems* (missing file, corrupt contents) are stored in the handle (`LoadState::Failed`, `error()`), so one bad asset never aborts a level load and the failure is cached and reportable once.
- **Handles may be `Loading`.** v0 loads synchronously, so handles are `Ready` or `Failed` on return, **but consumers must not rely on that**: T02-009 makes loading async. `LoadState` is `#[non_exhaustive]` for the same reason.
- **Dependencies** (`ctx.load_dependency`) go through the same manager, so shared textures are loaded once. They are recorded per asset (`Handle::dependencies`), which is the hook for the `RFIL`-based dependency graph (T02-009). Cyclic dependencies are allowed and terminate.
- **Cache key is `(path, asset type)`**; v0 never evicts. Refcount eviction, reload and cooked-cache lookup are T02-009/011 and need no signature change (`Handle` already shares an `Arc`).
- **Loaders are stateless/synchronized and must not panic on bad input** (`docs/CODING_STANDARDS.md`); fuzzing (T02-030) enforces it per format.
- **Not sim-safe.** `ss2-assets` and `ss2-vfs` use maps for caching, not simulation state; sim crates (`ss2-sim`, `ss2-physics`, `ss2-net`) must not depend on them directly, only on already-loaded data handed to them.

### Usage

See the doctests in `crates/ss2-assets/src/lib.rs` (a toy text loader against `MemoryVfs`) and `crates/ss2-vfs/src/lib.rs`.

### The mock

`MemoryVfs` (full `Vfs`) plus the synchronous v0 `AssetManager` are the mocks: consumers' tests build a `MemoryVfs` from `(path, bytes)` pairs, register their own tiny loader (or the real one once it exists) and exercise real code paths with no game data and no files on disk.

## Alternatives considered

- **Async `Vfs` / `AssetLoader` traits.** Rejected for v0: archive reads are blocking I/O on worker threads in practice, `async fn` in traits complicates object safety, and the manager (not the VFS) owns scheduling. Keeping the leaf traits synchronous lets T02-009 add async at the manager level.
- **`Vfs::read_range(path, offset, len)` instead of `open`.** Rejected: `ReadSeek` covers it, supports parsers written against `Read + Seek`, and is what mmap/streaming backends naturally provide.
- **Untyped `AssetId`/`Handle` + `Any`.** Rejected: loses compile-time asset types at every call site; the type check at `load::<T>` is cheap.
- **Case-preserving paths with case-insensitive compare.** Rejected: two representations of one file invite bugs; the original data is case-insensitive, so canonical lower-case is simplest. (Original-case display names, if ever needed, can be kept by the archive index.)
- **A separate `AssetManager` trait.** Rejected for v0: its generic methods make it non-object-safe, and a trait with a single implementation adds nothing. Tests substitute the `Vfs` instead.

## Compatibility and migration

- New code only; `ss2-vfs` and `ss2-assets` gain a dependency on `ss2-core` (and `ss2-assets` on `ss2-vfs`, `tracing`). No existing consumer changes.
- After merge, these signatures are frozen: changes need a new RFC. Additions that keep existing code compiling (new `LoadState` variants, new default trait methods) are allowed with owner approval.
- Order of follow-up work: T02-001..005 implement `Vfs` for archives/loose dirs; T02-009 swaps the manager internals; format crates implement `AssetLoader`.

## Open questions

- **Reload / eviction semantics** for failed handles and for hot reload (T02-011b): decided by the `ss2-assets` owner in T02-009.
- **Handles that hold other handles** (model → textures) form `Arc` cycles when assets are cyclic; T02-009 may need weak dependency links. Decided by the `ss2-assets` owner.
- **Non-UTF-8 file names in archives** (code-page encoded names in some ZIPs): how `VfsPath` should treat them. Decided with T02-001 once real archives are surveyed.
