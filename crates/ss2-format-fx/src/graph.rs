//! Graphs and graph holders, shared by all four formats (`docs/re/pfx.md`, `docs/re/fx.md`,
//! `docs/re/rsc-ctl.md`).
//!
//! A graph is a list of control points of one of six kinds. **How a graph is evaluated
//! (interpolation, the role of the common members) is not specified**: this module returns the
//! stored data and leaves evaluation to the renderer spec (T04). A [`GraphHolder`] is the
//! "constant or graph" rule of the particle files: the parameter is the constant `value` unless a
//! graph is set.

use ss2_core::Result;
use ss2_meta::ObjectRef;

use crate::read::{Ctx, Obj};

/// The class of a graph (all derive from `CGraph`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphKind {
    /// `CValueGraph`: one control point `(0, value)`.
    Value,
    /// `CLinearGraph`: linear between the points.
    Linear,
    /// `CSineGraph`: two points.
    Sine,
    /// `CStepUpGraph`: two points.
    StepUp,
    /// `CStepDownGraph`: two points.
    StepDown,
    /// `CRandomGraph`: two points.
    Random,
}

/// One control point. The second member of the stored record (`CControlPoint.4`) is uninitialised
/// editor memory and is dropped (spec invariant 4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlPoint {
    /// The x axis: the particle's or effect's life, a pitch angle, ... depending on the user.
    pub x: f32,
    /// The value at `x`.
    pub y: f32,
}

/// The members of the common base `CGraph` (`.7` to `.12`), whose roles are not identified.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphCommon {
    /// `CGraph.7` (0.06 to 560; 1 in the text and vibration effects).
    pub member_7: f32,
    /// `CGraph.8` (0.5 to 2.4; 1.0 mostly).
    pub member_8: f32,
    /// `CGraph.9`: 0 for value, step-up and step-down graphs, 3 for linear and 4 for sine graphs
    /// (looks like a type code).
    pub kind_code: u32,
    /// `CGraph.10` (0 to 11,202; unexplained).
    pub member_10: i32,
    /// `CGraph.11` (0 to 24).
    pub member_11: i32,
    /// `CGraph.12`: equals the constant of a value graph, 0 in other classes.
    pub member_12: f32,
}

/// A graph: its class, its control points and the stored base members.
#[derive(Clone, Debug, PartialEq)]
pub struct Graph {
    /// The class.
    pub kind: GraphKind,
    /// The control points in file order (one for a value graph, otherwise 1 to 21).
    pub points: Vec<ControlPoint>,
    /// The members of `CGraph`.
    pub common: GraphCommon,
    /// `CLinearGraph.1` (0.963 or 1.0); `None` for other classes.
    pub linear_member_1: Option<f32>,
    /// `CRandomGraph.3` (0 in the retail files); `None` for other classes.
    pub random_member_3: Option<i32>,
}

/// A parameter that is a constant unless a graph replaces it.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphHolder {
    /// `gh_fValue`: the constant. With a graph present it is probably the graph's scale or default
    /// (not determined).
    pub value: f32,
    /// `gh_pgrGraph`: the graph (set in 2.3 % of the retail holders).
    pub graph: Option<Graph>,
}

impl GraphHolder {
    /// The constant value if no graph is set.
    pub fn constant(&self) -> Option<f32> {
        self.graph.is_none().then_some(self.value)
    }
}

/// A pair of holders for a `Min`/`Max` range (how a value is sampled between them is open).
#[derive(Clone, Debug, PartialEq)]
pub struct MinMax {
    /// The lower bound.
    pub min: GraphHolder,
    /// The upper bound.
    pub max: GraphHolder,
}

impl GraphHolder {
    pub(crate) fn read(o: &Obj<'_>, owner: &str, name: &str) -> Result<Self> {
        let h = o.sub(owner, name)?;
        let value = h.get("CGraphHolder", "gh_fValue")?;
        let graph: ObjectRef = h.get("CGraphHolder", "gh_pgrGraph")?;
        let graph = graph.0.map(|id| Graph::read(o.ctx, id)).transpose()?;
        Ok(Self { value, graph })
    }
}

impl MinMax {
    pub(crate) fn read(o: &Obj<'_>, owner: &str, min: &str, max: &str) -> Result<Self> {
        Ok(Self {
            min: GraphHolder::read(o, owner, min)?,
            max: GraphHolder::read(o, owner, max)?,
        })
    }
}

impl Graph {
    /// Read the graph object `id`, dispatching on its class. An unknown class is an error
    /// (spec invariant 1).
    pub(crate) fn read(ctx: &Ctx<'_>, id: u32) -> Result<Self> {
        let o = ctx.object(id)?;
        let (kind, list_owner, list_name) = match o.class() {
            "CValueGraph" => (GraphKind::Value, "CValueGraph", "1"),
            "CLinearGraph" => (GraphKind::Linear, "CLinearGraph", "2"),
            "CSineGraph" => (GraphKind::Sine, "CSineGraph", "1"),
            "CStepUpGraph" => (GraphKind::StepUp, "CStepUpGraph", "1"),
            "CStepDownGraph" => (GraphKind::StepDown, "CStepDownGraph", "1"),
            "CRandomGraph" => (GraphKind::Random, "CRandomGraph", "4"),
            other => return Err(ctx.bad(format!("`{other}` is not a known graph class"))),
        };
        let points = if kind == GraphKind::Value {
            vec![read_point(&o.sub(list_owner, list_name)?)?]
        } else {
            o.subs(list_owner, list_name)?
                .iter()
                .map(read_point)
                .collect::<Result<_>>()?
        };
        Ok(Self {
            kind,
            points,
            common: GraphCommon {
                member_7: o.get("CGraph", "7")?,
                member_8: o.get("CGraph", "8")?,
                kind_code: o.get("CGraph", "9")?,
                member_10: o.get("CGraph", "10")?,
                member_11: o.get("CGraph", "11")?,
                member_12: o.get("CGraph", "12")?,
            },
            linear_member_1: if kind == GraphKind::Linear {
                Some(o.get("CLinearGraph", "1")?)
            } else {
                None
            },
            random_member_3: if kind == GraphKind::Random {
                Some(o.get("CRandomGraph", "3")?)
            } else {
                None
            },
        })
    }

    /// Read the graph a pointer member names (`None` for null).
    pub(crate) fn read_pointer(o: &Obj<'_>, owner: &str, name: &str) -> Result<Option<Self>> {
        let r: ObjectRef = o.get(owner, name)?;
        r.0.map(|id| Self::read(o.ctx, id)).transpose()
    }
}

fn read_point(p: &Obj<'_>) -> Result<ControlPoint> {
    let v = p.sub("CControlPoint", "3")?;
    Ok(ControlPoint {
        x: v.get("Vector2f", "1")?,
        y: v.get("Vector2f", "2")?,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn a_holder_without_a_graph_is_a_constant() {
        let h = GraphHolder {
            value: 2.5,
            graph: None,
        };
        assert_eq!(h.constant(), Some(2.5));
        let g = GraphHolder {
            value: 1.0,
            graph: Some(Graph {
                kind: GraphKind::StepUp,
                points: vec![],
                common: GraphCommon {
                    member_7: 1.0,
                    member_8: 1.0,
                    kind_code: 0,
                    member_10: 0,
                    member_11: 0,
                    member_12: 0.0,
                },
                linear_member_1: None,
                random_member_3: None,
            }),
        };
        assert_eq!(g.constant(), None);
    }
}
