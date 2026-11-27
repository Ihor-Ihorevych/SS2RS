//! ss2-format-mtr — the `.mtr` material loader (roadmap T02-018; spec `docs/re/mtr.md`).
//!
//! A `.mtr` file holds one or more **shader presets** (`CShaderPreset`): a short list of *configs*,
//! each pairing a shader class of the engine (an external reference, identified by its class code)
//! with an *arguments object* of one of eight classes. [`MaterialFile::from_bytes`] turns every
//! preset into a [`Material`]: the shader class, the platform mask, the arguments as numbered
//! parameters ([`Param`]) with texture references resolved to file paths, and the class names of
//! the animating modifiers.
//!
//! What is deliberately **not** here: what a shader class computes (`T01-020`/`T01-059`), the
//! meaning of the blend codes and unnamed floats (kept as numbers), and the graphs behind
//! modifiers. The same conversion works for the presets embedded in `.bmf` and `.mdl` files
//! ([`Material::from_preset`]).
//!
//! ```ignore
//! let file = ss2_format_mtr::MaterialFile::from_bytes(&bytes)?;
//! for material in &file.presets {
//!     for config in &material.configs {
//!         println!("{:?} for platforms {:#x}", config.shader, config.platform_mask);
//!         for (member, texture) in config.textures() { /* member number and .tex path */ }
//!     }
//! }
//! ```

use ss2_assets::{AssetLoader, LoadContext};
use ss2_core::{Error, Result};
use ss2_meta::{Container, Decoded, Struct, Value};

const WHAT: &str = ".mtr material";

/// The shader class of a config, from the `EXOB` flag word of its shader reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderClass {
    /// `CShadeShader` (1000): the standard lit material with up to four texture layers.
    Shade,
    /// `CSpecularShader` (1010).
    Specular,
    /// `CReflectionShader` (1020): environment cube map plus normal map.
    Reflection,
    /// `CRefractionShader` (1025).
    Refraction,
    /// `CBlendLayersShader` (1030): three or four blended layers.
    BlendLayers,
    /// `CTerrainShader` (1040): terrain layers blended by a mask.
    Terrain,
    /// `CPolyBumpShader` (1050): normal-mapped lit material (arguments `CPixelightShaderArgs`).
    PolyBump,
    /// `CWaterShader` (1070).
    Water,
    /// A code that is not in the retail data.
    Other(u32),
}

impl ShaderClass {
    /// The class for an `EXOB` flag word.
    pub fn from_code(code: u32) -> Self {
        match code {
            1000 => Self::Shade,
            1010 => Self::Specular,
            1020 => Self::Reflection,
            1025 => Self::Refraction,
            1030 => Self::BlendLayers,
            1040 => Self::Terrain,
            1050 => Self::PolyBump,
            1070 => Self::Water,
            other => Self::Other(other),
        }
    }

    /// The flag word of the class.
    pub fn code(self) -> u32 {
        match self {
            Self::Shade => 1000,
            Self::Specular => 1010,
            Self::Reflection => 1020,
            Self::Refraction => 1025,
            Self::BlendLayers => 1030,
            Self::Terrain => 1040,
            Self::PolyBump => 1050,
            Self::Water => 1070,
            Self::Other(c) => c,
        }
    }
}

/// One argument value (`CShaderArgVal*` wrapper).
#[derive(Clone, Debug, PartialEq)]
pub enum Param {
    /// `CShaderArgValCOLOR`: 32 bit, alpha in the top byte.
    Color(u32),
    /// `CShaderArgValFLOAT`.
    Float(f32),
    /// `CShaderArgValBOOL`.
    Bool(bool),
    /// `CShaderArgValTexture`.
    Texture(TextureRef),
    /// `CShaderArgValUVMap`: the UV map id that picks a stream of the mesh surface (`bmf.md`).
    UvMap(u32),
    /// `CShaderArgValColorMap` (0 in the retail data; meaning unknown).
    ColorMap(i64),
    /// `CShaderArgValBlendType` (`GfuBlendType` code, for example `0x1F4`).
    BlendType(u32),
    /// `CShaderArgValDepthCmp` (`GfxComp` code, for example `0x2A`).
    DepthCmp(u32),
}

