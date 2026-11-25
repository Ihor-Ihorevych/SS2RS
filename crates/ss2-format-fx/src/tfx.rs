//! `.tfx` text effect configurations (spec `docs/re/fx.md`).
//!
//! A [`TextEffectConfiguration`] (class `CTextEffectConfiguration`) is an animated on-screen text
//! style (HUD messages, conversation subtitles): a font, a placement on the 640 x 480 virtual
//! screen, timing and two lists of effects whose parameters are driven by graphs. The spec's
//! confidence for these members is **low**: they are returned with their numbers and the observed
//! reading, and nothing is evaluated. The two graph pairs of an effect are returned as stored
//! (`graphs[0..4]` are `.1` to `.4`).

use ss2_core::Result;

use crate::graph::Graph;
use crate::read::{Ctx, Obj};

const WHAT: &str = ".tfx text effect";

/// A `CFontInstance`: a font reference with colours and size members.
#[derive(Clone, Debug, PartialEq)]
pub struct FontInstance {
    /// `.1`: the `.fnt` file (`SeriousZurich_HD.fnt`, `SeriousVerdana_HD.fnt`).
    pub font: Option<String>,
    /// `.2`: the colour (ARGB).
    pub color: u32,
    /// `.3` `FontFlags` (0 or 1).
    pub flags: u32,
    /// `.4` to `.7`: floats (0 or 1, 0, 0.175 to 0.3, 0.75).
    pub members_4_to_7: [f32; 4],
    /// `.8` (`PIX`): a size in pixels (45 to 106).
    pub pixel_size: i32,
    /// `.9`: a float (1.0 in the retail files).
    pub member_9: f32,
    /// `.10`: the shadow colour (0 when none).
    pub shadow_color: u32,
    /// `.11` to `.14`: floats.
    pub members_11_to_14: [f32; 4],
    /// `.16`.
    pub member_16: i32,
    /// `.15`: an identifier (the text `empty` in the retail files).
    pub member_15: String,
}

/// The four graphs and the enable flag common to both effect bases (`CTextEffect`,
/// `CLetterEffect`). By the observed patterns the pair `.1`/`.2` shapes the appearing phase and
/// `.3`/`.4` the disappearing phase (not proven).
#[derive(Clone, Debug, PartialEq)]
pub struct EffectGraphs {
    /// The enable flag (`CTextEffect.5`, `CLetterEffect.6`; 1 in all retail effects).
    pub enabled: bool,
    /// `.1` to `.4`: the graphs (`None` for a null pointer; never null in the retail files).
    pub graphs: [Option<Graph>; 4],
}

/// A whole-text effect (`CTextEffect` and its derived classes).
#[derive(Clone, Debug, PartialEq)]
pub struct TextEffect {
    /// The graphs of the base class.
    pub base: EffectGraphs,
    /// The class-specific part.
    pub kind: TextEffectKind,
}

/// The whole-text effect classes seen in the retail files.
#[derive(Clone, Debug, PartialEq)]
pub enum TextEffectKind {
    /// `CBlendTextEffect`: the two colours it blends between (transparent and opaque white).
    Blend {
        /// `.1`.
        from: u32,
        /// `.2`.
        to: u32,
    },
    /// `CStretchTextEffect`: four floats (start and end scales).
    Stretch {
        /// `.1` to `.4`.
        values: [f32; 4],
    },
    /// `CMoveTextEffect`: an offset.
    Move {
        /// `.1`: the offset on the virtual screen.
        offset: [f32; 2],
    },
}

/// A per-letter effect (`CLetterEffect` and its derived classes).
#[derive(Clone, Debug, PartialEq)]
pub struct LetterEffect {
    /// The graphs of the base class.
    pub base: EffectGraphs,
    /// `CLetterEffect.5`: the delay between letters (0, 0.01 or 0.02).
    pub letter_delay: f32,
    /// The class-specific part.
    pub kind: LetterEffectKind,
}

/// The per-letter effect classes seen in the retail files.
#[derive(Clone, Debug, PartialEq)]
pub enum LetterEffectKind {
    /// `CSizeLetterTextEffect`: four floats.
    Size {
        /// `.1` to `.4`.
        values: [f32; 4],
    },
    /// `CScreenOffsetAppearLetterTextEffect`.
    ScreenOffsetAppear {
        /// `.1`: an index (garbage-looking in the retail file; kept as stored).
        member_1: i32,
        /// `.2` `TextSide` (4 in the retail file).
        side: u32,
    },
}

