//! ss2-assets — asset manager, loader registry and cooked cache (track T02).
//!
//! The contract of roadmap T00-014 (`docs/rfcs/0001-vfs-and-asset-api.md`) with the internals of
//! T02-009. Three things make up the API that renderer, audio, world and gameplay code are written
//! against:
//!
//! - [`AssetLoader`]: implemented once per file format (`.tex`, `.mdl`, ...) by the format crates.
//! - [`AssetManager`]: owns a [`Vfs`], a registry of loaders and a cache; hands out [`Handle`]s.
//! - [`Handle`]: a cheap, cloneable, typed reference to an asset that may still be loading.
//!
//! What the manager does, chosen with [`AssetOptions`]:
//!
//! - **Loading** is synchronous on the calling thread by default ([`AssetManager::new`]); with
//!   `workers > 0` it runs on a pool of threads and [`AssetManager::load`] returns at once with a
//!   handle in [`LoadState::Loading`] ([`Handle::wait`] blocks until it is done). Code must treat
//!   a handle as possibly loading in both modes.
//! - **Reference counting**: the cache holds an asset strongly (never evicts, the default) or
//!   weakly (`evict_unreferenced`): when the last [`Handle`] to an asset is dropped, the asset is
//!   freed and the next request loads it again. An asset keeps the assets it depends on alive.
//! - **Dependencies**: [`LoadContext::load_dependency`] (typed) and [`LoadContext::load_any`] (by
//!   path, for references listed in a file, for example the `RFIL` table of a CTSEMETA file) are
//!   recorded per asset ([`Handle::dependencies`], [`AssetManager::dependency_graph`]) and loaded
//!   through the same cache, so shared textures load once. [`Handle::wait_all`] waits for the
//!   whole dependency closure.
//! - **Errors**: a failed load never aborts the caller. It is stored in the handle
//!   ([`Handle::error`]), logged, and listed by [`AssetManager::failures`]; a loader that panics
//!   is reported as a failure too.
//!
//! ```
//! use std::sync::Arc;
//! use ss2_assets::{AssetLoader, AssetManager, LoadContext, LoadState};
//! use ss2_core::Result;
//! use ss2_vfs::MemoryVfs;
//!
//! /// A toy format: the whole file is UTF-8 text.
//! struct TextLoader;
//! impl AssetLoader for TextLoader {
//!     type Asset = String;
//!     fn extensions(&self) -> &[&str] { &["txt"] }
//!     fn load(&self, ctx: &mut LoadContext<'_>) -> Result<String> {
//!         String::from_utf8(ctx.read()?)
//!             .map_err(|e| ss2_core::Error::invalid("text asset", e.to_string()))
//!     }
//! }
//!
//! let vfs = MemoryVfs::from_files([("docs/hello.txt", &b"hi there"[..])]).unwrap();
//! let assets = AssetManager::new(Arc::new(vfs));
//! assets.register_loader(TextLoader).unwrap();
//!
//! let handle = assets.load::<String>("Docs/Hello.TXT").unwrap();
//! assert_eq!(handle.wait(), LoadState::Ready);
//! assert_eq!(handle.get().as_deref().map(String::as_str), Some("hi there"));
//! ```

use std::any::{Any, TypeId};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};
use std::thread::JoinHandle;

mod registry;

use registry::{ErasedLoader, LoaderRegistry, erase, normalize_extensions};
use ss2_core::{Error, Result};
use ss2_vfs::{Vfs, VfsPath};

/// Most failures [`AssetManager::failures`] remembers (the oldest are dropped).
const MAX_FAILURES: usize = 10_000;

/// Knows how to turn the bytes of one file format into an asset value.
///
/// Implementations must be stateless or internally synchronized: the manager may call
/// [`load`](Self::load) from several threads. They must **never panic on bad input**: return
/// [`Error::Invalid`] (`ss2_core::Error::invalid(..)`) instead.
pub trait AssetLoader: Send + Sync + 'static {
    /// The in-memory type this loader produces.
    type Asset: Send + Sync + 'static;

    /// File extensions this loader handles, for example `&["tex"]`. Registration lowercases
    /// entries and strips leading dots. Each extension maps to one loader per manager.
    fn extensions(&self) -> &[&str];

    /// Parse the asset at [`LoadContext::path`].
    ///
    /// # Errors
    /// Any [`Error`]; it is stored in the asset's [`Handle`] and reported, not propagated.
    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<Self::Asset>;
}

/// How an [`AssetManager`] loads and caches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssetOptions {
    /// Worker threads. `0` (the default) loads on the calling thread, inside
    /// [`AssetManager::load`].
    pub workers: usize,
    /// Free an asset when its last [`Handle`] is dropped (the next request loads it again). The
    /// default keeps every asset until the manager is dropped.
    pub evict_unreferenced: bool,
}

impl AssetOptions {
    /// The settings of a running game: a worker pool sized to the machine (one thread fewer than
    /// the cores, at least one, at most eight) and eviction of unreferenced assets.
    pub fn for_game() -> Self {
        let cores = std::thread::available_parallelism().map_or(2, usize::from);
        Self {
            workers: cores.saturating_sub(1).clamp(1, 8),
            evict_unreferenced: true,
        }
    }
}

