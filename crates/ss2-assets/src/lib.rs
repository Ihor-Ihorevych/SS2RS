//! ss2-assets — asset manager, loader registry and cooked cache (track T02).
//!
//! This is the **v0 contract** (roadmap T00-014, `docs/rfcs/0001-vfs-and-asset-api.md`). Three
//! things make up the API that renderer, audio, world and gameplay code are written against:
//!
//! - [`AssetLoader`]: implemented once per file format (`.tex`, `.mdl`, ...) by the format crates.
//! - [`AssetManager`]: owns a [`Vfs`], a registry of loaders and a cache; hands out [`Handle`]s.
//! - [`Handle`]: a cheap, cloneable, typed reference to an asset that may still be loading.
//!
//! The v0 [`AssetManager`] is deliberately simple (it loads **synchronously** on the calling
//! thread, never evicts) but it is a real implementation, so consumers can use it in tests today.
//! T02-009 replaces the internals (async loading, refcount eviction, dependency graph) **without
//! changing these signatures**: code must already treat a handle as possibly [`LoadState::Loading`].
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
//! assert_eq!(handle.state(), LoadState::Ready);      // v0 loads synchronously
//! assert_eq!(handle.get().as_deref().map(String::as_str), Some("hi there"));
//! ```

use std::any::{Any, TypeId};
use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use ss2_core::{Error, Result};
use ss2_vfs::{Vfs, VfsPath};

/// Knows how to turn the bytes of one file format into an asset value.
///
/// Implementations must be stateless or internally synchronized: the manager may call
/// [`load`](Self::load) from several threads. They must **never panic on bad input**: return
/// [`Error::Invalid`] (`ss2_core::Error::invalid(..)`) instead.
pub trait AssetLoader: Send + Sync + 'static {
    /// The in-memory type this loader produces.
    type Asset: Send + Sync + 'static;

    /// Lower-case file extensions without the dot, e.g. `&["tex"]`. Each extension maps to at
    /// most one loader per manager.
    fn extensions(&self) -> &[&str];

    /// Parse the asset at [`LoadContext::path`].
    ///
    /// # Errors
    /// Any [`Error`]; it is stored in the asset's [`Handle`] and reported, not propagated.
    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<Self::Asset>;
}

/// What an [`AssetLoader`] sees while loading one asset.
pub struct LoadContext<'a> {
    path: &'a VfsPath,
    manager: &'a AssetManager,
    dependencies: Vec<VfsPath>,
}

impl LoadContext<'_> {
    /// The (normalized) path being loaded.
    pub fn path(&self) -> &VfsPath {
        self.path
    }

    /// The file system, for loaders that need sibling files or streaming access.
    pub fn vfs(&self) -> &dyn Vfs {
        &*self.manager.vfs
    }

    /// Read the whole file being loaded.
    ///
    /// # Errors
    /// [`Error::NotFound`] / [`Error::Io`] from the [`Vfs`].
    pub fn read(&self) -> Result<Vec<u8>> {
        self.manager.vfs.read(self.path)
    }

    /// Request another asset that this one refers to (a model's textures, a world's sounds).
    /// The dependency is recorded (see [`Handle::dependencies`]) and loaded through the same
    /// manager, so it is cached and shared. The returned handle may not be ready yet.
    ///
    /// # Errors
    /// Same as [`AssetManager::load`]: bad path, no loader, or the loader makes a different type.
    pub fn load_dependency<D: Send + Sync + 'static>(&mut self, path: &str) -> Result<Handle<D>> {
        let handle = self.manager.load::<D>(path)?;
        if !self.dependencies.contains(&handle.path) {
            self.dependencies.push(handle.path.clone());
        }
        Ok(handle)
    }
}

/// Coarse progress of one asset. New states may be added, so `match` needs a wildcard arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoadState {
    /// Still being loaded (always transient in v0, observable with cyclic dependencies).
    Loading,
    /// [`Handle::get`] returns the asset.
    Ready,
    /// Loading failed; see [`Handle::error`]. Failures are cached until the manager is rebuilt
    /// (reload/eviction arrive with T02-009).
    Failed,
}

enum SlotState {
    Loading,
    Ready {
        asset: Arc<dyn Any + Send + Sync>,
        dependencies: Vec<VfsPath>,
    },
    Failed(String),
}

struct Slot(Mutex<SlotState>);

impl Slot {
    fn lock(&self) -> MutexGuard<'_, SlotState> {
        // A panic in another thread cannot leave the state half-written, so poison is harmless.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
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
        match &*self.slot.lock() {
            SlotState::Loading => LoadState::Loading,
            SlotState::Ready { .. } => LoadState::Ready,
            SlotState::Failed(_) => LoadState::Failed,
        }
    }

    /// Why loading failed, if it did (the loader's error message).
    pub fn error(&self) -> Option<String> {
        match &*self.slot.lock() {
            SlotState::Failed(msg) => Some(msg.clone()),
            _ => None,
        }
    }

    /// Paths this asset asked for through [`LoadContext::load_dependency`]; empty until ready.
    pub fn dependencies(&self) -> Vec<VfsPath> {
        match &*self.slot.lock() {
            SlotState::Ready { dependencies, .. } => dependencies.clone(),
            _ => Vec::new(),
        }
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

/// Type-erased loader so loaders for different asset types live in one registry.
trait ErasedLoader: Send + Sync {
    fn asset_type(&self) -> TypeId;
    fn asset_type_name(&self) -> &'static str;
    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<Arc<dyn Any + Send + Sync>>;
}

struct Typed<L>(L);

impl<L: AssetLoader> ErasedLoader for Typed<L> {
    fn asset_type(&self) -> TypeId {
        TypeId::of::<L::Asset>()
    }

    fn asset_type_name(&self) -> &'static str {
        std::any::type_name::<L::Asset>()
    }

    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<Arc<dyn Any + Send + Sync>> {
        Ok(Arc::new(self.0.load(ctx)?))
    }
}

