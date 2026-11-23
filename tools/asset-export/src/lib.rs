//! `asset-export`: textures to PNG and models to glTF 2.0, for modders and spec reviewers
//! (roadmap T11-014; documented in `docs/ASSET_EXPORT.md`).
//!
//! * [`tex`]: a `.tex` file to a PNG (mip level, cube face, channel handling); the DXT1/3/5 block
//!   decoders are in [`image`].
//! * [`sheet`]: many textures as thumbnails in one PNG, with a JSON list of the cells.
//! * [`mesh`]: a `.bmf` mesh to a `.gltf` or `.glb` file: positions, normals, tangents, UV sets,
//!   indices, materials with external texture references and the bounding box.
//! * [`batch`]: a whole folder of an install (or on disk) with a deterministic manifest.
//!
//! Nothing here panics on damaged input: every problem is an [`ExportError`].
//!
//! ```
//! use asset_export::tex::{TexOptions, export_texture};
//! # fn demo(tex_file: &[u8]) -> Result<(), asset_export::ExportError> {
//! let out = export_texture(tex_file, &TexOptions::default())?;
//! assert!(out.png.starts_with(b"\x89PNG"));
//! # Ok(()) }
//! ```

pub mod base64;
pub mod batch;
pub mod error;
pub mod glob;
pub mod image;
pub mod mesh;
pub mod sheet;
pub mod source;
pub mod tex;

#[cfg(test)]
mod fixtures;

pub use error::{ExportError, Result};