/// A snapshot of the manager's counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssetStats {
    /// Assets that were requested for the first time (cache misses).
    pub requested: u64,
    /// Loads that finished successfully.
    pub loaded: u64,
    /// Loads that failed.
    pub failed: u64,
    /// Loaded assets that were freed because no handle referred to them any more.
    pub evicted: u64,
    /// Entries in the cache now (loading, ready or failed, and still alive).
    pub cached: usize,
}

#[derive(Default)]
struct Counters {
    requested: AtomicU64,
    loaded: AtomicU64,
    failed: AtomicU64,
    evicted: AtomicU64,
}

/// A dependency of a loaded asset: keeps the dependency alive as long as the asset is.
#[derive(Clone)]
struct Dependency {
    path: VfsPath,
    slot: Arc<Slot>,
}

enum SlotState {
    Loading,
    Ready {
        asset: Arc<dyn Any + Send + Sync>,
        dependencies: Vec<Dependency>,
    },
    Failed(String),
}

struct Slot {
    state: Mutex<SlotState>,
    done: Condvar,
    counters: Arc<Counters>,
}

impl Slot {
    fn new(counters: Arc<Counters>) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(SlotState::Loading),
            done: Condvar::new(),
            counters,
        })
    }

    fn lock(&self) -> MutexGuard<'_, SlotState> {
        // A panic in another thread cannot leave the state half-written, so poison is harmless.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn state(&self) -> LoadState {
        match &*self.lock() {
            SlotState::Loading => LoadState::Loading,
            SlotState::Ready { .. } => LoadState::Ready,
            SlotState::Failed(_) => LoadState::Failed,
        }
    }

    fn wait(&self) -> LoadState {
        let mut guard = self.lock();
        while matches!(*guard, SlotState::Loading) {
            guard = self
                .done
                .wait(guard)
                .unwrap_or_else(PoisonError::into_inner);
        }
        match &*guard {
            SlotState::Ready { .. } => LoadState::Ready,
            _ => LoadState::Failed,
        }
    }

    fn dependencies(&self) -> Vec<Dependency> {
        match &*self.lock() {
            SlotState::Ready { dependencies, .. } => dependencies.clone(),
            _ => Vec::new(),
        }
    }

    /// Wait for this asset and, recursively, for everything it depends on.
    fn wait_all(self: &Arc<Self>) -> LoadState {
        let mut seen = BTreeSet::new();
        let mut stack = vec![Arc::clone(self)];
        let mut worst = LoadState::Ready;
        while let Some(slot) = stack.pop() {
            if !seen.insert(Arc::as_ptr(&slot) as usize) {
                continue;
            }
            if slot.wait() == LoadState::Failed {
                worst = LoadState::Failed;
            }
            stack.extend(slot.dependencies().into_iter().map(|d| d.slot));
        }
        worst
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        let state = self.state.get_mut().unwrap_or_else(PoisonError::into_inner);
        if matches!(state, SlotState::Ready { .. }) {
            self.counters.evicted.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Coarse progress of one asset. New states may be added, so `match` needs a wildcard arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoadState {
    /// Queued or being loaded.
    Loading,
    /// [`Handle::get`] returns the asset.
    Ready,
    /// Loading failed; see [`Handle::error`]. The failure is cached while the entry lives (with
    /// `evict_unreferenced`, until the last handle is dropped, so a later request retries).
    Failed,
}

/// A typed, cloneable reference to an asset. Cloning is a refcount bump; the asset itself is
/// shared as an [`Arc<T>`].
pub struct Handle<T> {
    slot: Arc<Slot>,
    path: VfsPath,
    _asset: PhantomData<fn() -> T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            slot: Arc::clone(&self.slot),
            path: self.path.clone(),
            _asset: PhantomData,
        }
    }
}

impl<T> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Handle")
            .field("path", &self.path.as_str())
            .field("state", &self.state())
            .finish()
    }
}

impl<T> Handle<T> {
    /// The normalized path this handle was loaded from.
    pub fn path(&self) -> &VfsPath {
        &self.path
    }

    /// Current [`LoadState`].
    pub fn state(&self) -> LoadState {
        self.slot.state()
    }

    /// Block until the asset is no longer loading; returns [`LoadState::Ready`] or
    /// [`LoadState::Failed`]. Returns at once for a handle that is done.
    pub fn wait(&self) -> LoadState {
        self.slot.wait()
    }

    /// Block until this asset **and everything it depends on** is done. [`LoadState::Failed`] if
    /// any of them failed (dependencies that failed do not fail their parent, see
    /// [`LoadContext::load_dependency`]; this tells the caller whether the whole set is usable).
    pub fn wait_all(&self) -> LoadState {
        self.slot.wait_all()
    }

    /// Why loading failed, if it did (the loader's error message).
    pub fn error(&self) -> Option<String> {
        match &*self.slot.lock() {
            SlotState::Failed(msg) => Some(msg.clone()),
            _ => None,
        }
    }

    /// Paths this asset asked for through [`LoadContext::load_dependency`] or
    /// [`LoadContext::load_any`]; empty until ready.
    pub fn dependencies(&self) -> Vec<VfsPath> {
        self.slot
            .dependencies()
            .into_iter()
            .map(|d| d.path)
            .collect()
    }

    /// `true` if both handles refer to the same cached asset.
    pub fn same_asset(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.slot, &other.slot)
    }
}