/// What a texture slot refers to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextureRef {
    /// An empty slot.
    None,
    /// A `.tex` file (the path as listed in the file's `RFIL`).
    External(String),
    /// A texture object stored inside this file (8 models do this); the object id.
    Embedded(u32),
}

/// One animating modifier of a config (`CShaderArgs.1`). Its target and graph are not decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Modifier {
    /// Class name (`CConstantShaderModifier`, `CFloatGraphShaderModifier`, `CGradientShaderModifier`).
    pub class: String,
}

/// A shader with its arguments.
#[derive(Clone, Debug, PartialEq)]
pub struct ShaderConfig {
    /// The shader class.
    pub shader: ShaderClass,
    /// `EPlatform` mask of the config (bit 0 PC, bit 1 Xbox).
    pub platform_mask: u32,
    /// The arguments class (`CShadeShaderArgs`, `CSpecularShaderArgs`, …).
    pub args_class: String,
    /// The members every arguments class shares (`CShaderCommonParams.1`, `.2`, `.5` …): blend
    /// type, depth comparison and switches, by member number.
    pub common: Vec<(u32, Param)>,
    /// The members of the arguments class itself, by member number, in file order.
    pub params: Vec<(u32, Param)>,
    /// The modifiers.
    pub modifiers: Vec<Modifier>,
}

impl ShaderConfig {
    /// The parameter `member` of the arguments class.
    pub fn param(&self, member: u32) -> Option<&Param> {
        self.params
            .iter()
            .find(|(m, _)| *m == member)
            .map(|(_, p)| p)
    }

    /// The common parameter `member` (for example 1 = blend type, 2 = depth comparison).
    pub fn common_param(&self, member: u32) -> Option<&Param> {
        self.common
            .iter()
            .find(|(m, _)| *m == member)
            .map(|(_, p)| p)
    }

    /// Every texture slot that names an external `.tex` file, as `(member number, path)`.
    pub fn textures(&self) -> impl Iterator<Item = (u32, &str)> {
        self.params.iter().filter_map(|(m, p)| match p {
            Param::Texture(TextureRef::External(path)) => Some((*m, path.as_str())),
            _ => None,
        })
    }

    /// The member number of the **base colour** texture slot of this config's arguments class,
    /// from the slot roles of `mtr.md`: `.10` for the shade, pixelight (poly-bump) and reflection
    /// classes, `.21` for the specular class. For the layered classes (blended layers, terrain)
    /// it is the first layer, `.10`. The refraction and water classes have no base colour map
    /// (`None`), nor does an unknown class.
    pub fn base_color_slot(&self) -> Option<u32> {
        match self.args_class.as_str() {
            "CShadeShaderArgs"
            | "CPixelightShaderArgs"
            | "CReflectionShaderArgs"
            | "CBlendShaderArgs"
            | "CTerrainShaderArgs" => Some(10),
            "CSpecularShaderArgs" => Some(21),
            _ => None,
        }
    }

    /// The `.tex` path in the base colour slot ([`ShaderConfig::base_color_slot`]), if that slot
    /// names an external texture (an empty slot or an embedded texture gives `None`).
    pub fn base_color_texture(&self) -> Option<&str> {
        match self.param(self.base_color_slot()?)? {
            Param::Texture(TextureRef::External(path)) => Some(path),
            _ => None,
        }
    }

    /// The UV map id belonging to the texture in slot `member` (the next member of the group).
    pub fn uv_map_of(&self, member: u32) -> Option<u32> {
        match self.param(member + 1)? {
            Param::UvMap(id) => Some(*id),
            _ => None,
        }
    }
}