/// A decoded `.tfx` file.
#[derive(Clone, Debug, PartialEq)]
pub struct TextEffectConfiguration {
    /// `.1`: whole-text effects (0 to 3).
    pub effects: Vec<TextEffect>,
    /// `.2`: per-letter effects (0 to 2).
    pub letter_effects: Vec<LetterEffect>,
    /// `.3`, `.4`, `.5`: look like time in, time held and time out in seconds (100 = until
    /// removed). Low confidence.
    pub timing: [f32; 3],
    /// `.6`: position on the 640 x 480 virtual screen.
    pub position: [f32; 2],
    /// `.14`: size of the text box.
    pub box_size: [f32; 2],
    /// `.7`: the font.
    pub font: FontInstance,
    /// `.8` `TextAlignmentH` (0 probably left, 1 probably centred).
    pub alignment_h: u32,
    /// `.9` `TextAlignmentV` (1 in all retail files).
    pub alignment_v: u32,
    /// `.10` `GfuBlendType` (501 in all retail files).
    pub blend: u32,
    /// `.12`.
    pub member_12: bool,
    /// `.13`.
    pub member_13: f32,
    /// `.15`.
    pub member_15: bool,
    /// `.20`: looks like a background box colour (alpha `0x78` for subtitles).
    pub background_color: u32,
}

impl TextEffectConfiguration {
    /// Read a `.tfx` file.
    ///
    /// # Errors
    /// [`ss2_core::Error::Invalid`] naming what is wrong (also an effect or graph of a class the
    /// spec does not list).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let (container, decoded) = Ctx::decode(bytes, WHAT)?;
        let ctx = Ctx {
            container: &container,
            decoded: &decoded,
            what: WHAT,
        };
        let root = ctx.root("CTextEffectConfiguration")?;
        const C: &str = "CTextEffectConfiguration";
        let pair = |a: &str| -> Result<[f32; 2]> {
            let v = root.sub(C, a)?;
            Ok([v.get("Vector2f", "1")?, v.get("Vector2f", "2")?])
        };
        Ok(Self {
            effects: root
                .objects(C, "1")?
                .iter()
                .map(read_text_effect)
                .collect::<Result<_>>()?,
            letter_effects: root
                .objects(C, "2")?
                .iter()
                .map(read_letter_effect)
                .collect::<Result<_>>()?,
            timing: [root.get(C, "3")?, root.get(C, "4")?, root.get(C, "5")?],
            position: pair("6")?,
            box_size: pair("14")?,
            font: read_font(&root.sub(C, "7")?)?,
            alignment_h: root.get(C, "8")?,
            alignment_v: root.get(C, "9")?,
            blend: root.get(C, "10")?,
            member_12: root.get(C, "12")?,
            member_13: root.get(C, "13")?,
            member_15: root.get(C, "15")?,
            background_color: root.get(C, "20")?,
        })
    }

    /// The font file the configuration refers to (the only external reference of a `.tfx`).
    pub fn referenced_files(&self) -> Vec<&str> {
        self.font.font.as_deref().into_iter().collect()
    }
}

fn read_font(f: &Obj<'_>) -> Result<FontInstance> {
    const F: &str = "CFontInstance";
    let ident: u32 = f.get(F, "15")?;
    let member_15 = f
        .ctx
        .container
        .ident(ident)
        .ok_or_else(|| {
            f.ctx
                .bad(format!("identifier {ident} is not in the identifier table"))
        })?
        .to_string();
    let g = |n: &str| f.get::<f32>(F, n);
    Ok(FontInstance {
        font: f.file(F, "1")?,
        color: f.get(F, "2")?,
        flags: f.get(F, "3")?,
        members_4_to_7: [g("4")?, g("5")?, g("6")?, g("7")?],
        pixel_size: f.get(F, "8")?,
        member_9: g("9")?,
        shadow_color: f.get(F, "10")?,
        members_11_to_14: [g("11")?, g("12")?, g("13")?, g("14")?],
        member_16: f.get(F, "16")?,
        member_15,
    })
}

fn read_graphs(o: &Obj<'_>, owner: &str, enabled: &str) -> Result<EffectGraphs> {
    let g = |n: &str| Graph::read_pointer(o, owner, n);
    Ok(EffectGraphs {
        enabled: o.get(owner, enabled)?,
        graphs: [g("1")?, g("2")?, g("3")?, g("4")?],
    })
}