impl<T: Send + Sync + 'static> Handle<T> {
    /// The loaded asset, or `None` while loading or after a failure.
    pub fn get(&self) -> Option<Arc<T>> {
        match &*self.slot.lock() {
            SlotState::Ready { asset, .. } => Arc::clone(asset).downcast::<T>().ok(),
            _ => None,
        }
    }
}

/// A reference to an asset whose type the requester does not know (what [`LoadContext::load_any`]
/// returns): enough to wait for it, to see whether it failed, and to keep it alive.
#[derive(Clone)]
pub struct AnyHandle {
    slot: Arc<Slot>,
    path: VfsPath,
}

impl fmt::Debug for AnyHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnyHandle")
            .field("path", &self.path.as_str())
            .field("state", &self.slot.state())
            .finish()
    }
}

impl AnyHandle {
    /// The normalized path.
    pub fn path(&self) -> &VfsPath {
        &self.path
    }

    /// Current [`LoadState`].
    pub fn state(&self) -> LoadState {
        self.slot.state()
    }

    /// Block until the asset is no longer loading.
    pub fn wait(&self) -> LoadState {
        self.slot.wait()
    }

    /// Block until this asset and everything it depends on is done.
    pub fn wait_all(&self) -> LoadState {
        self.slot.wait_all()
    }

    /// Why loading failed, if it did.
    pub fn error(&self) -> Option<String> {
        match &*self.slot.lock() {
            SlotState::Failed(msg) => Some(msg.clone()),
            _ => None,
        }
    }

    /// The typed handle, if the asset is a `T`. `None` for another type.
    pub fn downcast<T: Send + Sync + 'static>(&self) -> Option<Handle<T>> {
        // The asset type is known only once loaded; before that, trust the caller's `T` when the
        // slot is still loading and verify it on `get`.
        match &*self.slot.lock() {
            SlotState::Ready { asset, .. } if !(**asset).is::<T>() => return None,
            _ => {}
        }
        Some(Handle {
            slot: Arc::clone(&self.slot),
            path: self.path.clone(),
            _asset: PhantomData,
        })
    }
}

/// What an [`AssetLoader`] sees while loading one asset.
pub struct LoadContext<'a> {
    path: &'a VfsPath,
    core: &'a Arc<Core>,
    dependencies: Vec<Dependency>,
}

impl LoadContext<'_> {
    /// The (normalized) path being loaded.
    pub fn path(&self) -> &VfsPath {
        self.path
    }

    /// The file system, for loaders that need sibling files or streaming access.
    pub fn vfs(&self) -> &dyn Vfs {
        &*self.core.vfs
    }

    /// Read the whole file being loaded.
    ///
    /// # Errors
    /// [`Error::NotFound`] / [`Error::Io`] from the [`Vfs`].
    pub fn read(&self) -> Result<Vec<u8>> {
        self.core.vfs.read(self.path)
    }

    fn record(&mut self, dependency: Dependency) {
        if !self
            .dependencies
            .iter()
            .any(|d| Arc::ptr_eq(&d.slot, &dependency.slot))
        {
            self.dependencies.push(dependency);
        }
    }

    /// Request another asset that this one refers to (a model's textures, a world's sounds).
    /// The dependency is recorded (see [`Handle::dependencies`]) and loaded through the same
    /// manager, so it is cached and shared; it stays alive as long as this asset does. The
    /// returned handle may not be ready yet, and a dependency that fails does not fail this
    /// asset.
    ///
    /// # Errors
    /// Same as [`AssetManager::load`]: bad path, no loader, or the loader makes a different type.
    pub fn load_dependency<D: Send + Sync + 'static>(&mut self, path: &str) -> Result<Handle<D>> {
        let path = VfsPath::new(path)?;
        let slot = self.core.request(&path, Some(TypeId::of::<D>()))?;
        self.record(Dependency {
            path: path.clone(),
            slot: Arc::clone(&slot),
        });
        Ok(Handle {
            slot,
            path,
            _asset: PhantomData,
        })
    }

    /// Like [`LoadContext::load_dependency`] for a reference whose asset type the caller does not
    /// care about: the loader registered for the extension decides. This is how the file
    /// references of a CTSEMETA container (`RFIL`) become dependencies.
    ///
    /// # Errors
    /// [`Error::Invalid`] for a bad path or one without extension; [`Error::NotFound`] if no
    /// loader handles the extension.
    pub fn load_any(&mut self, path: &str) -> Result<AnyHandle> {
        let path = VfsPath::new(path)?;
        let slot = self.core.request(&path, None)?;
        self.record(Dependency {
            path: path.clone(),
            slot: Arc::clone(&slot),
        });
        Ok(AnyHandle { slot, path })
    }
}

type Job = Box<dyn FnOnce() + Send>;

#[derive(Default)]
struct Queue {
    jobs: Mutex<VecDeque<Job>>,
    wake: Condvar,
    closed: AtomicBool,
}

enum Entry {
    Strong(Arc<Slot>),
    Weak(Weak<Slot>),
}

impl Entry {
    fn get(&self) -> Option<Arc<Slot>> {
        match self {
            Self::Strong(s) => Some(Arc::clone(s)),
            Self::Weak(w) => w.upgrade(),
        }
    }
}