/// The physical material (`.mat`, sound and impact behaviour) of a preset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhysicalMaterial {
    /// None.
    None,
    /// A `.mat` file (the path).
    External(String),
    /// A `CSamMaterial` object inside the file (the object id).
    Embedded(u32),
}

/// One material: a preset with its configs.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    /// The object id of the preset in its file.
    pub id: u32,
    /// The configs (0 to 3; several mean alternatives, how one is chosen is not determined).
    pub configs: Vec<ShaderConfig>,
    /// The physical material.
    pub physical: PhysicalMaterial,
}

/// A decoded `.mtr` file: its presets in object order.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialFile {
    /// All presets of the file.
    pub presets: Vec<Material>,
}

impl Material {
    /// The base colour texture of the material: that of the first config drawn on a PC (bit 0 of
    /// its platform mask) that has one ([`ShaderConfig::base_color_texture`]). How the engine
    /// chooses between several configs is not determined (`mtr.md`); first-in-file is a guess.
    pub fn base_color_texture(&self) -> Option<&str> {
        self.configs
            .iter()
            .filter(|c| c.platform_mask & 0x1 != 0)
            .find_map(ShaderConfig::base_color_texture)
    }
}

fn bad(detail: impl Into<String>) -> Error {
    Error::invalid(WHAT, detail)
}

/// The path of external object `id`, if it is one.
fn external_path(container: &Container<'_>, id: u32) -> Option<String> {
    let e = container
        .external_objects
        .iter()
        .find(|e| e.object_id == id)?;
    Some(container.files.get(e.file_ref as usize)?.path.to_string())
}

fn member_number(name: &str) -> Option<u32> {
    name.parse().ok()
}

fn param_of(container: &Container<'_>, decoded: &Decoded, value: &Value) -> Result<Param> {
    let Value::Struct(wrapper) = value else {
        return Err(bad(format!(
            "an argument is not a value wrapper: {value:?}"
        )));
    };
    let class = decoded.types.type_name(wrapper.type_id).unwrap_or("?");
    let inner = wrapper
        .values
        .first()
        .ok_or_else(|| bad(format!("the wrapper {class} has no value")))?;
    let int = |v: &Value| {
        v.as_i64()
            .ok_or_else(|| bad(format!("{class} holds {v:?}")))
    };
    Ok(match class {
        "CShaderArgValCOLOR" => {
            Param::Color(u32::try_from(int(inner)?).map_err(|_| bad("colour out of range"))?)
        }
        "CShaderArgValFLOAT" => Param::Float(match inner {
            Value::F32(f) => *f,
            other => return Err(bad(format!("{class} holds {other:?}"))),
        }),
        "CShaderArgValBOOL" => Param::Bool(int(inner)? != 0),
        "CShaderArgValUVMap" => {
            Param::UvMap(u32::try_from(int(inner)?).map_err(|_| bad("UV map id out of range"))?)
        }
        "CShaderArgValColorMap" => Param::ColorMap(int(inner)?),
        "CShaderArgValBlendType" => Param::BlendType(
            u32::try_from(int(inner)?).map_err(|_| bad("blend type out of range"))?,
        ),
        "CShaderArgValDepthCmp" => Param::DepthCmp(
            u32::try_from(int(inner)?).map_err(|_| bad("depth comparison out of range"))?,
        ),
        "CShaderArgValTexture" => match inner {
            Value::Ref(None) => Param::Texture(TextureRef::None),
            Value::Ref(Some(id)) => Param::Texture(match external_path(container, *id) {
                Some(path) => TextureRef::External(path),
                None if decoded.object(*id).is_some() => TextureRef::Embedded(*id),
                None => {
                    return Err(bad(format!(
                        "a texture slot refers to object {id}, which is neither an external file nor an object of the file"
                    )));
                }
            }),
            other => return Err(bad(format!("{class} holds {other:?}"))),
        },
        other => return Err(bad(format!("unknown argument wrapper class `{other}`"))),
    })
}

