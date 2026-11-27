//! Mod loading (roadmap T13-001; the formats are in `docs/re/mods.md`, the rules chosen here in
//! `docs/MOD_LOADING.md`).
//!
//! `mods.rs` reads one mod's descriptors and mounts one mod's archives. This module answers the
//! questions that come with **several** mods:
//!
//! * which mods does the install have, and what do they need ([`scan_installed_mods`]);
//! * in which order do the wanted mods load, dependencies first ([`resolve_load_order`]);
//! * how do they sit on the VFS mount stack ([`mount_mods`]);
//! * which files does each mod add or replace, and where two mods provide the same file, who
//!   wins ([`analyze_overlays`], [`overlay_report`]).
//!
//! The original game runs **one** mod at a time and does not say how several would combine
//! (`docs/re/mods.md`, "What is not in the data"). The rules here are SS2RS's own and are
//! documented as such: the order is the order of the request, dependencies (the optional
//! `REQUIRES_MOD` key of the descriptor `.nfo`, which the original game ignores) come before the
//! mods that need them, and ties are broken by id. The base game is always the bottom of the
//! stack, so a stack of one mod behaves exactly like the original's "current mod".
//!
//! Everything here is deterministic: no hash order, no clock, no host paths in the results.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::modcheck::ascii;

/// The id of the game itself, the default mod (`Content/SeriousSam2.mod`). It is always loaded
/// first and is never part of a [`LoadPlan`].
pub const BASE_MOD_ID: &str = "serioussam2";

/// The optional `.nfo` key that lists the mods a mod needs (SS2RS's own; the original game
/// ignores unknown keys): ids or `.mod` file names separated by `;`, `,` or white space.
pub const REQUIRES_MOD_KEY: &str = "REQUIRES_MOD";

/// The id a mod is known by: the name without a trailing `.mod`, trimmed, lower case (the VFS
/// lower-cases every name, `docs/re/mods.md`).
pub fn normalize_mod_id(raw: &str) -> String {
    let trimmed = raw.trim();
    let stem = match trimmed.len().checked_sub(4) {
        Some(cut)
            if trimmed.is_char_boundary(cut) && trimmed[cut..].eq_ignore_ascii_case(".mod") =>
        {
            &trimmed[..cut]
        }
        _ => trimmed,
    };
    stem.to_lowercase()
}

/// Split the value of [`REQUIRES_MOD_KEY`] into normalized ids, in order, without repeats and
/// without empty items. Separators are `;`, `,` and white space.
pub fn parse_mod_list(value: &str) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for item in value.split(|c: char| c == ';' || c == ',' || c.is_whitespace()) {
        let id = normalize_mod_id(item);
        if !id.is_empty() && !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

/// One mod as the load order sees it: its id and the ids it needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModNode {
    /// The mod's id (normalized when it enters [`resolve_load_order`]).
    pub id: String,
    /// The ids of the mods that must be loaded before this one.
    pub requires: Vec<String>,
}

impl ModNode {
    /// A node from an id and the ids it needs.
    pub fn new(
        id: impl Into<String>,
        requires: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            id: id.into(),
            requires: requires.into_iter().map(Into::into).collect(),
        }
    }
}

/// Why a mod is in a [`LoadPlan`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Why {
    /// The request named it.
    Requested,
    /// Another mod needs it; the id is the smallest of the mods that do.
    RequiredBy(String),
}

/// One mod of a [`LoadPlan`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedMod {
    /// The mod's id.
    pub id: String,
    /// Why it is loaded.
    pub why: Why,
}

/// The mods to load on top of the base game, **in load order**: later mods win.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoadPlan {
    /// The mods, first loaded first.
    pub mods: Vec<PlannedMod>,
}

impl LoadPlan {
    /// The ids in load order.
    pub fn ids(&self) -> Vec<&str> {
        self.mods.iter().map(|m| m.id.as_str()).collect()
    }
}

impl LoadPlan {
    /// A plain ASCII listing of the plan, one mod per line, with the reason it is loaded.
    pub fn render(&self) -> String {
        if self.mods.is_empty() {
            return "Load order: only the base game.
"
            .to_owned();
        }
        let mut out = String::from(
            "Load order on top of the base game (a later mod wins):
",
        );
        for (i, m) in self.mods.iter().enumerate() {
            let why = match &m.why {
                Why::Requested => "requested".to_owned(),
                Why::RequiredBy(by) => format!("needed by {}", ascii(by)),
            };
            out.push_str(&format!(
                "  {}. {} ({why})
",
                i + 1,
                ascii(&m.id)
            ));
        }
        out
    }
}

