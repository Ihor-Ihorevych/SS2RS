use super::*;

const DOC: &str = include_str!("../../../../docs/CAMPAIGN.md");

fn index(r: Rule) -> usize {
    match r {
        Rule::LevelSet => 0,
        Rule::LevelOrder => 1,
        Rule::MoviesPerLevel => 2,
        Rule::OutroThenLoad => 3,
        Rule::StartInventory => 4,
        Rule::ListShape => 5,
        Rule::ListValidation => 6,
        Rule::Exits => 7,
        Rule::LevelKinds => 8,
        Rule::StageOrder => 9,
        Rule::NoMoviesOnRestart => 10,
        Rule::WhenToLeave => 11,
        Rule::GateEveryone => 12,
        Rule::GateTimeout => 13,
        Rule::GhostKeepsState => 14,
        Rule::NobodyAlive => 15,
        Rule::DifficultyCarries => 16,
        Rule::CarryFloor => 17,
        Rule::DeadAndLateStart => 18,
        Rule::RestartKeepsEntry => 19,
        Rule::ElapsedOnlyInPlay => 20,
        Rule::UnlockExitTarget => 21,
        Rule::UnlockTriggers => 22,
        Rule::SaveOnlyWhenPlaying => 23,
        Rule::Fingerprint => 24,
    }
}

#[test]
fn every_rule_is_listed_once_in_order() {
    for (i, r) in Rule::ALL.iter().enumerate() {
        assert_eq!(index(*r), i, "{r:?}");
    }
    let mut keys: Vec<&str> = Rule::ALL.iter().map(|r| r.key()).collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), Rule::ALL.len(), "keys must be unique");
    assert!(
        keys.iter()
            .all(|k| !k.is_empty() && k.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'))
    );
}

#[test]
fn five_rules_come_from_the_spec_and_the_rest_are_chosen() {
    let spec: Vec<Rule> = Rule::ALL
        .into_iter()
        .filter(|r| r.provenance() == Provenance::Spec)
        .collect();
    assert_eq!(
        spec,
        [
            Rule::LevelSet,
            Rule::LevelOrder,
            Rule::MoviesPerLevel,
            Rule::OutroThenLoad,
            Rule::StartInventory
        ]
    );
    let chosen = Rule::ALL
        .into_iter()
        .filter(|r| r.provenance() == Provenance::Chosen)
        .count();
    assert_eq!(chosen, 20);
}

/// The rows of the rules table of `docs/CAMPAIGN.md`: `| `key` | Spec or Chosen | text |`.
fn doc_rows() -> Vec<(String, String)> {
    DOC.lines()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split('|').map(str::trim).collect();
            let key = cols.get(1)?.strip_prefix('`')?.strip_suffix('`')?;
            let prov = *cols.get(2)?;
            matches!(prov, "Spec" | "Chosen").then(|| (key.to_string(), prov.to_string()))
        })
        .collect()
}

#[test]
fn the_documentation_has_one_row_per_rule_with_the_same_provenance() {
    let rows = doc_rows();
    assert_eq!(
        rows.len(),
        Rule::ALL.len(),
        "the doc table and Rule::ALL differ in size"
    );
    for (row, rule) in rows.iter().zip(Rule::ALL) {
        assert_eq!(row.0, rule.key(), "rows are in the order of Rule::ALL");
        let expect = match rule.provenance() {
            Provenance::Spec => "Spec",
            Provenance::Chosen => "Chosen",
        };
        assert_eq!(row.1, expect, "{}", rule.key());
    }
}

#[test]
fn bytes_errors_say_what_is_wrong() {
    let all = [
        (BytesError::TooShort, "short"),
        (BytesError::BadChecksum, "checksum"),
        (BytesError::WrongMagic, "expected kind"),
        (BytesError::UnknownVersion { found: 7 }, "version 7"),
        (BytesError::Truncated, "end too early"),
        (BytesError::Trailing, "left over"),
        (BytesError::Invalid("because"), "because"),
    ];
    for (e, needle) in all {
        assert!(e.to_string().contains(needle), "{e}");
    }
}