#[derive(Default)]
struct Inner {
    loaders: LoaderRegistry,
    slots: BTreeMap<(VfsPath, TypeId), Entry>,
    failures: VecDeque<(VfsPath, String)>,
}

struct Core {
    vfs: Arc<dyn Vfs>,
    options: AssetOptions,
    inner: Mutex<Inner>,
    queue: Queue,
    counters: Arc<Counters>,
    /// Loads queued or running, for [`AssetManager::wait_idle`].
    pending: Mutex<usize>,
    idle: Condvar,
}

impl Core {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Find or create the cache entry of `path` and start loading it when it is new.
    fn request(self: &Arc<Self>, path: &VfsPath, expect: Option<TypeId>) -> Result<Arc<Slot>> {
        let ext = path
            .extension()
            .ok_or_else(|| Error::invalid("asset path", format!("{path} has no extension")))?;
        let (loader, slot, first_request) = {
            let mut inner = self.lock();
            let loader = inner
                .loaders
                .get(ext)
                .ok_or_else(|| Error::NotFound(format!("no asset loader for .{ext} ({path})")))?;
            if let Some(want) = expect.filter(|want| *want != loader.asset_type()) {
                let _ = want;
                return Err(Error::invalid(
                    "asset type",
                    format!(
                        "the .{ext} loader produces {}, not the type requested for {path}",
                        loader.asset_type_name(),
                    ),
                ));
            }
            let key = (path.clone(), loader.asset_type());
            match inner.slots.get(&key).and_then(Entry::get) {
                Some(slot) => (loader, slot, false),
                None => {
                    let slot = Slot::new(Arc::clone(&self.counters));
                    let entry = if self.options.evict_unreferenced {
                        Entry::Weak(Arc::downgrade(&slot))
                    } else {
                        Entry::Strong(Arc::clone(&slot))
                    };
                    inner.slots.insert(key, entry);
                    (loader, slot, true)
                }
            }
        };
        if first_request {
            self.counters.requested.fetch_add(1, Ordering::Relaxed);
            *self.pending.lock().unwrap_or_else(PoisonError::into_inner) += 1;
            let core = Arc::clone(self);
            let (job_slot, job_path) = (Arc::clone(&slot), path.clone());
            let job: Job = Box::new(move || core.run_load(&loader, &job_path, &job_slot));
            if self.options.workers == 0 {
                job();
            } else {
                self.queue
                    .jobs
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push_back(job);
                self.queue.wake.notify_one();
            }
        }
        Ok(slot)
    }

    /// Run one load and publish the outcome. The manager lock is not held: a loader may request
    /// dependencies, which re-enter [`Core::request`].
    fn run_load(
        self: &Arc<Self>,
        loader: &Arc<dyn ErasedLoader>,
        path: &VfsPath,
        slot: &Arc<Slot>,
    ) {
        let mut ctx = LoadContext {
            path,
            core: self,
            dependencies: Vec::new(),
        };
        let outcome =
            catch_unwind(AssertUnwindSafe(|| loader.load(&mut ctx))).unwrap_or_else(|_| {
                Err(Error::invalid(
                    "asset loader",
                    format!("the loader panicked on {path}"),
                ))
            });
        let dependencies = ctx.dependencies;
        let next = match outcome {
            Ok(asset) => {
                self.counters.loaded.fetch_add(1, Ordering::Relaxed);
                SlotState::Ready {
                    asset,
                    dependencies,
                }
            }
            Err(e) => {
                tracing::warn!(target: "ss2_assets", "failed to load {path}: {e}");
                self.counters.failed.fetch_add(1, Ordering::Relaxed);
                let mut inner = self.lock();
                if inner.failures.len() >= MAX_FAILURES {
                    inner.failures.pop_front();
                }
                inner.failures.push_back((path.clone(), e.to_string()));
                SlotState::Failed(e.to_string())
            }
        };
        *slot.lock() = next;
        slot.done.notify_all();
        let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        *pending -= 1;
        if *pending == 0 {
            self.idle.notify_all();
        }
    }

    fn worker(self: &Arc<Self>) {
        loop {
            let job = {
                let mut jobs = self
                    .queue
                    .jobs
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                loop {
                    if let Some(job) = jobs.pop_front() {
                        break Some(job);
                    }
                    if self.queue.closed.load(Ordering::Acquire) {
                        break None;
                    }
                    jobs = self
                        .queue
                        .wake
                        .wait(jobs)
                        .unwrap_or_else(PoisonError::into_inner);
                }
            };
            match job {
                Some(job) => job(),
                None => return,
            }
        }
    }
}

/// Loads assets through registered [`AssetLoader`]s and caches them by `(path, type)`.
///
/// `&self` everywhere: share one manager (`Arc<AssetManager>`) across the whole engine. Dropping
/// the manager lets queued loads finish and stops the worker threads.
pub struct AssetManager {
    core: Arc<Core>,
    workers: Vec<JoinHandle<()>>,
}

impl fmt::Debug for AssetManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssetManager")
            .field("options", &self.core.options)
            .finish_non_exhaustive()
    }
}