#[derive(Default)]
struct Inner {
    loaders: BTreeMap<String, Arc<dyn ErasedLoader>>,
    slots: BTreeMap<(VfsPath, TypeId), Arc<Slot>>,
}

/// Loads assets through registered [`AssetLoader`]s and caches them by `(path, type)`.
///
/// `&self` everywhere: share one manager (`Arc<AssetManager>`) across the whole engine.
pub struct AssetManager {
    vfs: Arc<dyn Vfs>,
    inner: Mutex<Inner>,
}

impl fmt::Debug for AssetManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssetManager").finish_non_exhaustive()
    }
}

impl AssetManager {
    /// A manager with no loaders, reading files from `vfs`.
    pub fn new(vfs: Arc<dyn Vfs>) -> Self {
        Self {
            vfs,
            inner: Mutex::new(Inner::default()),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Register `loader` for each of its [`extensions`](AssetLoader::extensions).
    ///
    /// # Errors
    /// [`Error::Invalid`] if an extension already has a loader (nothing is registered then).
    pub fn register_loader<L: AssetLoader>(&self, loader: L) -> Result<()> {
        let extensions: Vec<String> = loader
            .extensions()
            .iter()
            .map(|e| e.trim_start_matches('.').to_lowercase())
            .collect();
        let mut inner = self.lock();
        if let Some(dup) = extensions.iter().find(|e| inner.loaders.contains_key(*e)) {
            return Err(Error::invalid(
                "asset loader",
                format!("extension .{dup} already has a loader"),
            ));
        }
        let loader: Arc<dyn ErasedLoader> = Arc::new(Typed(loader));
        for ext in extensions {
            inner.loaders.insert(ext, Arc::clone(&loader));
        }
        Ok(())
    }

    /// Request the asset at `path` as type `T`, loading it if it is not cached yet.
    ///
    /// Loading problems (missing file, corrupt data) do **not** make this return `Err`: they end
    /// up in the handle ([`LoadState::Failed`], [`Handle::error`]) so one bad asset never aborts a
    /// level. `Err` is for caller mistakes only.
    ///
    /// # Errors
    /// [`Error::Invalid`] for a bad path, a path without extension, or when the extension's
    /// loader produces a type other than `T`; [`Error::NotFound`] if no loader handles the
    /// extension.
    pub fn load<T: Send + Sync + 'static>(&self, path: &str) -> Result<Handle<T>> {
        let path = VfsPath::new(path)?;
        let ext = path
            .extension()
            .ok_or_else(|| Error::invalid("asset path", format!("{path} has no extension")))?
            .to_owned();
        let key = (path.clone(), TypeId::of::<T>());

        let (loader, slot, first_request) = {
            let mut inner = self.lock();
            let loader =
                inner.loaders.get(&ext).cloned().ok_or_else(|| {
                    Error::NotFound(format!("no asset loader for .{ext} ({path})"))
                })?;
            if loader.asset_type() != TypeId::of::<T>() {
                return Err(Error::invalid(
                    "asset type",
                    format!(
                        "the .{ext} loader produces {}, not {}",
                        loader.asset_type_name(),
                        std::any::type_name::<T>()
                    ),
                ));
            }
            match inner.slots.get(&key) {
                Some(slot) => (loader, Arc::clone(slot), false),
                None => {
                    let slot = Arc::new(Slot(Mutex::new(SlotState::Loading)));
                    inner.slots.insert(key, Arc::clone(&slot));
                    (loader, slot, true)
                }
            }
        };

        // The manager lock is released: a loader may request dependencies, which re-enter `load`.
        if first_request {
            let mut ctx = LoadContext {
                path: &path,
                manager: self,
                dependencies: Vec::new(),
            };
            let outcome = loader.load(&mut ctx);
            *slot.lock() = match outcome {
                Ok(asset) => SlotState::Ready {
                    asset,
                    dependencies: ctx.dependencies,
                },
                Err(e) => {
                    tracing::warn!(target: "ss2_assets", "failed to load {path}: {e}");
                    SlotState::Failed(e.to_string())
                }
            };
        }
        Ok(Handle {
            slot,
            path,
            _asset: PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    use ss2_vfs::MemoryVfs;

    use super::*;

    struct TextLoader;
    impl AssetLoader for TextLoader {
        type Asset = String;
        fn extensions(&self) -> &[&str] {
            &["txt", "TXT2"]
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

    fn manager(files: &[(&str, &[u8])]) -> AssetManager {
        let vfs = MemoryVfs::from_files(files.iter().copied()).unwrap();
        let m = AssetManager::new(Arc::new(vfs));
        m.register_loader(TextLoader).unwrap();
        m.register_loader(ListLoader).unwrap();
        m
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
            m.load::<String>("x.txt2").unwrap().get().unwrap().as_str(),
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
        assert!(wrong.contains("String") && wrong.contains("u32"), "{wrong}");
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
    }

    #[test]
    fn cyclic_dependencies_terminate() {
        let m = manager(&[("a.lst", b"b.lst\n"), ("b.lst", b"a.lst\n")]);
        let a = m.load::<usize>("a.lst").unwrap();
        assert_eq!(a.state(), LoadState::Ready);
        assert_eq!(m.load::<usize>("b.lst").unwrap().state(), LoadState::Ready);
    }

    #[test]
    fn handles_are_send_sync_and_debug() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Handle<String>>();
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
}