fn read_text_effect(o: &Obj<'_>) -> Result<TextEffect> {
    let base = read_graphs(o, "CTextEffect", "5")?;
    let kind = match o.class() {
        "CBlendTextEffect" => TextEffectKind::Blend {
            from: o.get("CBlendTextEffect", "1")?,
            to: o.get("CBlendTextEffect", "2")?,
        },
        "CStretchTextEffect" => {
            let g = |n: &str| o.get::<f32>("CStretchTextEffect", n);
            TextEffectKind::Stretch {
                values: [g("1")?, g("2")?, g("3")?, g("4")?],
            }
        }
        "CMoveTextEffect" => {
            let v = o.sub("CMoveTextEffect", "1")?;
            TextEffectKind::Move {
                offset: [v.get("Vector2f", "1")?, v.get("Vector2f", "2")?],
            }
        }
        other => {
            return Err(o
                .ctx
                .bad(format!("`{other}` is not a known text effect class")));
        }
    };
    Ok(TextEffect { base, kind })
}

fn read_letter_effect(o: &Obj<'_>) -> Result<LetterEffect> {
    let base = read_graphs(o, "CLetterEffect", "6")?;
    let kind = match o.class() {
        "CSizeLetterTextEffect" => {
            let g = |n: &str| o.get::<f32>("CSizeLetterTextEffect", n);
            LetterEffectKind::Size {
                values: [g("1")?, g("2")?, g("3")?, g("4")?],
            }
        }
        "CScreenOffsetAppearLetterTextEffect" => LetterEffectKind::ScreenOffsetAppear {
            member_1: o.get("CScreenOffsetAppearLetterTextEffect", "1")?,
            side: o.get("CScreenOffsetAppearLetterTextEffect", "2")?,
        },
        other => {
            return Err(o
                .ctx
                .bad(format!("`{other}` is not a known letter effect class")));
        }
    };
    Ok(LetterEffect {
        base,
        letter_delay: o.get("CLetterEffect", "5")?,
        kind,
    })
}

#[cfg(test)]
mod tests {
    use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

    use super::*;
    use crate::graph::GraphKind;
    use crate::vfx::tests::{
        T_FLOAT, T_GRAPH_PTR, T_INDEX, T_RESOURCE, T_RESOURCE_FILE, graph_base, graph_types, points,
    };