impl Material {
    /// Convert the `CShaderPreset` object `id` of `decoded` (also used for the presets embedded in
    /// `.bmf` and `.mdl` files).
    ///
    /// # Errors
    /// [`Error::Invalid`] if the object is not a preset, a shader reference is not external, the
    /// arguments object is not one of the argument classes, or an argument is not a known wrapper.
    pub fn from_preset(container: &Container<'_>, decoded: &Decoded, id: u32) -> Result<Self> {
        let object = decoded
            .object(id)
            .ok_or_else(|| bad(format!("object {id} does not exist")))?;
        let preset = object
            .value
            .as_struct()
            .filter(|s| decoded.types.type_name(s.type_id) == Some("CShaderPreset"))
            .ok_or_else(|| bad(format!("object {id} is not a CShaderPreset")))?;
        let types = &decoded.types;
        let configs_value = types
            .member(preset, "CShaderPreset", "1")
            .ok_or_else(|| bad("a preset without a config list"))?;
        let Value::List(configs) = configs_value else {
            return Err(bad("the config list is not a list"));
        };
        let mut out = Vec::with_capacity(configs.len());
        for config in configs {
            let config = config
                .as_struct()
                .ok_or_else(|| bad("a config is not a class instance"))?;
            let get = |name: &str| {
                types
                    .member(config, "CShaderConfig", name)
                    .ok_or_else(|| bad(format!("a config without member {name}")))
            };
            let shader_id = get("1")?
                .as_ref_id()
                .ok_or_else(|| bad("a config without a shader"))?;
            let shader = container
                .external_objects
                .iter()
                .find(|e| e.object_id == shader_id)
                .map(|e| ShaderClass::from_code(e.flags))
                .ok_or_else(|| {
                    bad(format!(
                        "the shader of a config (object {shader_id}) is not an external reference"
                    ))
                })?;
            let args_id = get("2")?
                .as_ref_id()
                .ok_or_else(|| bad("a config without arguments"))?;
            let platform_mask = get("3")?
                .as_i64()
                .and_then(|v| u32::try_from(v).ok())
                .ok_or_else(|| bad("bad platform mask"))?;
            out.push(Self::config(
                container,
                decoded,
                shader,
                platform_mask,
                args_id,
            )?);
        }
        let physical = match types
            .member(preset, "CShaderPreset", "2")
            .and_then(Value::as_ref_id)
        {
            None => PhysicalMaterial::None,
            Some(pid) if decoded.object(pid).is_some() => PhysicalMaterial::Embedded(pid),
            Some(pid) => external_path(container, pid)
                .map_or(PhysicalMaterial::None, PhysicalMaterial::External),
        };
        Ok(Self {
            id,
            configs: out,
            physical,
        })
    }

    fn config(
        container: &Container<'_>,
        decoded: &Decoded,
        shader: ShaderClass,
        platform_mask: u32,
        args_id: u32,
    ) -> Result<ShaderConfig> {
        let object = decoded
            .object(args_id)
            .ok_or_else(|| bad(format!("the arguments object {args_id} does not exist")))?;
        let args: &Struct = object
            .value
            .as_struct()
            .ok_or_else(|| bad("the arguments are not a class instance"))?;
        let args_class = decoded
            .types
            .type_name(args.type_id)
            .unwrap_or("?")
            .to_string();
        if !args_class.ends_with("ShaderArgs") {
            return Err(bad(format!("`{args_class}` is not an arguments class")));
        }
        let layout = decoded
            .types
            .members(args.type_id)
            .ok_or_else(|| bad("the arguments class has no layout"))?;
        let (mut common, mut params, mut modifiers) = (Vec::new(), Vec::new(), Vec::new());
        for (member, value) in layout.iter().zip(&args.values) {
            match member.owner_name.as_str() {
                "CShaderArgs" => {
                    // `CShaderModifierList.1`: a list of references to modifier objects.
                    if let Some(list) = value.as_struct().and_then(|s| s.values.first())
                        && let Value::List(items) = list
                    {
                        for item in items {
                            if let Some(id) = item.as_ref_id() {
                                let class = decoded
                                    .object(id)
                                    .and_then(|o| decoded.types.type_name(o.type_id))
                                    .unwrap_or("?")
                                    .to_string();
                                modifiers.push(Modifier { class });
                            }
                        }
                    }
                }
                "CShaderCommonParams" => {
                    let n = member_number(&member.name)
                        .ok_or_else(|| bad("a common member without a number"))?;
                    common.push((n, param_of(container, decoded, value)?));
                }
                owner if owner == args_class => {
                    let n = member_number(&member.name)
                        .ok_or_else(|| bad("an argument member without a number"))?;
                    params.push((n, param_of(container, decoded, value)?));
                }
                other => {
                    return Err(bad(format!(
                        "unexpected base class `{other}` in `{args_class}`"
                    )));
                }
            }
        }
        Ok(ShaderConfig {
            shader,
            platform_mask,
            args_class,
            common,
            params,
            modifiers,
        })
    }
}