impl Drop for AssetManager {
    fn drop(&mut self) {
        self.core.queue.closed.store(true, Ordering::Release);
        self.core.queue.wake.notify_all();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

impl AssetManager {
    /// A manager with no loaders, reading files from `vfs`: synchronous, never evicting
    /// ([`AssetOptions::default`]).
    pub fn new(vfs: Arc<dyn Vfs>) -> Self {
        Self::with_options(vfs, AssetOptions::default())
    }

    /// A manager with the given [`AssetOptions`].
    pub fn with_options(vfs: Arc<dyn Vfs>, options: AssetOptions) -> Self {
        let core = Arc::new(Core {
            vfs,
            options,
            inner: Mutex::new(Inner::default()),
            queue: Queue::default(),
            counters: Arc::new(Counters::default()),
            pending: Mutex::new(0),
            idle: Condvar::new(),
        });
        let workers = (0..options.workers)
            .map(|i| {
                let core = Arc::clone(&core);
                std::thread::Builder::new()
                    .name(format!("ss2-assets-{i}"))
                    .spawn(move || core.worker())
                    .expect("the operating system can start a thread")
            })
            .collect();
        Self { core, workers }
    }

    /// Register `loader` for each of its [`extensions`](AssetLoader::extensions).
    ///
    /// Each format crate can implement its own loader and pass it to this method at startup.
    /// All extensions are registered together or the registry is left unchanged.
    ///
    /// # Errors
    /// [`Error::Invalid`] if the list is empty, an extension cannot be a VFS file suffix, or
    /// another loader already handles an extension in the list.
    pub fn register_loader<L: AssetLoader>(&self, loader: L) -> Result<()> {
        let extensions = normalize_extensions(loader.extensions())?;
        let loader = erase(loader);
        self.core.lock().loaders.register(extensions, &loader)
    }

    /// Request the asset at `path` as type `T`, loading it if it is not cached yet.
    ///
    /// With worker threads the handle is returned at once and may be [`LoadState::Loading`];
    /// without them the load has finished when this returns. Loading problems (missing file,
    /// corrupt data) do **not** make this return `Err`: they end up in the handle
    /// ([`LoadState::Failed`], [`Handle::error`]) so one bad asset never aborts a level. `Err` is
    /// for caller mistakes only.
    ///
    /// # Errors
    /// [`Error::Invalid`] for a bad path, a path without extension, or when the extension's
    /// loader produces a type other than `T`; [`Error::NotFound`] if no loader handles the
    /// extension.
    pub fn load<T: Send + Sync + 'static>(&self, path: &str) -> Result<Handle<T>> {
        let path = VfsPath::new(path)?;
        let slot = self.core.request(&path, Some(TypeId::of::<T>()))?;
        Ok(Handle {
            slot,
            path,
            _asset: PhantomData,
        })
    }

    /// Block until no load is queued or running (not counting loads requested meanwhile).
    pub fn wait_idle(&self) {
        let mut pending = self
            .core
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        while *pending > 0 {
            pending = self
                .core
                .idle
                .wait(pending)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// The counters, and the number of live cache entries.
    pub fn stats(&self) -> AssetStats {
        let c = &self.core.counters;
        let cached = self
            .core
            .lock()
            .slots
            .values()
            .filter(|e| e.get().is_some())
            .count();
        AssetStats {
            requested: c.requested.load(Ordering::Relaxed),
            loaded: c.loaded.load(Ordering::Relaxed),
            failed: c.failed.load(Ordering::Relaxed),
            evicted: c.evicted.load(Ordering::Relaxed),
            cached,
        }
    }

    /// Drop cache entries whose asset was freed (only with `evict_unreferenced`); returns how
    /// many. Entries are also replaced when the path is requested again; call this to keep the
    /// table small in a long-running game.
    pub fn collect(&self) -> usize {
        let mut inner = self.core.lock();
        let before = inner.slots.len();
        inner.slots.retain(|_, e| e.get().is_some());
        before - inner.slots.len()
    }

    /// Every dependency edge of every loaded asset that is still alive: asset path to the paths
    /// it asked for (sorted).
    pub fn dependency_graph(&self) -> BTreeMap<VfsPath, Vec<VfsPath>> {
        let slots: Vec<_> = self
            .core
            .lock()
            .slots
            .iter()
            .filter_map(|((path, _), e)| e.get().map(|s| (path.clone(), s)))
            .collect();
        let mut graph = BTreeMap::new();
        for (path, slot) in slots {
            let mut deps: Vec<VfsPath> = slot.dependencies().into_iter().map(|d| d.path).collect();
            deps.sort();
            if !deps.is_empty() {
                graph.insert(path, deps);
            }
        }
        graph
    }

    /// The loads that failed, oldest first, as `(path, message)` (at most the last 10,000).
    pub fn failures(&self) -> Vec<(VfsPath, String)> {
        self.core.lock().failures.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use ss2_vfs::MemoryVfs;

    use super::*;

    struct TextLoader;
    impl AssetLoader for TextLoader {
        type Asset = String;
        fn extensions(&self) -> &[&str] {
            &["txt", ".TXT2"]
        }
        fn load(&self, ctx: &mut LoadContext<'_>) -> Result<String> {
            String::from_utf8(ctx.read()?).map_err(|e| Error::invalid("text asset", e.to_string()))
        }
    }

    /// `.lst`: one dependency path per line; the asset is the line count.
    struct ListLoader;
    impl AssetLoader for ListLoader {
        type Asset = usize;
        fn extensions(&self) -> &[&str] {
            &["lst"]
        }
        fn load(&self, ctx: &mut LoadContext<'_>) -> Result<usize> {
            let text = String::from_utf8(ctx.read()?)
                .map_err(|e| Error::invalid("list asset", e.to_string()))?;
            let mut n = 0;
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                let _ = if line.ends_with(".lst") {
                    ctx.load_dependency::<usize>(line).map(drop)
                } else {
                    ctx.load_dependency::<String>(line).map(drop)
                };
                n += 1;
            }
            Ok(n)
        }
    }

    fn manager_with(files: &[(&str, &[u8])], options: AssetOptions) -> AssetManager {
        let vfs = MemoryVfs::from_files(files.iter().copied()).unwrap();
        let m = AssetManager::with_options(Arc::new(vfs), options);
        m.register_loader(TextLoader).unwrap();
        m.register_loader(ListLoader).unwrap();
        m
    }

    fn manager(files: &[(&str, &[u8])]) -> AssetManager {
        manager_with(files, AssetOptions::default())
    }

    fn threaded(files: &[(&str, &[u8])]) -> AssetManager {
        manager_with(
            files,
            AssetOptions {
                workers: 3,
                evict_unreferenced: false,
            },
        )
    }

    #[test]
    fn loads_and_caches_by_normalized_path() {
        let m = manager(&[("a/b.txt", b"hello")]);
        let h1 = m.load::<String>("A/B.txt").unwrap();
        let h2 = m.load::<String>(r"a\b.TXT").unwrap();
        assert_eq!(h1.state(), LoadState::Ready);
        assert_eq!(h1.get().unwrap().as_str(), "hello");
        assert!(h1.same_asset(&h2));
        assert!(Arc::ptr_eq(&h1.get().unwrap(), &h2.get().unwrap()));
        assert_eq!(h1.path().as_str(), "a/b.txt");
    }

    #[test]
    fn loader_registered_for_several_extensions() {
        let m = manager(&[("x.txt2", b"two")]);
        assert_eq!(
            m.load::<String>("x.TXT2").unwrap().get().unwrap().as_str(),
            "two"
        );
    }

    #[test]
    fn missing_file_and_bad_data_become_failed_handles() {
        let m = manager(&[("bad.txt", &[0xFF, 0xFE])]);
        let missing = m.load::<String>("nope.txt").unwrap();
        assert_eq!(missing.state(), LoadState::Failed);
        assert!(missing.get().is_none());
        assert!(missing.error().unwrap().contains("nope.txt"));

        let bad = m.load::<String>("bad.txt").unwrap();
        assert_eq!(bad.state(), LoadState::Failed);
        assert!(
            bad.error().unwrap().contains("text asset"),
            "{:?}",
            bad.error()
        );
        assert!(bad.dependencies().is_empty());
    }

    #[test]
    fn caller_mistakes_are_errors() {
        let m = manager(&[("a.txt", b"x")]);
        assert!(matches!(
            m.load::<String>("a.unknown"),
            Err(Error::NotFound(_))
        ));
        assert!(matches!(
            m.load::<String>("noextension"),
            Err(Error::Invalid { .. })
        ));
        assert!(matches!(
            m.load::<String>("../a.txt"),
            Err(Error::Invalid { .. })
        ));
        let wrong = m.load::<u32>("a.txt").unwrap_err().to_string();
        assert!(
            wrong.contains("String") && wrong.contains("a.txt"),
            "{wrong}"
        );
    }

    #[test]
    fn duplicate_extension_registration_is_rejected_atomically() {
        struct Clash;
        impl AssetLoader for Clash {
            type Asset = u8;
            fn extensions(&self) -> &[&str] {
                &["new", ".TXT"]
            }
            fn load(&self, _: &mut LoadContext<'_>) -> Result<u8> {
                Ok(0)
            }
        }
        let m = manager(&[("a.new", b"x")]);
        assert!(m.register_loader(Clash).is_err());
        assert!(
            matches!(m.load::<u8>("a.new"), Err(Error::NotFound(_))),
            "`.new` must not be registered when the batch is rejected"
        );
    }

    #[test]
    fn invalid_extension_registration_is_rejected_atomically() {
        struct Plugin(&'static [&'static str]);
        impl AssetLoader for Plugin {
            type Asset = ();
            fn extensions(&self) -> &[&str] {
                self.0
            }
            fn load(&self, _: &mut LoadContext<'_>) -> Result<()> {
                Ok(())
            }
        }

        let m = manager(&[]);
        for extensions in [
            &[][..],
            &["new", ""][..],
            &["new", "."],
            &["new", "multiple.parts"],
            &["new", "nested/ext"],
            &["new", "nested\\ext"],
            &["new", "drive:ext"],
            &["new", "bad\0ext"],
        ] {
            assert!(
                matches!(
                    m.register_loader(Plugin(extensions)),
                    Err(Error::Invalid { .. })
                ),
                "accepted {extensions:?}"
            );
            assert!(matches!(m.load::<()>("a.new"), Err(Error::NotFound(_))));
        }

        m.register_loader(Plugin(&["new"])).unwrap();
        assert_eq!(m.load::<()>("a.NEW").unwrap().state(), LoadState::Ready);
    }

    #[test]
    fn dependencies_are_recorded_loaded_and_shared() {
        let m = manager(&[
            ("level.lst", b"tex/a.txt\ntex/b.txt\ntex/A.txt\n"),
            ("tex/a.txt", b"A"),
            ("tex/b.txt", b"B"),
        ]);
        let level = m.load::<usize>("level.lst").unwrap();
        assert_eq!(*level.get().unwrap(), 3);
        let deps: Vec<_> = level
            .dependencies()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            deps,
            ["tex/a.txt", "tex/b.txt"],
            "de-duplicated, in request order"
        );
        let a = m.load::<String>("tex/a.txt").unwrap();
        assert_eq!(a.get().unwrap().as_str(), "A");
    }

    #[test]
    fn missing_dependency_does_not_fail_the_parent() {
        let m = manager(&[("level.lst", b"gone.txt\n")]);
        let level = m.load::<usize>("level.lst").unwrap();
        assert_eq!(level.state(), LoadState::Ready);
        assert_eq!(
            m.load::<String>("gone.txt").unwrap().state(),
            LoadState::Failed
        );
        assert_eq!(
            level.wait_all(),
            LoadState::Failed,
            "the closure is not usable"
        );
    }

    #[test]
    fn cyclic_dependencies_terminate() {
        let m = manager(&[("a.lst", b"b.lst\n"), ("b.lst", b"a.lst\n")]);
        let a = m.load::<usize>("a.lst").unwrap();
        assert_eq!(a.state(), LoadState::Ready);
        assert_eq!(m.load::<usize>("b.lst").unwrap().state(), LoadState::Ready);
        assert_eq!(a.wait_all(), LoadState::Ready);
    }

    #[test]
    fn handles_are_send_sync_and_debug() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Handle<String>>();
        assert_send_sync::<AnyHandle>();
        assert_send_sync::<AssetManager>();
        let m = manager(&[("a.txt", b"x")]);
        let dbg = format!("{:?}", m.load::<String>("a.txt").unwrap());
        assert!(dbg.contains("a.txt") && dbg.contains("Ready"), "{dbg}");
    }