    /// A configuration with one move effect, one size letter effect, a font and the 640 x 480
    /// virtual-screen placement. Type ids: 19 `CFontInstance`, 20 `CTextEffect`, 21 move effect,
    /// 22 `CLetterEffect`, 23 size letter effect, 24 `CTextEffectConfiguration`, 25 `CStepUpGraph`.
    fn text_file(unknown_effect: bool) -> Vec<u8> {
        let mut b = MetaBuilder::new();
        let font = b.file_ref("Content/Test/Font.fnt", 0);
        let empty = b.identifier("empty");
        b.external_type(100, "CFont");
        graph_types(&mut b);
        b.type_block(TypeBlock::short(14, "Ptr<CFont>", 0, 11, 100));
        b.type_block(TypeBlock::alias(35, "PIX", 0));
        b.type_block(TypeBlock::primitive(15, "IDENT", 4, 4));
        b.type_block(TypeBlock::enumeration(16, "FontFlags", 4));
        b.type_block(TypeBlock::pointer(26, "CTextEffect*", 20));
        b.type_block(TypeBlock::pointer(27, "CLetterEffect*", 22));
        b.type_block(TypeBlock::short(
            28,
            "CDynamicContainer<CTextEffect>",
            0,
            8,
            20,
        ));
        b.type_block(TypeBlock::short(
            29,
            "CDynamicContainer<CLetterEffect>",
            0,
            8,
            22,
        ));
        b.type_block(TypeBlock::enumeration(30, "TextAlignmentH", 4));
        b.type_block(TypeBlock::enumeration(31, "TextAlignmentV", 4));
        b.type_block(TypeBlock::enumeration(32, "GfuBlendType", 4));
        b.type_block(TypeBlock::enumeration(33, "TextSide", 4));
        b.type_block(TypeBlock::class(
            19,
            "CFontInstance",
            1,
            5,
            None,
            &[
                ("1", 14),
                ("2", 8),
                ("3", 16),
                ("4", T_FLOAT),
                ("5", T_FLOAT),
                ("6", T_FLOAT),
                ("7", T_FLOAT),
                ("8", 35),
                ("9", T_FLOAT),
                ("10", 8),
                ("11", T_FLOAT),
                ("12", T_FLOAT),
                ("13", T_FLOAT),
                ("14", T_FLOAT),
                ("16", 0),
                ("15", 15),
            ],
        ));
        b.type_block(TypeBlock::class(
            20,
            "CTextEffect",
            1,
            5,
            None,
            &[
                ("5", 7),
                ("1", T_GRAPH_PTR),
                ("2", T_GRAPH_PTR),
                ("3", T_GRAPH_PTR),
                ("4", T_GRAPH_PTR),
            ],
        ));
        b.type_block(TypeBlock::class(
            21,
            "CMoveTextEffect",
            1,
            5,
            Some(20),
            &[("1", 4)],
        ));
        b.type_block(TypeBlock::class(
            22,
            "CLetterEffect",
            1,
            5,
            None,
            &[
                ("6", 7),
                ("1", T_GRAPH_PTR),
                ("2", T_GRAPH_PTR),
                ("3", T_GRAPH_PTR),
                ("4", T_GRAPH_PTR),
                ("5", T_FLOAT),
            ],
        ));
        b.type_block(TypeBlock::class(
            23,
            "CSizeLetterTextEffect",
            1,
            5,
            Some(22),
            &[
                ("1", T_FLOAT),
                ("2", T_FLOAT),
                ("3", T_FLOAT),
                ("4", T_FLOAT),
            ],
        ));
        b.type_block(TypeBlock::class(
            24,
            "CTextEffectConfiguration",
            1,
            5,
            Some(T_RESOURCE),
            &[
                ("1", 28),
                ("2", 29),
                ("3", T_FLOAT),
                ("4", T_FLOAT),
                ("5", T_FLOAT),
                ("6", 4),
                ("14", 4),
                ("7", 19),
                ("8", 30),
                ("9", 31),
                ("10", 32),
                ("12", 7),
                ("13", T_FLOAT),
                ("15", 7),
                ("20", 8),
            ],
        ));
        b.type_block(TypeBlock::class(
            25,
            "CStepUpGraph",
            1,
            5,
            Some(13),
            &[("1", 6)],
        ));
        // A class a `.tfx` must not contain.
        b.type_block(TypeBlock::class(
            34,
            "CMysteryTextEffect",
            1,
            5,
            Some(20),
            &[],
        ));
        b.external_object(9, font, 1, 100);
        b.object(
            0,
            T_RESOURCE_FILE,
            ObjectBytes::new().dynamic_container(&[1]).finish(),
        );
        let vec2 = |x: f32, y: f32| ObjectBytes::new().f32(x).f32(y).finish();
        let font_bytes = ObjectBytes::new()
            .object_ref(Some(9))
            .u32(0xFFFF_FFFF)
            .u32(1)
            .f32(0.0)
            .f32(0.0)
            .f32(0.3)
            .f32(0.75)
            .i32(90)
            .f32(1.0)
            .u32(0x7800_0000)
            .f32(0.0)
            .f32(1.0)
            .f32(1.0)
            .f32(0.0)
            .i32(0)
            .u32(empty)
            .finish();
        b.object(
            1,
            24,
            ObjectBytes::new()
                .i32(1)
                .dynamic_container(&[2])
                .dynamic_container(&[3])
                .f32(0.5)
                .f32(3.0)
                .f32(0.5)
                .raw(&vec2(320.0, 358.0))
                .raw(&vec2(640.0, 480.0))
                .raw(&font_bytes)
                .u32(1)
                .u32(1)
                .u32(501)
                .u32(0)
                .f32(0.5)
                .u32(1)
                .u32(0)
                .finish(),
        );
        b.object(
            2,
            if unknown_effect { 34 } else { 21 },
            {
                let o = ObjectBytes::new()
                    .u32(1)
                    .object_ref(Some(4))
                    .object_ref(Some(5))
                    .object_ref(Some(4))
                    .object_ref(Some(5));
                if unknown_effect {
                    o
                } else {
                    o.raw(&vec2(80.0, 80.0))
                }
            }
            .finish(),
        );
        b.object(
            3,
            23,
            ObjectBytes::new()
                .u32(1)
                .object_ref(Some(5))
                .object_ref(Some(5))
                .object_ref(Some(4))
                .object_ref(Some(5))
                .f32(0.02)
                .f32(0.9)
                .f32(1.0)
                .f32(0.9)
                .f32(1.0)
                .finish(),
        );
        b.object(
            4,
            25,
            graph_base(0)
                .raw(&points(&[(0.0, 0.0), (1.0, 1.0)]).finish())
                .finish(),
        );
        b.object(
            5,
            T_LINEAR_FOR_TEXT,
            graph_base(3)
                .f32(1.0)
                .raw(&points(&[(0.0, 1.0), (1.0, 1.0)]).finish())
                .finish(),
        );
        let _ = T_INDEX;
        b.build().unwrap()
    }

