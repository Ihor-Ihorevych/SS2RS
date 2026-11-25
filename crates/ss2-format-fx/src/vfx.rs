//! `.vfx` controller vibration effects (spec `docs/re/fx.md`).
//!
//! A [`VibrationEffect`] (class `CXboxVibroFX`) has one optional graph per motor and a duration.
//! By the file names `.4` is the low-frequency (large) motor and `.5` the high-frequency (small)
//! one; the graphs run over the normalized duration (x 0 to 1) with y the motor strength 0 to 1.
//! Nothing in the retail data refers to a `.vfx` by path.

use ss2_core::Result;

use crate::graph::Graph;
use crate::read::Ctx;

const WHAT: &str = ".vfx vibration effect";

/// A decoded `.vfx` file.
#[derive(Clone, Debug, PartialEq)]
pub struct VibrationEffect {
    /// `CXboxVibroFX.4`: the low-frequency (large) motor, `None` when null.
    pub low_frequency: Option<Graph>,
    /// `CXboxVibroFX.5`: the high-frequency (small) motor, `None` when null.
    pub high_frequency: Option<Graph>,
    /// `CXboxVibroFX.6`: almost certainly the duration in seconds (5.0 in all retail files).
    pub duration: f32,
}

impl VibrationEffect {
    /// Read a `.vfx` file.
    ///
    /// # Errors
    /// [`ss2_core::Error::Invalid`] naming what is wrong (also an unknown graph class).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let (container, decoded) = Ctx::decode(bytes, WHAT)?;
        let ctx = Ctx {
            container: &container,
            decoded: &decoded,
            what: WHAT,
        };
        let root = ctx.root("CXboxVibroFX")?;
        Ok(Self {
            low_frequency: Graph::read_pointer(&root, "CXboxVibroFX", "4")?,
            high_frequency: Graph::read_pointer(&root, "CXboxVibroFX", "5")?,
            duration: root.get("CXboxVibroFX", "6")?,
        })
    }

    /// A vibration effect refers to no other file.
    pub fn referenced_files(&self) -> Vec<&str> {
        Vec::new()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

    use super::*;
    use crate::graph::GraphKind;

    /// Type ids of the graph classes in [`graph_types`].
    pub(crate) const T_RESOURCE_FILE: u32 = 11;
    pub(crate) const T_RESOURCE: u32 = 10;
    pub(crate) const T_RESOURCE_LIST: u32 = 9;
    pub(crate) const T_GRAPH_PTR: u32 = 12;
    pub(crate) const T_LINEAR: u32 = 17;
    pub(crate) const T_SINE: u32 = 18;
    pub(crate) const T_FLOAT: u32 = 2;
    pub(crate) const T_INDEX: u32 = 1;

    /// Types 0 to 18: primitives, `CResource`, `CResourceFile`, `CGraph` and its linear and sine
    /// classes (as in `fx.md`/`rsc-ctl.md`).
    pub(crate) fn graph_types(b: &mut MetaBuilder) {
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        b.type_block(TypeBlock::alias(1, "INDEX", 0));
        b.type_block(TypeBlock::primitive(2, "FLOAT", 4, 4));
        b.type_block(TypeBlock::primitive(3, "ULONG", 4, 4));
        b.type_block(TypeBlock::class(
            4,
            "Vector2f",
            1,
            5,
            None,
            &[("1", 2), ("2", 2)],
        ));
        b.type_block(TypeBlock::class(
            5,
            "CControlPoint",
            1,
            5,
            None,
            &[("3", 4), ("4", 1)],
        ));
        b.type_block(TypeBlock::short(
            6,
            "CStaticStackArray<CControlPoint>",
            0,
            7,
            5,
        ));
        b.type_block(TypeBlock::primitive(7, "BOOL", 4, 4));
        b.type_block(TypeBlock::primitive(8, "COLOR", 4, 4));
        b.type_block(TypeBlock::class(
            13,
            "CGraph",
            1,
            5,
            None,
            &[
                ("7", 2),
                ("8", 2),
                ("9", 3),
                ("10", 1),
                ("11", 1),
                ("12", 2),
            ],
        ));
        b.type_block(TypeBlock::short(
            T_RESOURCE_LIST,
            "CDynamicContainer<CResource>",
            0,
            8,
            T_RESOURCE,
        ));
        b.type_block(TypeBlock::class(
            T_RESOURCE,
            "CResource",
            4,
            5,
            None,
            &[("1", T_INDEX)],
        ));
        b.type_block(TypeBlock::class(
            T_RESOURCE_FILE,
            "CResourceFile",
            3,
            5,
            None,
            &[("rf_crsShared", T_RESOURCE_LIST)],
        ));
        b.type_block(TypeBlock::pointer(T_GRAPH_PTR, "CGraph*", 13));
        b.type_block(TypeBlock::class(
            T_LINEAR,
            "CLinearGraph",
            1,
            5,
            Some(13),
            &[("1", 2), ("2", 6)],
        ));
        b.type_block(TypeBlock::class(
            T_SINE,
            "CSineGraph",
            1,
            5,
            Some(13),
            &[("1", 6)],
        ));
    }

    /// The members of a `CGraph` base (`.7` to `.12`) for a linear (`kind_code` 3) graph.
    pub(crate) fn graph_base(kind_code: u32) -> ObjectBytes {
        ObjectBytes::new()
            .f32(1.0)
            .f32(1.0)
            .u32(kind_code)
            .i32(137)
            .i32(2)
            .f32(0.0)
    }

    /// Two control points as stored: `(x, y)` and the garbage index.
    pub(crate) fn points(pts: &[(f32, f32)]) -> ObjectBytes {
        let mut o = ObjectBytes::new().array_header(b"SSAR", pts.len() as u32);
        for (i, (x, y)) in pts.iter().enumerate() {
            o = o.f32(*x).f32(*y).u32(if i == 0 { 0xCDCD_CDCD } else { 0 });
        }
        o
    }

    /// `bothfreqmax`-like file: low motor a linear ramp (object 2), high motor a sine (object 3),
    /// duration 5.
    pub(crate) fn vibro_file(low: bool, high: bool) -> Vec<u8> {
        let mut b = MetaBuilder::new();
        graph_types(&mut b);
        b.type_block(TypeBlock::class(
            19,
            "CXboxVibroFX",
            1,
            5,
            Some(T_RESOURCE),
            &[("4", T_GRAPH_PTR), ("5", T_GRAPH_PTR), ("6", T_FLOAT)],
        ));
        b.object(
            0,
            T_RESOURCE_FILE,
            ObjectBytes::new().dynamic_container(&[1]).finish(),
        );
        b.object(
            1,
            19,
            ObjectBytes::new()
                .i32(1)
                .object_ref(low.then_some(2))
                .object_ref(high.then_some(3))
                .f32(5.0)
                .finish(),
        );
        b.object(
            2,
            T_LINEAR,
            graph_base(3)
                .f32(1.0)
                .raw(&points(&[(0.0, 0.0), (1.0, 1.0)]).finish())
                .finish(),
        );
        b.object(
            3,
            T_SINE,
            graph_base(4)
                .raw(&points(&[(0.0, 0.5), (1.0, 1.0)]).finish())
                .finish(),
        );
        b.build().unwrap()
    }

    #[test]
    fn reads_both_motors_and_the_duration() {
        let v = VibrationEffect::from_bytes(&vibro_file(true, true)).unwrap();
        assert_eq!(v.duration, 5.0);
        let low = v.low_frequency.unwrap();
        assert_eq!(low.kind, GraphKind::Linear);
        assert_eq!(low.points.len(), 2);
        assert_eq!((low.points[1].x, low.points[1].y), (1.0, 1.0));
        assert_eq!(low.common.kind_code, 3);
        assert_eq!(low.common.member_10, 137);
        assert_eq!(low.linear_member_1, Some(1.0));
        let high = v.high_frequency.unwrap();
        assert_eq!(high.kind, GraphKind::Sine);
        assert_eq!((high.points[0].x, high.points[0].y), (0.0, 0.5));
        assert_eq!(high.linear_member_1, None);
    }

    #[test]
    fn a_null_motor_is_none() {
        let v = VibrationEffect::from_bytes(&vibro_file(false, true)).unwrap();
        assert!(v.low_frequency.is_none() && v.high_frequency.is_some());
        assert!(v.referenced_files().is_empty());
    }

    #[test]
    fn junk_and_damaged_files_are_errors_not_panics() {
        assert!(VibrationEffect::from_bytes(b"junk").is_err());
        let good = vibro_file(true, true);
        for len in (0..good.len()).step_by(5) {
            let _ = VibrationEffect::from_bytes(&good[..len]);
        }
        for i in (0..good.len()).step_by(3) {
            let mut bad = good.clone();
            bad[i] ^= 0x5A;
            let _ = VibrationEffect::from_bytes(&bad);
        }
    }

    /// Data-gated: all 5 `.vfx` files load; three have a low and three a high motor graph.
    #[test]
    fn every_vibration_effect_of_the_real_install_loads() {
        let dir = ss2_testkit::require_game_data!();
        let files = crate::testutil::install_files(&dir, "vfx");
        let (mut low, mut high) = (0, 0);
        for (path, bytes) in &files {
            let v = VibrationEffect::from_bytes(bytes).unwrap_or_else(|e| panic!("{path}: {e}"));
            assert_eq!(v.duration, 5.0, "{path}");
            low += usize::from(v.low_frequency.is_some());
            high += usize::from(v.high_frequency.is_some());
        }
        eprintln!(".vfx: {} files, {low} low, {high} high", files.len());
        assert_eq!((files.len(), low, high), (5, 3, 3));
    }
}