    #[test]
    fn shared_across_threads() {
        let m = Arc::new(manager(&[("a.txt", b"x")]));
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let m = Arc::clone(&m);
                std::thread::spawn(move || m.load::<String>("a.txt").unwrap())
            })
            .collect();
        let handles: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert!(handles.windows(2).all(|w| w[0].same_asset(&w[1])));
    }

    #[test]
    fn worker_threads_load_in_the_background() {
        let files: Vec<(String, Vec<u8>)> = (0..50)
            .map(|i| (format!("t/{i}.txt"), format!("n{i}").into_bytes()))
            .collect();
        let refs: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(p, d)| (p.as_str(), d.as_slice()))
            .collect();
        let m = threaded(&refs);
        let handles: Vec<_> = (0..50)
            .map(|i| m.load::<String>(&format!("t/{i}.txt")).unwrap())
            .collect();
        m.wait_idle();
        for (i, h) in handles.iter().enumerate() {
            assert_eq!(h.wait(), LoadState::Ready);
            assert_eq!(h.get().unwrap().as_str(), format!("n{i}"));
        }
        let s = m.stats();
        assert_eq!((s.requested, s.loaded, s.failed, s.cached), (50, 50, 0, 50));
    }

    #[test]
    fn threaded_dependencies_cycles_and_failures() {
        let m = threaded(&[
            ("a.lst", b"b.lst\nx.txt\nmissing.txt\n"),
            ("b.lst", b"a.lst\n"),
            ("x.txt", b"X"),
        ]);
        let a = m.load::<usize>("a.lst").unwrap();
        assert_eq!(a.wait_all(), LoadState::Failed, "one dependency is missing");
        assert_eq!(a.get().as_deref(), Some(&3));
        let graph = m.dependency_graph();
        let a_deps: Vec<_> = graph[&VfsPath::new("a.lst").unwrap()]
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(a_deps, ["b.lst", "missing.txt", "x.txt"]);
        assert_eq!(graph[&VfsPath::new("b.lst").unwrap()].len(), 1);
        let failures = m.failures();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].0.as_str(), "missing.txt");
        assert!(failures[0].1.contains("missing.txt"));
    }

    #[test]
    fn a_loader_that_panics_becomes_a_failed_handle() {
        struct Boom;
        impl AssetLoader for Boom {
            type Asset = u8;
            fn extensions(&self) -> &[&str] {
                &["boom"]
            }
            fn load(&self, _: &mut LoadContext<'_>) -> Result<u8> {
                panic!("on purpose");
            }
        }
        for options in [
            AssetOptions::default(),
            AssetOptions {
                workers: 2,
                evict_unreferenced: false,
            },
        ] {
            let m = manager_with(&[("a.boom", b"x")], options);
            m.register_loader(Boom).unwrap();
            let h = m.load::<u8>("a.boom").unwrap();
            assert_eq!(h.wait(), LoadState::Failed);
            assert!(h.error().unwrap().contains("panicked"), "{:?}", h.error());
            assert_eq!(m.failures().len(), 1);
        }
    }

    /// Counts how often it runs.
    struct Counting(Arc<AtomicUsize>);
    impl AssetLoader for Counting {
        type Asset = usize;
        fn extensions(&self) -> &[&str] {
            &["cnt"]
        }
        fn load(&self, _: &mut LoadContext<'_>) -> Result<usize> {
            Ok(self.0.fetch_add(1, Ordering::SeqCst) + 1)
        }
    }

    #[test]
    fn unreferenced_assets_are_evicted_and_reloaded() {
        let runs = Arc::new(AtomicUsize::new(0));
        let m = manager_with(
            &[("a.cnt", b"")],
            AssetOptions {
                workers: 0,
                evict_unreferenced: true,
            },
        );
        m.register_loader(Counting(Arc::clone(&runs))).unwrap();

        let h = m.load::<usize>("a.cnt").unwrap();
        let again = m.load::<usize>("a.cnt").unwrap();
        assert!(h.same_asset(&again));
        assert_eq!(
            runs.load(Ordering::SeqCst),
            1,
            "a live handle keeps the cache entry"
        );
        drop((h, again));
        assert_eq!(m.stats().evicted, 1);
        assert_eq!(m.stats().cached, 0);

        let fresh = m.load::<usize>("a.cnt").unwrap();
        assert_eq!(*fresh.get().unwrap(), 2, "it was loaded again");
        assert_eq!(runs.load(Ordering::SeqCst), 2);
        drop(fresh);
        assert_eq!(m.collect(), 1);
    }

    #[test]
    fn a_parent_keeps_its_dependencies_alive() {
        let m = manager_with(
            &[("level.lst", b"a.txt\n"), ("a.txt", b"A")],
            AssetOptions {
                workers: 0,
                evict_unreferenced: true,
            },
        );
        let level = m.load::<usize>("level.lst").unwrap();
        // Nobody else holds `a.txt`: the parent does.
        assert_eq!(m.stats().cached, 2);
        let a = m.load::<String>("a.txt").unwrap();
        assert!(level.dependencies().iter().any(|p| p.as_str() == "a.txt"));
        drop(a);
        assert_eq!(m.stats().evicted, 0, "still referenced by `level.lst`");
        drop(level);
        assert_eq!(
            m.stats().evicted,
            2,
            "parent and dependency are freed together"
        );
    }

    #[test]
    fn failures_are_retried_after_the_handle_is_dropped_when_evicting() {
        let m = manager_with(
            &[],
            AssetOptions {
                workers: 0,
                evict_unreferenced: true,
            },
        );
        let h = m.load::<String>("late.txt").unwrap();
        assert_eq!(h.state(), LoadState::Failed);
        drop(h);
        let h2 = m.load::<String>("late.txt").unwrap();
        assert_eq!(h2.state(), LoadState::Failed);
        assert_eq!(m.stats().requested, 2, "the second request tried again");
        assert_eq!(m.failures().len(), 2);
    }

    #[test]
    fn load_any_follows_the_extension_and_downcasts() {
        struct RefsLoader;
        impl AssetLoader for RefsLoader {
            type Asset = usize;
            fn extensions(&self) -> &[&str] {
                &["refs"]
            }
            fn load(&self, ctx: &mut LoadContext<'_>) -> Result<usize> {
                let text = String::from_utf8_lossy(&ctx.read()?).into_owned();
                let mut n = 0;
                for line in text.lines() {
                    ctx.load_any(line)?;
                    n += 1;
                }
                Ok(n)
            }
        }
        let m = threaded(&[
            ("m.refs", b"a.txt\nb.txt\n"),
            ("a.txt", b"A"),
            ("b.txt", b"B"),
        ]);
        m.register_loader(RefsLoader).unwrap();
        let h = m.load::<usize>("m.refs").unwrap();
        assert_eq!(h.wait_all(), LoadState::Ready);
        assert_eq!(h.dependencies().len(), 2);
        // A loader can mix typed and untyped requests, and an unknown extension is an error.
        struct Probe;
        impl AssetLoader for Probe {
            type Asset = u8;
            fn extensions(&self) -> &[&str] {
                &["probe"]
            }
            fn load(&self, ctx: &mut LoadContext<'_>) -> Result<u8> {
                let any = ctx.load_any("a.txt")?;
                assert_eq!(any.wait(), LoadState::Ready);
                assert!(any.downcast::<String>().is_some());
                assert!(any.downcast::<u32>().is_none());
                assert!(matches!(
                    ctx.load_any("nothing.unknown"),
                    Err(Error::NotFound(_))
                ));
                Ok(1)
            }
        }
        m.register_loader(Probe).unwrap();
        let vfs_probe = manager_with(
            &[("x.probe", b""), ("a.txt", b"A")],
            AssetOptions::default(),
        );
        vfs_probe.register_loader(Probe).unwrap();
        let ok = vfs_probe.load::<u8>("x.probe").unwrap();
        assert_eq!(ok.state(), LoadState::Ready, "{:?}", ok.error());
        let a = m.load::<String>("a.txt").unwrap();
        assert_eq!(a.get().unwrap().as_str(), "A");
    }

    #[test]
    fn dropping_a_threaded_manager_finishes_and_joins() {
        let m = threaded(&[("a.txt", b"x")]);
        let h = m.load::<String>("a.txt").unwrap();
        drop(m);
        assert_eq!(
            h.wait(),
            LoadState::Ready,
            "queued work finished before the join"
        );
    }
}