    const T_LINEAR_FOR_TEXT: u32 = crate::vfx::tests::T_LINEAR;

    #[test]
    fn reads_effects_graphs_font_and_placement() {
        let t = TextEffectConfiguration::from_bytes(&text_file(false)).unwrap();
        assert_eq!(t.timing, [0.5, 3.0, 0.5]);
        assert_eq!(t.position, [320.0, 358.0]);
        assert_eq!(t.box_size, [640.0, 480.0]);
        assert_eq!((t.alignment_h, t.alignment_v, t.blend), (1, 1, 501));
        assert_eq!(t.background_color, 0);
        assert_eq!(t.font.font.as_deref(), Some("Content/Test/Font.fnt"));
        assert_eq!(t.font.member_15, "empty");
        assert_eq!(t.font.shadow_color, 0x7800_0000);
        assert_eq!((t.font.pixel_size, t.font.member_9), (90, 1.0));
        assert_eq!(t.referenced_files(), ["Content/Test/Font.fnt"]);

        assert_eq!(t.effects.len(), 1);
        let e = &t.effects[0];
        assert_eq!(
            e.kind,
            TextEffectKind::Move {
                offset: [80.0, 80.0]
            }
        );
        assert!(e.base.enabled);
        let kinds: Vec<_> = e
            .base
            .graphs
            .iter()
            .map(|g| g.as_ref().unwrap().kind)
            .collect();
        assert_eq!(
            kinds,
            [
                GraphKind::StepUp,
                GraphKind::Linear,
                GraphKind::StepUp,
                GraphKind::Linear
            ]
        );
        assert_eq!(t.letter_effects.len(), 1);
        let l = &t.letter_effects[0];
        assert_eq!(l.letter_delay, 0.02);
        assert_eq!(
            l.kind,
            LetterEffectKind::Size {
                values: [0.9, 1.0, 0.9, 1.0]
            }
        );
    }

    #[test]
    fn an_unknown_effect_class_is_refused() {
        let err = TextEffectConfiguration::from_bytes(&text_file(true))
            .unwrap_err()
            .to_string();
        assert!(err.contains("CMysteryTextEffect"), "{err}");
    }

    #[test]
    fn nothing_panics_on_damaged_files() {
        assert!(TextEffectConfiguration::from_bytes(b"junk").is_err());
        let good = text_file(false);
        for len in (0..good.len()).step_by(7) {
            let _ = TextEffectConfiguration::from_bytes(&good[..len]);
        }
        for i in (0..good.len()).step_by(3) {
            let mut bad = good.clone();
            bad[i] ^= 0x21;
            let _ = TextEffectConfiguration::from_bytes(&bad);
        }
    }

    /// Data-gated: all 10 distinct `.tfx` files load, with the 13 whole-text effects, 5 letter
    /// effects and 72 graphs of `fx.md`, and one font each.
    #[test]
    fn every_text_effect_of_the_real_install_loads() {
        let dir = ss2_testkit::require_game_data!();
        let files = crate::testutil::install_files(&dir, "tfx");
        let (mut effects, mut letters, mut graphs) = (0usize, 0usize, 0usize);
        for (path, bytes) in &files {
            let t = TextEffectConfiguration::from_bytes(bytes)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            assert_eq!(t.referenced_files().len(), 1, "{path}");
            effects += t.effects.len();
            letters += t.letter_effects.len();
            let all = t
                .effects
                .iter()
                .map(|e| &e.base)
                .chain(t.letter_effects.iter().map(|e| &e.base));
            graphs += all.flat_map(|b| &b.graphs).flatten().count();
        }
        eprintln!(
            ".tfx: {} files, {effects} effects, {letters} letter effects, {graphs} graphs",
            files.len()
        );
        assert_eq!(files.len(), 10);
        assert_eq!((effects, letters), (13, 5));
        assert_eq!(graphs, (13 + 5) * 4);
    }
}
