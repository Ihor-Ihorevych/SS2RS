# `.mtr` materials (`CShaderPreset`)

- **Roadmap task:** T01-011
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the object structure, the shader class codes, the member layout of all eight argument classes and the link to mesh UV maps; **medium** for the roles of the texture slots (inferred from the referenced textures, see "Evidence"); **low** for the meaning of the blend types, the common flags and the modifier members. The shader programs themselves are **not** in the data (see "Shader module"). Scope: all **91** distinct format-8 `.mtr` files, plus the **7,391** `CShaderPreset` objects embedded in `.bmf` (260) and `.mdl` (7,131) files, which use the same classes.

Prerequisites: `ctsemeta-container.md`, `ctsemeta-types.md`, `ctsemeta-objects.md`; `bmf.md` (surfaces), `mdl.md`, `tex.md`.

## Summary

A **material** is a `CShaderPreset`. It holds a short list of **configs**; each config pairs a **shader** (an external reference to a shader class of the engine's shader module, never data in the file) with an **arguments object** that holds all parameters: common render state, colours, up to eight texture slots with UV map id and transform numbers, and optional **modifiers** that animate parameters. A mesh surface refers to its preset (`CRenderMeshSurface.16`, `bmf.md`), either as an object embedded in the same file (258 of 353 surfaces in `.bmf`) or as an external `.mtr` file (the `.mtr` files hold exactly one preset). A preset also names a **physical material** (`.mat`) used by gameplay (impact effects).

## Evidence

`tools/re/mtr_verify.py` (read-only, `SS2_DATA_DIR`), over the retail install with all patches:

- All 91 `.mtr` files decode (classes `CResourceFile`, `CShaderPreset`, the argument classes, modifiers and graphs). Over all files: **7,482** presets, **8,009** configs.
- **All 8,009** shader references are external references to `Bin/Shaders.module`, and the `EXOB` flag word of each is determined by the argument class (table below): no exception.
- Texture slots were identified by what the referenced textures are: names and formats of the textures each slot refers to (see "Texture slots"). For example all 353 textures in the reflection class's slot 2 are DXT1 cube maps (`0x23` flags), which independently confirms the cube map flag of `tex.md`.
- **6,134 of 6,169** shade configs on surfaces use only UV map ids that occur in the surface's UV map list (`bmf.md`, surface member 12).

## Objects

```
CShaderPreset
  CResource.{1,5,3,2}   as for the other resources
  .1   list of CShaderConfig       0 (5 presets), 1 (6,955), 2 (512), 3 (10)
  .2   -> physical material        external .mat (class CSamMaterial) in 5,274; embedded CSamMaterial in 1; null in 2,207
  .3   int                         255 in all 7,482
CShaderConfig
  .1   -> shader                   external object of Bin/Shaders.module (EXOB flags = shader class code)
  .2   -> arguments object         one of the eight argument classes
  .3   EPlatform                   0x3 (6,750 configs), 0xFFFFFFFF (1,257), 0x1 (2)
```

A preset with several configs presumably offers alternatives per platform or quality (the masks differ); how the engine chooses is **not determined**. `EPlatform` bit meanings are unknown (see `bmf.md`).

### Shader reference and class codes

The shader of a config is an external object. In the file's `EXOB` table (`ctsemeta-container.md`) its entry has a **flag word** that names the **shader class**, and its type id resolves through `EXTY` to the class name when the file knows it (`CShader` otherwise). The code is fixed by the argument class that goes with it:

| EXOB flags | Shader class (where named in `EXTY`) | Arguments class | Configs |
|---|---|---|---|
| 1000 | `CShadeShader` | `CShadeShaderArgs` | 6,213 |
| 1010 | `CSpecularShader` | `CSpecularShaderArgs` | 787 |
| 1020 | `CReflectionShader` | `CReflectionShaderArgs` | 362 |
| 1025 | `CRefractionShader` | `CRefractionShaderArgs` | 38 |
| 1030 | `CBlendLayersShader` | `CBlendShaderArgs` | 29 |
| 1040 | `CTerrainShader` | `CTerrainShaderArgs` | 289 |
| 1050 | `CPolyBumpShader` | `CPixelightShaderArgs` | 239 |
| 1070 | `CWaterShader` | `CWaterShaderArgs` | 52 |

(The "Pixelight" arguments go with the shader named `CPolyBumpShader`; a normal-mapped lit material.) A reader needs only the **argument class** to know the layout; the code gives the shader class an engine must implement.

### Shader module

`Bin/Shaders.module` is listed in `RFIL` of every material (flags aside), but there is **no such file**: the install has `Bin/Shaders.dll` and `Bin/GPU-Programs.asm`. The module is a native library; the shaders are its classes, and the GPU programs are outside the data format. How the classes pick programs and what they compute is the subject of T01-020.

## Argument objects

### Common layout

Every arguments object starts with the same members. Each parameter is a **wrapper object** with exactly one member `.1` that holds the value; the wrapper class says the type.

| Wrapper class | Value | Used for |
|---|---|---|
| `CShaderArgValCOLOR` | 32-bit unsigned colour, **alpha in the top byte** (the order of the other three bytes is not confirmed); default mid-grey `0xFF7F7F7F` | tint colours |
| `CShaderArgValFLOAT` | float | scales, offsets, strengths |
| `CShaderArgValBOOL` | 0 or 1 | switches |
| `CShaderArgValTexture` | reference to an external `.tex`, or null | texture slots |
| `CShaderArgValUVMap` | small int | **UV map id** (see below) |
| `CShaderArgValColorMap` | int (0 in all 6,213 shade configs) | unknown |
| `CShaderArgValBlendType` | `GfuBlendType` enum (4 bytes) | blend mode |
| `CShaderArgValDepthCmp` | `GfxComp` enum (4 bytes) | depth comparison |

Members of the base classes (the same in all eight argument classes):

| Member | Meaning (observed) |
|---|---|
| `CShaderArgs.1` | `CShaderModifierList` (list of modifiers, see below): empty in 5,722 of 6,213 shade configs |
| `CShaderCommonParams.1` | blend type: `0x1F4` (5,679 of 6,213 shade configs), `0x1F5`, `0x1F7`, `0x1F8`, rarely `0x1F6`, `0x1F9`, `0x1FA` |
| `CShaderCommonParams.2` | depth comparison: `0x2A`, rarely `0x29` (17 configs in the shade, specular and reflection classes) |
| `CShaderCommonParams.5` | bool |
| `CShaderCommonParams.6` | float: 0.5 in 6,156 of 6,213 shade configs |
| `CShaderCommonParams.7`, `.8` | bools |

Meaning by evidence (all **low** confidence): blend `0x1F4` goes with textures without alpha channel in 4,562 of 5,679 shade configs, `0x1F5` and `0x1F8` with alpha-carrying textures (167 of 190, and 72 of 98), `0x1F7` with alpha-less ones (201 of 216): `0x1F4` looks like opaque (or alpha-tested), the others like blended modes. `CommonParams.5` is 1 for 769 of 907 alpha-carrying first textures under blend `0x1F4` but for only 88 of 4,562 alpha-less ones, so it is probably an **alpha test** switch with `.6` (0.5) as its reference. These readings are not confirmed by the shader code.

### Texture slot pattern

Texture slots come in **groups**; in a group the first member is the texture and the next ones belong to it:

```
slot n   : texture   (CShaderArgValTexture)
slot n+1 : UV map id (CShaderArgValUVMap)
slot n+2, n+3 : FLOAT, FLOAT   (1.0 in most; also 0.5, 2, 3, 5, 10, 100 ...)  likely scale u, v
slot n+4, n+5 : FLOAT, FLOAT   (0.0 in most; small +- numbers)                  likely offset u, v
slot n+7 or n+8: FLOAT (0, 90, 180, 30, -90)                                    likely rotation in degrees (n+8 in the terrain and specular classes)
```

The scale/offset/rotation roles come from the value ranges only. Groups start at `.10`, `.20`, `.30`, ... (shade, specular, pixelight, reflection, terrain) in steps of 10.

### `CShadeShaderArgs` (6,213 configs; the standard material)

| Member | Value | Notes |
|---|---|---|
| `.1` | COLOR | tint; `0xFF7F7F7F` in 3,646 |
| `.2` | ColorMap | always 0 |
| `.5` | FLOAT | 0.0 in 6,116, 1.0 in 96, 0.6 in 1 |
| `.6` | COLOR | `0x00FFFFFF` in 6,004 (white, alpha 0) |
| `.10` … `.15`, `.17` | group 1: texture, UV map, scale ×2, offset ×2, rotation | texture non-null in 6,096: the **base (diffuse) map** |
| `.20` … `.25` | group 2 | non-null in 3,550: second layer (names `concrete`, `wood`, light/shadow maps) |
| `.30` … `.35` | group 3 | non-null in 1,142 |
| `.40` … `.45` | group 4 | non-null in 169: names `glow`, `noise`, `neon` (self-illumination / detail) |
| `.50`, `.51` | FLOAT | 10000.0 in 6,209: probably a distance pair; unknown |

### The other classes (members and roles)

| Class | Texture groups and other members | Slot roles established |
|---|---|---|
| `CSpecularShaderArgs` (787) | `.1` float, `.8` float (10 to 120: probably a shininess), `.9` colour; groups `.10`, `.20` (with colour `.20` first, group members `.21`…`.26`, `.28`), `.30` | `.21` = base colour map (names `base`, `ingame`, wall/metal), `.10` = gloss/specular map (300 of 431 names end in `_g`, 301 are DXT5), `.30` = illumination (`il`, `metal`) |
| `CPixelightShaderArgs` (239) | `.1` colour, `.5` float, groups `.10`, `.20`, `.30`; `.40` colour, `.42` float (10 to 80) | `.10` = base map, **`.20` = normal map** (178 named `normalmap`) |
| `CReflectionShaderArgs` (362) | `.1` colour, **`.2` = texture**, `.5`, `.6` floats (`.6` = 90.0 in 356), groups `.10`, `.20`, `.30` (always null) | **`.2` = environment cube map** (all 353 are cube textures), `.10` base map, **`.20` = normal map** |
| `CRefractionShaderArgs` (38) | `.1` colour, `.3`, `.10` floats, `.11` texture, `.12` UV map, `.13`–`.16` floats, group `.20`, `.30` BOOL | `.11` = normal / bump map (names `nm`, `bump`) |
| `CTerrainShaderArgs` (289) | **`.1` texture** with `.2` UV map and `.3`–`.6` floats; layer groups `.10`, `.20`, `.30`, `.40`, `.50`, `.60`, `.70`, each with a COLOR (`.16`, `.26`, `.36`, `.46`, `.56`) in place of the rotation float | `.1` = **blend mask** (names `mask`, 255 of 289 have flags `0x20`, DXT3 or 32-bit); the layers are ground / rock / stone / concrete maps |
| `CBlendShaderArgs` (29) | `.1` colour, `.2` ColorMap; three or four groups `.10`, `.20`, `.30`, `.40` with two BOOLs after the UV map | blend of 3–4 layers (`rock`, `ground`, `drip`) |
| `CWaterShaderArgs` (52) | `.1` colour, **`.2` texture** (names `waterreflection`, `cavereflection`), `.3`, `.6`, `.7`, `.8`, groups `.10`, `.20` (`01nm`, `bump`), many wave floats `.22`…`.63` | `.2` = reflection image (not a cube map), `.20` = normal map |

Complete member lists are in `tools/re/mtr_verify.py` output and in the type table of any file that holds such an object (the member numbers are stable across files).

### Texture slots (evidence)

Names are the last `_` token of the file name without digits; formats are `.tex` format codes (`tex.md`): 7 = DXT1, 8 = DXT3, 9 = DXT5, 0/2/6 = 24-bit, 1/3/4/5/11 = 32-bit.

| Slot | Referenced textures | Reading |
|---|---|---|
| Shade `.10` (6,096) | formats DXT1 4,613 / DXT5 845 / DXT3 230 | base map |
| Pixelight `.20` (229), Reflection `.20` (159), Refraction `.11` (38), Water `.20` (42) | formats 24-bit (0, 2) and 32-bit (1) for 381 of 468; names `normalmap`, `nm`, `01nm`, `bump` | normal maps |
| Reflection `.2` (353) | DXT1, flags `0x23` in 353 of 353 | cube map |
| Terrain `.1` (289) | DXT3 196, 32-bit 63; name `mask` | blend mask |
| Specular `.10` (431) | DXT5 301; name `g` for 300 | gloss / specular map |

### UV map ids

The `CShaderArgValUVMap` value is a **UV map id**, matched against the per-slot UV map ids of the mesh surface (`bmf.md`, surface members 12 and 14): in 6,134 of 6,169 shade configs on surfaces, every UV id used by a non-null texture is in the surface's list (for example surface list `[3, 4, 5, 6]`, material using 3 and 4). Typical ids are 3, 4 and 5; 0 for unused slots. The 35 exceptions are not analysed.

## Modifiers

`CShaderArgs.1` is a `CShaderModifierList` with one member, a list of modifier objects (references). The modifiers **animate a parameter**:

| Class | Members | Count |
|---|---|---|
| `CConstantShaderModifier` | `CShaderModifier.1` = `EPlatform`; `.1`, `.2` ints (target); `.3` → a value wrapper (for example a colour) | 685 |
| `CFloatGraphShaderModifier` | `EPlatform`; `.1` (target id: 0 in 626, else 4, 20, 30, 56), `.2` int, `.3` → a **graph**, `.4` float (3.0 in the sample), `.5` a value wrapper | 644 |
| `CGradientShaderModifier` | `EPlatform`; `.1` (0 in 143), `.2`, `.3` a list of (position, ARGB colour) keys, `.4`, `.5` a value wrapper | 150 |

Graph classes seen: `CSineGraph`, `CStepUpGraph`, `CStepDownGraph`, `CStepGraph`, `CLinearGraph`, `CValueGraph`, `CRandomGraph`; each has base members `CGraph.7`…`.12` and a key list of (time, value) pairs (the pair members hold floats; a period-like value `360` appears in the sine graph). **What the target ids select and the exact graph semantics are not determined** (T01-059).

## Reader invariants

1. A preset is valid only if every config's shader reference is an external reference (or the file fails loudly) and its arguments object is one of the eight classes.
2. Resolve texture slots through `EXOB`/`RFIL`: a non-null texture value is an external `.tex`; fail on an id that is neither external nor an object.
3. Treat `CShaderPreset.2` as optional (null, external `.mat`, or embedded).
4. Use the UV map id to choose the mesh's UV stream by id (`bmf.md`), not the slot position.
5. Unknown wrapper or graph classes must fail the file, not be guessed.

## Test vector

`Content/SeriousSam2/Models/Enemies/Tank/Sources/ShaderPresets/TankGeneral_Skin_Lod01.mtr` (2,740 bytes): `RFIL` lists `Bin/Shaders.module` and `Content/SeriousSam2/Models/Enemies/Tank/Textures/Body.tex`. `EXOB`: object 2 (module, flags **1010**, type id 13) and object 4 (the texture). One `CShaderPreset`: `.1` has one config: shader → object 2, arguments → object 3 (`CSpecularShaderArgs`), platform `0x3`; `.2` null; `.3` 255. Arguments: blend `0x1F4`, depth compare `0x2A`, common `.5` = 0, `.6` = 0.5, `.7` = `.8` = 0, no modifiers; `.1` = 0.5; `.8` = 30.0; `.9` = `0xFF7F7F7F`; group `.10` with a null texture, UV map 0; colour `.20` = 4286277696 (`0xFF7B6840`); group `.21` with the texture of object 4 (`Body.tex`), UV map **1**, scales 1.0, offsets 0.0; `.28` = 0; group `.30` with a null texture.

## Open questions

- What the shader classes compute and how a class picks a program (T01-020); the meaning of the blend types, the common bools, `ColorMap`, the unexplained floats (`.50`/`.51`, `.5`, per-class numbers) → **T01-059**.
- Whether presets with several configs choose by platform or by quality; the bits of `EPlatform`.
- Modifier targets and graph semantics → T01-059.
- Colour byte order (the top byte is alpha).
- What the engine draws for surfaces without a preset (88 in `.bmf`, see `bmf.md`).
