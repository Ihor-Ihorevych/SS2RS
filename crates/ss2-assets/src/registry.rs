use std::any::{Any, TypeId};
use std::collections::BTreeMap;
use std::sync::Arc;

use ss2_core::{Error, Result};

use super::{AssetLoader, LoadContext};

/// Type-erased loader so loaders for different asset types live in one registry.
pub(super) trait ErasedLoader: Send + Sync {
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

pub(super) fn erase(loader: impl AssetLoader) -> Arc<dyn ErasedLoader> {
    Arc::new(Typed(loader))
}

pub(super) fn normalize_extensions(extensions: &[&str]) -> Result<Vec<String>> {
    if extensions.is_empty() {
        return Err(Error::invalid("asset loader", "no extensions declared"));
    }
    extensions
        .iter()
        .map(|raw| {
            let extension = raw.trim_start_matches('.');
            // VfsPath::extension returns the suffix after the final dot in one path segment.
            if extension.is_empty()
                || extension
                    .chars()
                    .any(|c| c.is_control() || matches!(c, '.' | '/' | '\\' | ':'))
            {
                return Err(Error::invalid(
                    "asset loader",
                    format!("invalid extension {raw:?}"),
                ));
            }
            Ok(extension.to_lowercase())
        })
        .collect()
}

#[derive(Default)]
pub(super) struct LoaderRegistry {
    by_extension: BTreeMap<String, Arc<dyn ErasedLoader>>,
}

impl LoaderRegistry {
    pub(super) fn register(
        &mut self,
        extensions: Vec<String>,
        loader: &Arc<dyn ErasedLoader>,
    ) -> Result<()> {
        if let Some(duplicate) = extensions
            .iter()
            .find(|ext| self.by_extension.contains_key(*ext))
        {
            return Err(Error::invalid(
                "asset loader",
                format!("extension .{duplicate} already has a loader"),
            ));
        }

        for extension in extensions {
            self.by_extension.insert(extension, Arc::clone(loader));
        }
        Ok(())
    }

    pub(super) fn get(&self, extension: &str) -> Option<Arc<dyn ErasedLoader>> {
        self.by_extension.get(extension).cloned()
    }
}