impl MaterialFile {
    /// Read a `.mtr` file.
    ///
    /// # Errors
    /// [`Error::Invalid`] naming what is wrong (see [`Material::from_preset`]).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let container = Container::parse(bytes)?;
        if container.header.format_version != 8 {
            return Err(bad(format!(
                "format version {}",
                container.header.format_version
            )));
        }
        let decoded = container.decode_objects()?;
        let mut presets = Vec::new();
        for o in &decoded.objects {
            if decoded.types.type_name(o.type_id) == Some("CShaderPreset") {
                presets.push(Material::from_preset(&container, &decoded, o.id)?);
            }
        }
        Ok(Self { presets })
    }
}

/// The `.mtr` loader: register it with `AssetManager::register_loader` to load [`MaterialFile`]s.
/// The textures of a material are returned as paths; request them through the asset manager.
#[derive(Clone, Copy, Debug, Default)]
pub struct MtrLoader;

impl AssetLoader for MtrLoader {
    type Asset = MaterialFile;

    fn extensions(&self) -> &[&str] {
        &["mtr"]
    }

    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<MaterialFile> {
        MaterialFile::from_bytes(&ctx.read()?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ss2_assets::{AssetManager, LoadState};
    use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};
    use ss2_vfs::MemoryVfs;

    use super::*;

    /// A file with one preset: one specular config whose arguments hold a colour, a texture slot
    /// with its UV map, a float, the common blend type, and one modifier.
    fn material_file(texture_external: bool, shader_flags: u32) -> Vec<u8> {
        let mut b = MetaBuilder::new();
        let module = b.file_ref("Bin/Shaders.module", 0);
        let tex = b.file_ref("Content/Test/Textures/body.tex", 0);
        b.external_type(100, "CShader");
        b.external_type(101, "CStaticTexture");
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        b.type_block(TypeBlock::alias(1, "INDEX", 0));
        b.type_block(TypeBlock::primitive(2, "ULONG", 4, 4));
        b.type_block(TypeBlock::primitive(3, "FLOAT", 4, 4));
        b.type_block(TypeBlock::enumeration(4, "EPlatform", 4));
        b.type_block(TypeBlock::enumeration(5, "GfuBlendType", 4));
        b.type_block(TypeBlock::class(
            6,
            "CResourceFile",
            3,
            5,
            None,
            &[("1", 1)],
        ));
        b.type_block(TypeBlock::pointer(7, "CResourceFile*", 6));
        b.type_block(TypeBlock::class(
            8,
            "CResource",
            4,
            5,
            None,
            &[("1", 1), ("2", 7)],
        ));
        // Wrappers.
        b.type_block(TypeBlock::pointer(9, "CShader*", 100));
        b.type_block(TypeBlock::pointer(10, "CStaticTexture*", 101));
        b.type_block(TypeBlock::class(
            11,
            "CShaderArgValCOLOR",
            1,
            5,
            None,
            &[("1", 2)],
        ));
        b.type_block(TypeBlock::class(
            12,
            "CShaderArgValFLOAT",
            1,
            5,
            None,
            &[("1", 3)],
        ));
        b.type_block(TypeBlock::class(
            13,
            "CShaderArgValTexture",
            1,
            5,
            None,
            &[("1", 10)],
        ));
        b.type_block(TypeBlock::class(
            14,
            "CShaderArgValUVMap",
            1,
            5,
            None,
            &[("1", 2)],
        ));
        b.type_block(TypeBlock::class(
            15,
            "CShaderArgValBlendType",
            1,
            5,
            None,
            &[("1", 5)],
        ));
        // Modifier list and the argument classes.
        b.type_block(TypeBlock::class(
            16,
            "CShaderModifier",
            1,
            5,
            None,
            &[("1", 4)],
        ));
        b.type_block(TypeBlock::class(
            17,
            "CConstantShaderModifier",
            1,
            5,
            Some(16),
            &[],
        ));
        b.type_block(TypeBlock::pointer(18, "CShaderModifier*", 16));
        b.type_block(TypeBlock::short(
            19,
            "CDynamicContainer<CShaderModifier>",
            0,
            8,
            16,
        ));
        b.type_block(TypeBlock::class(
            20,
            "CShaderModifierList",
            1,
            5,
            None,
            &[("1", 19)],
        ));
        b.type_block(TypeBlock::class(
            21,
            "CShaderArgs",
            1,
            5,
            None,
            &[("1", 20)],
        ));
        b.type_block(TypeBlock::class(
            22,
            "CShaderCommonParams",
            1,
            5,
            Some(21),
            &[("1", 15)],
        ));
        b.type_block(TypeBlock::class(
            23,
            "CSpecularShaderArgs",
            1,
            5,
            Some(22),
            &[("9", 11), ("21", 13), ("22", 14), ("8", 12)],
        ));
        b.type_block(TypeBlock::pointer(24, "CSpecularShaderArgs*", 23));
        b.type_block(TypeBlock::class(
            25,
            "CShaderConfig",
            1,
            5,
            None,
            &[("1", 9), ("2", 24), ("3", 4)],
        ));
        b.type_block(TypeBlock::short(
            26,
            "CStaticArray<CShaderConfig>",
            0,
            6,
            25,
        ));
        b.type_block(TypeBlock::pointer(27, "CResource*", 8));
        b.type_block(TypeBlock::class(
            28,
            "CShaderPreset",
            1,
            5,
            Some(8),
            &[("1", 26), ("2", 27), ("3", 0)],
        ));
        b.external_object(2, module, shader_flags, 100);
        b.external_object(4, tex, 1, 101);

        b.object(0, 6, ObjectBytes::new().i32(1).finish());
        b.object(
            1,
            28,
            ObjectBytes::new()
                .i32(1)
                .object_ref(Some(0))
                .array_header(b"STAR", 1)
                .object_ref(Some(2))
                .object_ref(Some(3))
                .u32(0x3)
                .object_ref(None)
                .i32(255)
                .finish(),
        );
        // Object 3: the arguments (base members first: CShaderArgs, CShaderCommonParams, own).
        let texture_ref = if texture_external { Some(4) } else { Some(9) };
        b.object(
            3,
            23,
            ObjectBytes::new()
                .raw(&ObjectBytes::new().dynamic_container(&[5]).finish())
                .u32(0x1F4)
                .u32(0xFF7F7F7F)
                .object_ref(texture_ref)
                .u32(1)
                .f32(30.0)
                .finish(),
        );
        b.object(5, 17, ObjectBytes::new().u32(0x3).finish());
        b.build().unwrap()
    }

    #[test]
    fn reads_a_preset_with_its_config_and_arguments() {
        let file = MaterialFile::from_bytes(&material_file(true, 1010)).unwrap();
        assert_eq!(file.presets.len(), 1);
        let m = &file.presets[0];
        assert_eq!(
            (m.id, m.configs.len(), &m.physical),
            (1, 1, &PhysicalMaterial::None)
        );
        let c = &m.configs[0];
        assert_eq!(
            (c.shader, c.platform_mask, c.args_class.as_str()),
            (ShaderClass::Specular, 3, "CSpecularShaderArgs")
        );
        assert_eq!(c.common_param(1), Some(&Param::BlendType(0x1F4)));
        assert_eq!(c.param(9), Some(&Param::Color(0xFF7F7F7F)));
        assert_eq!(c.param(8), Some(&Param::Float(30.0)));
        assert_eq!(
            c.textures().collect::<Vec<_>>(),
            [(21, "Content/Test/Textures/body.tex")]
        );
        assert_eq!(c.uv_map_of(21), Some(1));
        assert_eq!(
            c.modifiers,
            [Modifier {
                class: "CConstantShaderModifier".into()
            }]
        );
    }

    #[test]
    fn the_base_colour_slot_depends_on_the_arguments_class() {
        // The fixture's specular config holds its texture in `.21`, the specular base colour map.
        let file = MaterialFile::from_bytes(&material_file(true, 1010)).unwrap();
        let m = &file.presets[0];
        assert_eq!(m.configs[0].base_color_slot(), Some(21));
        assert_eq!(
            m.base_color_texture(),
            Some("Content/Test/Textures/body.tex")
        );
        let config = |class: &str, member: u32| ShaderConfig {
            shader: ShaderClass::Shade,
            platform_mask: 0x3,
            args_class: class.into(),
            common: vec![],
            params: vec![(
                member,
                Param::Texture(TextureRef::External(format!("{class}.tex"))),
            )],
            modifiers: vec![],
        };
        for (class, slot) in [
            ("CShadeShaderArgs", Some(10)),
            ("CSpecularShaderArgs", Some(21)),
            ("CPixelightShaderArgs", Some(10)),
            ("CReflectionShaderArgs", Some(10)),
            ("CBlendShaderArgs", Some(10)),
            ("CTerrainShaderArgs", Some(10)),
            ("CRefractionShaderArgs", None),
            ("CWaterShaderArgs", None),
        ] {
            let c = config(class, slot.unwrap_or(10));
            assert_eq!(c.base_color_slot(), slot, "{class}");
            assert_eq!(c.base_color_texture().is_some(), slot.is_some(), "{class}");
        }
        // A slot that is empty or holds something else gives no texture.
        let mut empty = config("CShadeShaderArgs", 10);
        empty.params[0].1 = Param::Texture(TextureRef::None);
        assert_eq!(empty.base_color_texture(), None);
        // The material takes the first PC config that has one; an Xbox-only config is skipped.
        let mut xbox = config("CShadeShaderArgs", 10);
        xbox.platform_mask = 0x2;
        let material = Material {
            id: 1,
            configs: vec![xbox, empty, config("CPixelightShaderArgs", 10)],
            physical: PhysicalMaterial::None,
        };
        assert_eq!(
            material.base_color_texture(),
            Some("CPixelightShaderArgs.tex")
        );
    }

    #[test]
    fn shader_codes_round_trip() {
        for code in [1000, 1010, 1020, 1025, 1030, 1040, 1050, 1070] {
            assert_eq!(ShaderClass::from_code(code).code(), code);
        }
        assert_eq!(ShaderClass::from_code(1234), ShaderClass::Other(1234));
        let file = MaterialFile::from_bytes(&material_file(true, 1050)).unwrap();
        assert_eq!(file.presets[0].configs[0].shader, ShaderClass::PolyBump);
    }

    #[test]
    fn a_texture_slot_can_be_embedded_but_not_dangling() {
        // Object 9 does not exist: neither external nor an object of the file.
        let err = MaterialFile::from_bytes(&material_file(false, 1010))
            .unwrap_err()
            .to_string();
        assert!(err.contains("neither an external file"), "{err}");
        assert!(MaterialFile::from_bytes(b"junk").is_err());
    }

    #[test]
    fn nothing_panics_on_damaged_files() {
        let good = material_file(true, 1010);
        for len in (0..good.len()).step_by(5) {
            let _ = MaterialFile::from_bytes(&good[..len]);
        }
        for i in (0..good.len()).step_by(3) {
            let mut bad = good.clone();
            bad[i] ^= 0x5A;
            let _ = MaterialFile::from_bytes(&bad);
        }
    }

    #[test]
    fn the_loader_plugs_into_the_asset_manager() {
        let file = material_file(true, 1000);
        let vfs = MemoryVfs::from_files([("m/a.mtr", file.as_slice())]).unwrap();
        let assets = AssetManager::new(Arc::new(vfs));
        assets.register_loader(MtrLoader).unwrap();
        let h = assets.load::<MaterialFile>("M/A.MTR").unwrap();
        assert_eq!(h.wait(), LoadState::Ready);
        assert_eq!(
            h.get().unwrap().presets[0].configs[0].shader,
            ShaderClass::Shade
        );
    }

    /// Data-gated: every `.mtr` of the retail install loads, and so does **every shader preset of
    /// every model, mesh and material file**: `mtr.md` counts 7,482 presets and 8,009 configs over
    /// all containers (the `.mtr` files hold one preset each, the rest are embedded in `.mdl` and
    /// `.bmf` files). Skips without `SS2_DATA_DIR`.
    #[test]
    fn every_preset_of_the_real_install_loads() {
        use ss2_vfs::{EntryKind, Vfs, VfsPath};
        use std::collections::BTreeMap;
        let dir = ss2_testkit::require_game_data!();
        let vfs = ss2_vfs::mount_install(&dir).unwrap();
        let mut stack = vec![VfsPath::root()];
        let (mut mtr_files, mut presets, mut configs, mut textures) =
            (0usize, 0usize, 0usize, 0usize);
        let mut shaders: BTreeMap<String, usize> = BTreeMap::new();
        let mut failures = Vec::new();
        while let Some(d) = stack.pop() {
            for e in vfs.list(&d).unwrap() {
                let path = d.join(&e.name).unwrap();
                if e.kind == EntryKind::Dir {
                    stack.push(path);
                    continue;
                }
                if !matches!(path.extension(), Some("mtr" | "mdl" | "bmf")) {
                    continue;
                }
                let bytes = vfs.read(&path).unwrap();
                let result = Container::parse(&bytes).and_then(|c| {
                    let decoded = c.decode_objects()?;
                    let ids: Vec<u32> = decoded
                        .objects
                        .iter()
                        .filter(|o| decoded.types.type_name(o.type_id) == Some("CShaderPreset"))
                        .map(|o| o.id)
                        .collect();
                    ids.into_iter()
                        .map(|id| Material::from_preset(&c, &decoded, id))
                        .collect::<Result<Vec<_>>>()
                });
                match result {
                    Ok(list) => {
                        mtr_files += usize::from(path.extension() == Some("mtr"));
                        presets += list.len();
                        for m in &list {
                            for c in &m.configs {
                                configs += 1;
                                textures += c.textures().count();
                                *shaders
                                    .entry(format!("{:?}/{}", c.shader, c.args_class))
                                    .or_default() += 1;
                            }
                        }
                    }
                    Err(e) => failures.push(format!("{}: {e}", path.as_str())),
                }
            }
        }
        eprintln!(
            "{mtr_files} .mtr files; {presets} presets, {configs} configs, {textures} texture slots; shaders {shaders:?}; {} failures",
            failures.len()
        );
        assert!(
            failures.is_empty(),
            "first failures: {:?}",
            &failures[..failures.len().min(40)]
        );
        assert_eq!(mtr_files, 91);
        assert_eq!(configs, shaders.values().sum::<usize>());
        assert_eq!((presets, configs), (7482, 8009), "the counts of mtr.md");
    }
}