/// Why a request cannot be turned into a [`LoadPlan`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadOrderError {
    /// A requested mod, or one that a mod needs, is not installed.
    UnknownMod {
        /// The id that is not known.
        id: String,
        /// The mod that needs it; `None` when the request itself names it.
        required_by: Option<String>,
    },
    /// Mods that need each other in a circle.
    Cycle {
        /// The mods of the circle, each needing the next, the last needing the first; it starts
        /// with the smallest id.
        ids: Vec<String>,
    },
}

impl fmt::Display for LoadOrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownMod {
                id,
                required_by: None,
            } => write!(f, "the mod `{id}` is not installed"),
            Self::UnknownMod {
                id,
                required_by: Some(by),
            } => write!(f, "the mod `{by}` needs `{id}`, which is not installed"),
            Self::Cycle { ids } => {
                write!(
                    f,
                    "mods that need each other in a circle: {}",
                    ids.join(" -> ")
                )?;
                if let Some(first) = ids.first() {
                    write!(f, " -> {first}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for LoadOrderError {}

/// Turn a request into a load order.
///
/// `known` is every installed mod (its order does not matter); `requested` is what the player
/// wants enabled, **first loaded first**. The result holds the requested mods and, pulled in
/// automatically, everything they need, transitively. The rules:
///
/// 1. a mod loads after every mod it needs;
/// 2. among the mods that may load next, a pulled-in dependency goes before a requested mod, and
///    requested mods go in the order of the request;
/// 3. what is still tied is ordered by id.
///
/// So a request that already respects the dependencies comes out unchanged, and the same request
/// gives the same plan whatever the order of `known`. The base game ([`BASE_MOD_ID`]) is always
/// there: naming it, or needing it, changes nothing. A mod that needs itself is treated as not
/// needing anything. A repeated id counts once, at its first place; two entries of `known` with
/// one id are merged (their requirements are added).
///
/// # Errors
/// [`LoadOrderError::UnknownMod`] for the first (in request order, then in id order) id that is
/// not in `known`; [`LoadOrderError::Cycle`] when mods need each other in a circle.
pub fn resolve_load_order(
    known: &[ModNode],
    requested: &[String],
) -> Result<LoadPlan, LoadOrderError> {
    let mut table: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for node in known {
        let id = normalize_mod_id(&node.id);
        if id.is_empty() || id == BASE_MOD_ID {
            continue;
        }
        let needs = table.entry(id.clone()).or_default();
        for req in &node.requires {
            let req = normalize_mod_id(req);
            if !req.is_empty() && req != BASE_MOD_ID && req != id {
                needs.insert(req);
            }
        }
    }

    // The request: normalized, without the base game, each id at its first place.
    let mut rank: BTreeMap<String, usize> = BTreeMap::new();
    for raw in requested {
        let id = normalize_mod_id(raw);
        if id.is_empty() || id == BASE_MOD_ID {
            continue;
        }
        let next = rank.len();
        rank.entry(id).or_insert(next);
    }
    let mut asked: Vec<(&String, &usize)> = rank.iter().collect();
    asked.sort_by_key(|(_, r)| **r);

    // Everything that has to load: the request and what it needs, transitively.
    let mut included: BTreeSet<String> = BTreeSet::new();
    let mut pending: Vec<(String, Option<String>)> = asked
        .iter()
        .rev()
        .map(|(id, _)| ((*id).clone(), None))
        .collect();
    while let Some((id, required_by)) = pending.pop() {
        let Some(needs) = table.get(&id) else {
            return Err(LoadOrderError::UnknownMod { id, required_by });
        };
        if !included.insert(id.clone()) {
            continue;
        }
        for req in needs.iter().rev() {
            if !included.contains(req) {
                pending.push((req.clone(), Some(id.clone())));
            }
        }
    }

    // Kahn's algorithm that always takes the smallest ready mod: a pulled-in dependency
    // (class 0), then the requested mods in request order (class 1), then by id.
    let key = |id: &String| match rank.get(id) {
        Some(r) => (1u8, *r, id.clone()),
        None => (0u8, 0usize, id.clone()),
    };
    let mut missing: BTreeMap<&String, usize> = BTreeMap::new();
    let mut dependents: BTreeMap<&String, Vec<&String>> = BTreeMap::new();
    for id in &included {
        let needs = &table[id];
        missing.insert(id, needs.len());
        for req in needs {
            dependents.entry(req).or_default().push(id);
        }
    }
    let mut ready: BTreeSet<(u8, usize, String)> = included
        .iter()
        .filter(|id| missing[id] == 0)
        .map(key)
        .collect();
    let mut mods = Vec::with_capacity(included.len());
    while let Some(first) = ready.pop_first() {
        let id = first.2;
        for &dependent in dependents.get(&id).into_iter().flatten() {
            let left = missing
                .get_mut(dependent)
                .expect("every dependent is included");
            *left -= 1;
            if *left == 0 {
                ready.insert(key(dependent));
            }
        }
        let why = if rank.contains_key(&id) {
            Why::Requested
        } else {
            let by = dependents[&id]
                .iter()
                .min()
                .expect("a pulled-in mod has a dependent");
            Why::RequiredBy((*by).clone())
        };
        mods.push(PlannedMod { id, why });
    }
    if mods.len() < included.len() {
        let placed: BTreeSet<&str> = mods.iter().map(|m| m.id.as_str()).collect();
        let left: BTreeSet<&String> = included
            .iter()
            .filter(|id| !placed.contains(id.as_str()))
            .collect();
        return Err(LoadOrderError::Cycle {
            ids: find_cycle(&table, &left),
        });
    }
    Ok(LoadPlan { mods })
}

/// A circle among `left`, mods that all still wait for another of them: follow the smallest
/// waited-for mod until one repeats, and start the circle at its smallest id.
fn find_cycle(table: &BTreeMap<String, BTreeSet<String>>, left: &BTreeSet<&String>) -> Vec<String> {
    let mut path: Vec<&String> = Vec::new();
    let mut at = *left.iter().next().expect("a cycle needs at least one mod");
    loop {
        if let Some(start) = path.iter().position(|p| **p == *at) {
            let mut circle: Vec<String> = path[start..].iter().map(|s| (*s).clone()).collect();
            let smallest = (0..circle.len())
                .min_by_key(|&i| circle[i].clone())
                .unwrap_or(0);
            circle.rotate_left(smallest);
            return circle;
        }
        path.push(at);
        at = table[at]
            .iter()
            .find_map(|req| left.get(req).copied())
            .expect("a mod that is left waits for another one that is left");
    }
}

#[cfg(test)]
mod order_tests {
    use super::*;

    fn nodes(spec: &[(&str, &[&str])]) -> Vec<ModNode> {
        spec.iter()
            .map(|(id, req)| ModNode::new(*id, req.iter().copied()))
            .collect()
    }

    fn ask(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    fn ids(plan: &LoadPlan) -> Vec<&str> {
        plan.ids()
    }

    #[test]
    fn ids_are_normalized() {
        assert_eq!(normalize_mod_id("InSamnity2.mod"), "insamnity2");
        assert_eq!(normalize_mod_id("  Sam2Renovation.MOD "), "sam2renovation");
        assert_eq!(normalize_mod_id("plain"), "plain");
        assert_eq!(normalize_mod_id(".mod"), "");
        assert_eq!(normalize_mod_id("x.modx"), "x.modx");
        // A multi-byte character in the last four bytes must not panic.
        assert_eq!(normalize_mod_id("caf\u{e9}"), "caf\u{e9}");
    }

    #[test]
    fn a_mod_list_is_split_and_normalized() {
        assert_eq!(parse_mod_list("A.mod; b , C\td;;A"), ["a", "b", "c", "d"]);
        assert!(parse_mod_list("  ;, ").is_empty());
    }

    #[test]
    fn a_plan_renders_with_the_reasons() {
        let known = nodes(&[("addon", &["core"]), ("core", &[])]);
        let plan = resolve_load_order(&known, &ask(&["addon"])).unwrap();
        assert_eq!(
            plan.render(),
            "Load order on top of the base game (a later mod wins):
  1. core (needed by addon)
  2. addon (requested)
"
        );
        assert_eq!(
            LoadPlan::default().render(),
            "Load order: only the base game.
"
        );
    }

    #[test]
    fn an_empty_request_loads_only_the_base() {
        let plan = resolve_load_order(&nodes(&[("a", &[])]), &[]).unwrap();
        assert!(plan.mods.is_empty());
    }

    #[test]
    fn the_request_order_is_the_load_order() {
        let known = nodes(&[("a", &[]), ("b", &[]), ("c", &[])]);
        let plan = resolve_load_order(&known, &ask(&["c", "a", "b"])).unwrap();
        assert_eq!(ids(&plan), ["c", "a", "b"]);
        assert!(plan.mods.iter().all(|m| m.why == Why::Requested));
    }

    #[test]
    fn names_are_matched_without_case_and_with_the_extension() {
        let known = nodes(&[("Alpha", &[]), ("beta.mod", &[])]);
        let plan = resolve_load_order(&known, &ask(&["BETA", "alpha.MOD"])).unwrap();
        assert_eq!(ids(&plan), ["beta", "alpha"]);
    }

    #[test]
    fn a_repeated_request_counts_once_at_its_first_place() {
        let known = nodes(&[("a", &[]), ("b", &[])]);
        let plan = resolve_load_order(&known, &ask(&["b", "a", "b"])).unwrap();
        assert_eq!(ids(&plan), ["b", "a"]);
    }

    #[test]
    fn a_dependency_is_pulled_in_and_loads_first() {
        let known = nodes(&[("addon", &["core"]), ("core", &[])]);
        let plan = resolve_load_order(&known, &ask(&["addon"])).unwrap();
        assert_eq!(ids(&plan), ["core", "addon"]);
        assert_eq!(plan.mods[0].why, Why::RequiredBy("addon".into()));
        assert_eq!(plan.mods[1].why, Why::Requested);
    }

    #[test]
    fn a_requested_dependency_moves_before_its_dependent() {
        let known = nodes(&[("addon", &["core"]), ("core", &[]), ("x", &[])]);
        let plan = resolve_load_order(&known, &ask(&["addon", "x", "core"])).unwrap();
        // `addon` needs `core`, so it moves behind it; `x` keeps its place before `core`.
        assert_eq!(ids(&plan), ["x", "core", "addon"]);
        assert_eq!(plan.mods[1].why, Why::Requested);
    }

    #[test]
    fn dependencies_are_transitive_and_ties_go_by_id() {
        let known = nodes(&[
            ("top", &["mid-b", "mid-a"]),
            ("mid-a", &["root"]),
            ("mid-b", &["root"]),
            ("root", &[]),
        ]);
        let plan = resolve_load_order(&known, &ask(&["top"])).unwrap();
        assert_eq!(ids(&plan), ["root", "mid-a", "mid-b", "top"]);
        assert_eq!(plan.mods[1].why, Why::RequiredBy("top".into()));
        assert_eq!(plan.mods[0].why, Why::RequiredBy("mid-a".into()));
    }

    #[test]
    fn a_pulled_in_dependency_goes_before_unrelated_requested_mods() {
        let known = nodes(&[("a", &[]), ("b", &["dep"]), ("dep", &[])]);
        let plan = resolve_load_order(&known, &ask(&["a", "b"])).unwrap();
        // Both `a` and `dep` can load first; the dependency goes before a requested mod.
        assert_eq!(ids(&plan), ["dep", "a", "b"]);
    }

    #[test]
    fn the_base_game_is_always_there_and_never_planned() {
        let known = nodes(&[("addon", &["SeriousSam2.mod"]), ("serioussam2", &[])]);
        let plan = resolve_load_order(&known, &ask(&["SeriousSam2", "addon"])).unwrap();
        assert_eq!(ids(&plan), ["addon"]);
        // It need not be among the installed mods.
        let plan =
            resolve_load_order(&nodes(&[("addon", &["serioussam2"])]), &ask(&["addon"])).unwrap();
        assert_eq!(ids(&plan), ["addon"]);
    }

    #[test]
    fn a_mod_that_needs_itself_needs_nothing() {
        let plan = resolve_load_order(&nodes(&[("a", &["A"])]), &ask(&["a"])).unwrap();
        assert_eq!(ids(&plan), ["a"]);
    }

    #[test]
    fn duplicate_known_entries_are_merged() {
        let known = nodes(&[("a", &["b"]), ("a", &["c"]), ("b", &[]), ("c", &[])]);
        let plan = resolve_load_order(&known, &ask(&["a"])).unwrap();
        assert_eq!(ids(&plan), ["b", "c", "a"]);
    }

    #[test]
    fn an_unknown_request_or_dependency_is_named() {
        let known = nodes(&[("a", &["ghost"]), ("b", &[])]);
        assert_eq!(
            resolve_load_order(&known, &ask(&["nope"])),
            Err(LoadOrderError::UnknownMod {
                id: "nope".into(),
                required_by: None
            })
        );
        let err = resolve_load_order(&known, &ask(&["b", "a"])).unwrap_err();
        assert_eq!(
            err,
            LoadOrderError::UnknownMod {
                id: "ghost".into(),
                required_by: Some("a".into())
            }
        );
        assert_eq!(
            err.to_string(),
            "the mod `a` needs `ghost`, which is not installed"
        );
        assert_eq!(
            LoadOrderError::UnknownMod {
                id: "x".into(),
                required_by: None
            }
            .to_string(),
            "the mod `x` is not installed"
        );
    }

    #[test]
    fn of_several_unknown_dependencies_the_smallest_is_named() {
        let known = nodes(&[("a", &["zz", "aa"])]);
        let err = resolve_load_order(&known, &ask(&["a"])).unwrap_err();
        assert_eq!(
            err,
            LoadOrderError::UnknownMod {
                id: "aa".into(),
                required_by: Some("a".into())
            }
        );
    }

    #[test]
    fn a_cycle_is_reported_from_its_smallest_member() {
        let known = nodes(&[
            ("c", &["a"]),
            ("a", &["b"]),
            ("b", &["c"]),
            ("tail", &["b"]),
        ]);
        let err = resolve_load_order(&known, &ask(&["tail"])).unwrap_err();
        assert_eq!(
            err,
            LoadOrderError::Cycle {
                ids: ask(&["a", "b", "c"])
            }
        );
        assert_eq!(
            err.to_string(),
            "mods that need each other in a circle: a -> b -> c -> a"
        );
    }

    #[test]
    fn two_mods_in_a_circle_are_reported_too() {
        let known = nodes(&[("x", &["y"]), ("y", &["x"])]);
        let err = resolve_load_order(&known, &ask(&["y"])).unwrap_err();
        assert_eq!(
            err,
            LoadOrderError::Cycle {
                ids: ask(&["x", "y"])
            }
        );
    }

    #[test]
    fn a_cycle_outside_the_request_is_not_an_error() {
        let known = nodes(&[("x", &["y"]), ("y", &["x"]), ("ok", &[])]);
        let plan = resolve_load_order(&known, &ask(&["ok"])).unwrap();
        assert_eq!(ids(&plan), ["ok"]);
    }
}

#[cfg(test)]
mod order_properties {
    use super::*;
    use proptest::prelude::*;

    /// A random acyclic world: `n` mods named in a random order; each needs a subset of the mods
    /// before it in `perm`, so `perm` itself is a valid load order.
    fn world() -> impl Strategy<Value = (Vec<String>, Vec<ModNode>)> {
        (1usize..9)
            .prop_flat_map(|n| {
                let names: Vec<String> = (0..n).map(|i| format!("mod{i}")).collect();
                let perm = Just(names).prop_shuffle();
                let picks =
                    proptest::collection::vec(proptest::collection::vec(any::<bool>(), n), n);
                (perm, picks)
            })
            .prop_map(|(perm, picks)| {
                let nodes = perm
                    .iter()
                    .enumerate()
                    .map(|(i, id)| {
                        let needs = (0..i).filter(|&j| picks[i][j]).map(|j| perm[j].clone());
                        ModNode::new(id.clone(), needs)
                    })
                    .collect();
                (perm, nodes)
            })
    }

    fn closure(known: &[ModNode], request: &[String]) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut stack: Vec<String> = request.to_vec();
        while let Some(id) = stack.pop() {
            if out.insert(id.clone()) {
                let node = known.iter().find(|n| n.id == id).unwrap();
                stack.extend(node.requires.iter().cloned());
            }
        }
        out
    }

    fn assert_valid(known: &[ModNode], request: &[String], plan: &LoadPlan) {
        let ids = plan.ids();
        let want = closure(known, request);
        assert_eq!(
            ids.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>(),
            want
        );
        assert_eq!(ids.len(), want.len(), "no mod twice");
        for (at, id) in ids.iter().enumerate() {
            let node = known.iter().find(|n| n.id == *id).unwrap();
            for req in &node.requires {
                let before = ids.iter().position(|x| x == req).unwrap();
                assert!(before < at, "{req} must load before {id}: {ids:?}");
            }
        }
    }

    proptest! {
        #[test]
        fn every_plan_is_the_closure_in_a_dependency_respecting_order(
            (perm, known) in world(),
            picks in proptest::collection::vec(any::<prop::sample::Index>(), 0..12),
        ) {
            let request: Vec<String> = picks.iter().map(|i| perm[i.index(perm.len())].clone()).collect();
            let plan = resolve_load_order(&known, &request).unwrap();
            assert_valid(&known, &request, &plan);
        }

        #[test]
        fn the_plan_does_not_depend_on_the_order_of_the_known_mods(
            (perm, known) in world(),
            picks in proptest::collection::vec(any::<prop::sample::Index>(), 0..12),
            turn in 0usize..9,
        ) {
            let request: Vec<String> = picks.iter().map(|i| perm[i.index(perm.len())].clone()).collect();
            let mut shuffled = known.clone();
            shuffled.reverse();
            let len = shuffled.len();
            shuffled.rotate_left(turn % len);
            prop_assert_eq!(
                resolve_load_order(&known, &request),
                resolve_load_order(&shuffled, &request)
            );
        }

        #[test]
        fn a_plan_asked_for_again_comes_out_the_same(
            (perm, known) in world(),
            picks in proptest::collection::vec(any::<prop::sample::Index>(), 0..12),
        ) {
            let request: Vec<String> = picks.iter().map(|i| perm[i.index(perm.len())].clone()).collect();
            let plan = resolve_load_order(&known, &request).unwrap();
            let again: Vec<String> = plan.ids().iter().map(ToString::to_string).collect();
            let second = resolve_load_order(&known, &again).unwrap();
            prop_assert_eq!(second.ids(), plan.ids());
        }

        #[test]
        fn a_request_that_already_respects_its_dependencies_is_kept(
            (perm, known) in world(),
            take in 0usize..9,
        ) {
            // Any prefix of `perm` is closed under "needs" and in a valid order.
            let request: Vec<String> = perm.iter().take(take).cloned().collect();
            let plan = resolve_load_order(&known, &request).unwrap();
            prop_assert_eq!(plan.ids(), request.iter().map(String::as_str).collect::<Vec<_>>());
            prop_assert!(plan.mods.iter().all(|m| m.why == Why::Requested));
        }

        #[test]
        fn with_arbitrary_needs_the_answer_is_a_valid_plan_or_a_real_circle(
            n in 1usize..7,
            edges in proptest::collection::vec(proptest::collection::vec(any::<bool>(), 7), 7),
            picks in proptest::collection::vec(any::<prop::sample::Index>(), 1..8),
        ) {
            let names: Vec<String> = (0..n).map(|i| format!("m{i}")).collect();
            let known: Vec<ModNode> = (0..n)
                .map(|i| ModNode::new(names[i].clone(), (0..n).filter(|&j| j != i && edges[i][j]).map(|j| names[j].clone())))
                .collect();
            let request: Vec<String> = picks.iter().map(|i| names[i.index(n)].clone()).collect();
            match resolve_load_order(&known, &request) {
                Ok(plan) => assert_valid(&known, &request, &plan),
                Err(LoadOrderError::Cycle { ids }) => {
                    prop_assert!(!ids.is_empty());
                    let reach = closure(&known, &request);
                    for (i, id) in ids.iter().enumerate() {
                        prop_assert!(reach.contains(id));
                        let next = &ids[(i + 1) % ids.len()];
                        let node = known.iter().find(|k| &k.id == id).unwrap();
                        prop_assert!(node.requires.contains(next), "{id} must need {next}");
                    }
                    prop_assert_eq!(ids.iter().min(), ids.first());
                }
                Err(other) => prop_assert!(false, "unexpected {other}"),
            }
        }
    }
}
