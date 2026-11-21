//! `cargo xtask check-claims` and `cargo xtask claims`: keep the `[INPRG]` claims in
//! `docs/roadmap/T*.md` honest (`AGENTS.md` section 1).
//!
//! - **check-claims** (CI, every PR): fails on contradictory or malformed claim lines.
//! - **claims** (weekly job, or by hand): lists active claims with their age and the last push to
//!   their branch, and which ones are stale (may be pinged) or releasable (rule 8).
//!
//! Only a tag written right after the task ID (`**T02-006** [INPRG @handle · ...]`) counts as a
//! claim. Roadmap text that merely *talks about* the tag, inside backticks, never does.

use std::collections::BTreeMap;
use std::process::Command;
use std::sync::OnceLock;

use regex::Regex;

use crate::roadmap;

/// How serious a finding is. Only errors make `check-claims` fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// Fails CI.
    Error,
    /// Shown, does not fail.
    Warning,
}

/// One problem found in a roadmap file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// Roadmap file name, e.g. `T02-data-formats.md`.
    pub file: String,
    /// 1-based line number.
    pub line: usize,
    /// Task ID.
    pub id: String,
    /// Error or warning.
    pub severity: Severity,
    /// What is wrong.
    pub message: String,
}

/// A well-formed claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim {
    /// Task ID.
    pub id: String,
    /// Roadmap file name.
    pub file: String,
    /// 1-based line number.
    pub line: usize,
    /// `handle` or `handle/agent`, without `@`.
    pub handle: String,
    /// Work branch.
    pub branch: String,
    /// `YYYY-MM-DD` the claim was made.
    pub date: String,
}

/// Result of scanning the roadmap.
#[derive(Debug, Default)]
pub struct Scan {
    /// Everything wrong (and merely suspicious).
    pub findings: Vec<Finding>,
    /// All well-formed claims on `[~]` tasks.
    pub claims: Vec<Claim>,
}

impl Scan {
    /// `true` if any finding is an error.
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Error)
    }
}

fn item_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^- \[([ x~])\] \*\*(T\d\d-\d{3}[a-z]?)\*\*(.*)$").unwrap())
}

fn code_span_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"`[^`]*`").unwrap())
}

fn tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^\[INPRG @([A-Za-z0-9._-]+(?:/[A-Za-z0-9._-]+)?) · branch `([^`]+)` · (\d{4}-\d{2}-\d{2})\]$",
        )
        .unwrap()
    })
}

/// Days since 1970-01-01 for a civil date (inverse of `claim::date_from_unix_days`).
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Parse a real calendar date `YYYY-MM-DD` into days since the epoch; `None` if it is not one.
pub fn parse_date(text: &str) -> Option<i64> {
    let mut parts = text.split('-');
    let (y, m, d) = (
        parts.next()?.parse::<i64>().ok()?,
        parts.next()?.parse::<i64>().ok()?,
        parts.next()?.parse::<i64>().ok()?,
    );
    if parts.next().is_some() || text.len() != 10 || !(1..=12).contains(&m) {
        return None;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days_in_month = match m {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days_in_month)
        .contains(&d)
        .then(|| days_from_civil(y, m, d))
}

/// Everything in `rest` that is not inside a backtick code span.
fn without_code(rest: &str) -> String {
    code_span_re().replace_all(rest, "").into_owned()
}

/// Split `[INPRG ...]` off the start of `rest` (after optional spaces); `None` if there is none.
fn leading_tag(rest: &str) -> Option<&str> {
    let t = rest.trim_start();
    if !t.starts_with("[INPRG") {
        return None;
    }
    t.find(']').map(|end| &t[..=end])
}

/// Scan roadmap files given as `(file name, text)` pairs. `today` is `YYYY-MM-DD` (claims dated
/// after tomorrow are errors; one day of slack covers time zones).
pub fn scan(files: &[(String, String)], today: &str) -> Scan {
    let today_days = parse_date(today);
    let mut out = Scan::default();
    let mut ids: BTreeMap<String, (String, usize)> = BTreeMap::new();

    for (file, text) in files {
        let mut current: Option<String> = None;
        for (index, raw) in text.lines().enumerate() {
            let line = raw.trim_end_matches('\r');
            let number = index + 1;
            let mut push = |id: &str, severity, message: String| {
                out.findings.push(Finding {
                    file: file.clone(),
                    line: number,
                    id: id.to_owned(),
                    severity,
                    message,
                });
            };

            let Some(caps) = item_re().captures(line) else {
                // Continuation lines (two-space indent) belong to the current item.
                if line.starts_with("  ") && !line.trim().is_empty() {
                    if let Some(id) = &current
                        && without_code(line).contains("[INPRG")
                    {
                        push(
                            id,
                            Severity::Error,
                            "an [INPRG] tag in the description lines: a claim tag belongs right after the task ID, once".into(),
                        );
                    }
                } else {
                    current = None;
                }
                continue;
            };
            let state = &caps[1];
            let id = caps[2].to_owned();
            let rest = &caps[3];
            current = Some(id.clone());

            if let Some((first_file, first_line)) = ids.get(&id) {
                push(
                    &id,
                    Severity::Error,
                    format!(
                        "duplicate task ID (first defined at {first_file}:{first_line}); IDs are the key for the issue sync"
                    ),
                );
            } else {
                ids.insert(id.clone(), (file.clone(), number));
            }

            let anchored = leading_tag(rest);
            let mentions = without_code(rest).matches("[INPRG").count();
            if mentions > 1 {
                push(
                    &id,
                    Severity::Error,
                    format!("{mentions} [INPRG] tags on one task: exactly one claim at a time"),
                );
            }

            match (state, anchored) {
                ("x", Some(_)) => push(
                    &id,
                    Severity::Error,
                    "marked done [x] but still carries an [INPRG] tag: remove the tag when finishing (AGENTS.md rule 7)".into(),
                ),
                ("x", None) if mentions > 0 => push(
                    &id,
                    Severity::Error,
                    "marked done [x] but its text still contains an [INPRG] tag".into(),
                ),
                (" ", _) if mentions > 0 => push(
                    &id,
                    Severity::Error,
                    "has an [INPRG] tag but is not marked claimed [~]".into(),
                ),
                ("~", None) => push(
                    &id,
                    Severity::Error,
                    "marked claimed [~] but has no `[INPRG @handle · branch `...` · date]` tag right after the ID".into(),
                ),
                ("~", Some(tag)) => match tag_re().captures(tag) {
                    None => push(
                        &id,
                        Severity::Error,
                        format!("malformed claim tag {tag}: expected `[INPRG @handle · branch `name` · YYYY-MM-DD]` (handle or handle/agent, no emails)"),
                    ),
                    Some(t) => {
                        let date = t[3].to_owned();
                        match (parse_date(&date), today_days) {
                            (None, _) => push(
                                &id,
                                Severity::Error,
                                format!("claim date {date} is not a real calendar date"),
                            ),
                            (Some(d), Some(now)) if d > now + 1 => push(
                                &id,
                                Severity::Error,
                                format!("claim date {date} is in the future"),
                            ),
                            _ => {}
                        }
                        out.claims.push(Claim {
                            id: id.clone(),
                            file: file.clone(),
                            line: number,
                            handle: t[1].to_owned(),
                            branch: t[2].to_owned(),
                            date,
                        });
                    }
                },
                _ => {}
            }
        }
    }

    dependency_warnings(files, &mut out);
    out.findings
        .sort_by(|a, b| (&a.file, a.line, &a.message).cmp(&(&b.file, b.line, &b.message)));
    out
}

/// A claimed task whose dependencies are not finished is suspicious, not wrong: `AGENTS.md`
/// allows claiming against merged stubs/mocks, so this is a **warning**. A dependency that is
/// itself claimed by the same handle on the same branch is one piece of work (a split task done in
/// order) and is not reported.
fn dependency_warnings(files: &[(String, String)], out: &mut Scan) {
    let mut tasks = Vec::new();
    for (file, text) in files {
        tasks.extend(roadmap::parse_roadmap(text, file));
    }
    let claim_of = |id: &str| out.claims.iter().find(|c| c.id == id).cloned();
    let mut extra = Vec::new();
    for claim in &out.claims {
        let Some(task) = tasks.iter().find(|t| t.id == claim.id) else {
            continue;
        };
        for dep in &task.deps {
            let done = tasks.iter().any(|t| &t.id == dep && t.done);
            if done {
                continue;
            }
            let same_work =
                claim_of(dep).is_some_and(|d| d.handle == claim.handle && d.branch == claim.branch);
            if !same_work {
                extra.push(Finding {
                    file: claim.file.clone(),
                    line: claim.line,
                    id: claim.id.clone(),
                    severity: Severity::Warning,
                    message: format!(
                        "claimed while its dependency {dep} is not done (fine only if merged stubs/mocks exist, AGENTS.md rule 2)"
                    ),
                });
            }
        }
    }
    out.findings.extend(extra);
}

/// Render findings, one per line, as `file:line: severity: ID: message`. With `github`, also emits
/// workflow annotations (`::error file=...`) so they show up on the pull request.
pub fn render_findings(findings: &[Finding], github: bool) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for f in findings {
        let kind = match f.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let path = format!("docs/roadmap/{}", f.file);
        if github {
            let _ = writeln!(
                out,
                "::{kind} file={path},line={}::{}: {}",
                f.line, f.id, f.message
            );
        } else {
            let _ = writeln!(out, "{path}:{}: {kind}: {}: {}", f.line, f.id, f.message);
        }
    }
    out
}

/// Read every `docs/roadmap/T*.md` under `repo`.
pub fn read_roadmap(repo: &std::path::Path) -> anyhow::Result<Vec<(String, String)>> {
    use anyhow::Context as _;
    let dir = repo.join("docs/roadmap");
    let mut files = Vec::new();
    for entry in
        std::fs::read_dir(&dir).with_context(|| format!("cannot read {}", dir.display()))?
    {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        if name.starts_with('T') && name.ends_with(".md") {
            files.push((name, std::fs::read_to_string(&path)?));
        }
    }
    files.sort();
    Ok(files)
}

// ---------------------------------------------------------------------------------------------
// stale report

/// Where a claim stands under `AGENTS.md` rule 8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// Recent push (or recently claimed).
    Active,
    /// No push for `stale_days` or more: may be pinged.
    Stale,
    /// Stale and 48 hours past that: anyone may release it with a `[RELEASED] ... stale` commit.
    Releasable,
}

/// Standing of a claim made on `claimed`, whose branch was last pushed on `last_push` (if it
/// exists), judged on `today`. Idle time runs from the later of the two dates.
pub fn standing(
    claimed: &str,
    last_push: Option<&str>,
    today: &str,
    stale_days: i64,
) -> (Standing, i64) {
    let (Some(c), Some(now)) = (parse_date(claimed), parse_date(today)) else {
        return (Standing::Active, 0);
    };
    let latest = last_push.and_then(parse_date).map_or(c, |p| p.max(c));
    let idle = (now - latest).max(0);
    let standing = if idle >= stale_days + 2 {
        Standing::Releasable
    } else if idle >= stale_days {
        Standing::Stale
    } else {
        Standing::Active
    };
    (standing, idle)
}

/// Finds the date (`YYYY-MM-DD`) of the newest commit on a branch.
pub trait LastPush {
    /// `None` if the branch does not exist or cannot be queried.
    fn last_push(&self, branch: &str) -> Option<String>;
}

/// Asks GitHub through `gh` (for the repository in the current directory).
pub struct GhLastPush;

impl LastPush for GhLastPush {
    fn last_push(&self, branch: &str) -> Option<String> {
        let out = Command::new("gh")
            .args([
                "api",
                &format!("repos/{{owner}}/{{repo}}/commits/{branch}"),
                "--jq",
                ".commit.committer.date",
            ])
            .output()
            .ok()?;
        out.status.success().then(|| {
            String::from_utf8_lossy(&out.stdout)
                .trim()
                .chars()
                .take(10)
                .collect()
        })
    }
}

/// No lookups: idle time is simply the age of the claim.
pub struct NoLastPush;

impl LastPush for NoLastPush {
    fn last_push(&self, _branch: &str) -> Option<String> {
        None
    }
}

/// A markdown report of all claims, oldest idle first.
pub fn report(claims: &[Claim], today: &str, stale_days: i64, pushes: &dyn LastPush) -> String {
    use std::fmt::Write as _;
    let mut rows: Vec<(i64, String)> = claims
        .iter()
        .map(|c| {
            let last = pushes.last_push(&c.branch);
            let (st, idle) = standing(&c.date, last.as_deref(), today, stale_days);
            let label = match st {
                Standing::Active => "active",
                Standing::Stale => "STALE: may be pinged",
                Standing::Releasable => {
                    "RELEASABLE: pinged + 48 h passed (`cargo xtask release --force`)"
                }
            };
            let pushed = last.unwrap_or_else(|| "no branch yet".to_owned());
            (
                idle,
                format!(
                    "| {} | @{} | `{}` | {} | {} | {idle} | {label} |",
                    c.id, c.handle, c.branch, c.date, pushed
                ),
            )
        })
        .collect();
    rows.sort_by_key(|row| std::cmp::Reverse(row.0));

    let mut out = format!("## Claims on {today}\n\n");
    if rows.is_empty() {
        out.push_str("No task is claimed right now.\n");
        return out;
    }
    let _ = writeln!(
        out,
        "A claim is stale after {stale_days} days without a push to its branch (or since it was made).\n"
    );
    out.push_str("| Task | Claimed by | Branch | Claimed | Last push | Idle days | Standing |\n|---|---|---|---|---|---|---|\n");
    for (_, row) in rows {
        out.push_str(&row);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(text: &str) -> Scan {
        scan(&[("T99-test.md".to_owned(), text.to_owned())], "2026-11-21")
    }

    fn errors(scan: &Scan) -> Vec<String> {
        scan.findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .map(|f| format!("{}: {}", f.id, f.message))
            .collect()
    }

    const OK_CLAIM: &str =
        "- [~] **T99-002** [INPRG @alice · branch `T99-002-x` · 2026-11-20] · Claimed · `S` · `P1`";

    #[test]
    fn a_clean_roadmap_has_no_findings() {
        let s = one(&format!(
            "# T99\n- [x] **T99-001** · Done · `S` · `P0`\n{OK_CLAIM}\n- [ ] **T99-003** · Open · `S` · `P1`\n"
        ));
        assert!(s.findings.is_empty(), "{:?}", s.findings);
        assert_eq!(s.claims.len(), 1);
        assert_eq!(
            (
                s.claims[0].handle.as_str(),
                s.claims[0].branch.as_str(),
                s.claims[0].date.as_str()
            ),
            ("alice", "T99-002-x", "2026-11-20")
        );
    }

    #[test]
    fn text_that_only_talks_about_the_tag_is_not_a_claim() {
        // This is the real T00-028 line: done, and its description mentions `[INPRG]` in code.
        let s = one(
            "- [x] **T99-001** · automates the `[INPRG]` protocol and writes `[INPRG @h · branch `b` · d]` · `M` · `P0`\n",
        );
        assert!(s.findings.is_empty(), "{:?}", s.findings);
        let s = one("- [ ] **T99-001** · open task that says `[INPRG]` in code · `S` · `P0`\n");
        assert!(s.findings.is_empty(), "{:?}", s.findings);
    }

    #[test]
    fn done_but_still_tagged_is_an_error() {
        let s =
            one("- [x] **T99-001** [INPRG @alice · branch `b` · 2026-11-20] · Done · `S` · `P0`\n");
        assert_eq!(errors(&s).len(), 1);
        assert!(
            errors(&s)[0].contains("done [x] but still carries"),
            "{:?}",
            errors(&s)
        );
        assert!(s.has_errors());
    }

    #[test]
    fn tag_without_marker_and_marker_without_tag() {
        let s =
            one("- [ ] **T99-001** [INPRG @alice · branch `b` · 2026-11-20] · Open · `S` · `P0`\n");
        assert!(
            errors(&s)[0].contains("not marked claimed"),
            "{:?}",
            errors(&s)
        );
        let s = one("- [~] **T99-001** · Claimed with no tag · `S` · `P0`\n");
        assert!(errors(&s)[0].contains("has no `[INPRG"), "{:?}", errors(&s));
    }

    #[test]
    fn a_tag_in_the_middle_of_an_open_task_is_an_error() {
        let s = one(
            "- [ ] **T99-001** · Open [INPRG @a · branch `b` · 2026-11-20] hidden · `S` · `P0`\n",
        );
        assert!(
            errors(&s)[0].contains("not marked claimed"),
            "{:?}",
            errors(&s)
        );
    }

    #[test]
    fn malformed_tags_are_reported_specifically() {
        for (tag, what) in [
            ("[INPRG]", "malformed"),
            ("[INPRG @alice]", "malformed"),
            ("[INPRG @alice · branch `b`]", "malformed"),
            ("[INPRG alice · branch `b` · 2026-11-20]", "malformed"),
            ("[INPRG @a@b.com · branch `b` · 2026-11-20]", "malformed"),
            ("[INPRG @alice · branch `` · 2026-11-20]", "malformed"),
            ("[INPRG @alice · branch `b` · 20-11-2026]", "malformed"),
            (
                "[INPRG @alice · branch `b` · 2026-02-30]",
                "not a real calendar date",
            ),
            (
                "[INPRG @alice · branch `b` · 2026-13-01]",
                "not a real calendar date",
            ),
            ("[INPRG @alice · branch `b` · 2027-01-01]", "in the future"),
        ] {
            let s = one(&format!("- [~] **T99-001** {tag} · Task · `S` · `P0`\n"));
            let e = errors(&s);
            assert_eq!(e.len(), 1, "{tag}: {e:?}");
            assert!(e[0].contains(what), "{tag}: {e:?}");
        }
    }

    #[test]
    fn leap_days_and_slack_for_time_zones() {
        let claim = |date: &str, today: &str| {
            let text =
                format!("- [~] **T99-001** [INPRG @a · branch `b` · {date}] · T · `S` · `P0`\n");
            errors(&scan(&[("T.md".into(), text)], today))
        };
        assert!(
            claim("2028-02-29", "2028-03-01").is_empty(),
            "a real leap day"
        );
        assert!(claim("2028-02-29", "2026-11-21")[0].contains("in the future"));
        assert!(
            claim("2026-11-22", "2026-11-21").is_empty(),
            "one day of slack for time zones"
        );
        assert!(claim("2026-11-23", "2026-11-21")[0].contains("in the future"));
    }

    #[test]
    fn two_tags_on_one_task_are_an_error() {
        let s = one(
            "- [~] **T99-001** [INPRG @a · branch `b` · 2026-11-20] · again [INPRG @c · branch `d` · 2026-11-20] · `S` · `P0`\n",
        );
        assert!(
            errors(&s).iter().any(|e| e.contains("2 [INPRG] tags")),
            "{:?}",
            errors(&s)
        );
        let s = one(
            "- [~] **T99-001** [INPRG @a · branch `b` · 2026-11-20] · T · `S` · `P0`\n  Do: a second [INPRG @x · branch `y` · 2026-11-20] hides here\n",
        );
        assert!(
            errors(&s).iter().any(|e| e.contains("description lines")),
            "{:?}",
            errors(&s)
        );
        // But a mention inside a code span on a continuation line is fine.
        let s = one(
            "- [~] **T99-001** [INPRG @a · branch `b` · 2026-11-20] · T · `S` · `P0`\n  Done when: the `[INPRG @h · branch `b` · d]` format is documented\n",
        );
        assert!(errors(&s).is_empty(), "{:?}", errors(&s));
    }

    #[test]
    fn duplicate_ids_across_files_are_an_error_naming_the_first() {
        let files = [
            (
                "T00-a.md".to_owned(),
                "- [ ] **T00-032** · One · `S` · `P0`\n".to_owned(),
            ),
            (
                "T00-b.md".to_owned(),
                "# x\n- [x] **T00-032** · Two · `S` · `P0`\n".to_owned(),
            ),
        ];
        let s = scan(&files, "2026-11-21");
        let e = errors(&s);
        assert_eq!(e.len(), 1, "{e:?}");
        assert!(
            e[0].contains("duplicate task ID") && e[0].contains("T00-a.md:1"),
            "{e:?}"
        );
        assert_eq!(
            (s.findings[0].file.as_str(), s.findings[0].line),
            ("T00-b.md", 2)
        );
    }

    #[test]
    fn open_dependencies_warn_but_never_fail() {
        let s = one("- [ ] **T99-001** · Open dep · `S` · `P0`\n\
- [~] **T99-002** [INPRG @alice · branch `b` · 2026-11-20] · Claimed · `S` · `P1` · Deps: T99-001\n");
        assert!(!s.has_errors(), "{:?}", s.findings);
        let w: Vec<_> = s
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .collect();
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(w[0].message.contains("T99-001 is not done"), "{w:?}");
    }

    #[test]
    fn a_dependency_in_the_same_claimants_work_is_not_reported() {
        // Real shape: T01-050b/c depend on T01-050a, all claimed by the same agent on one branch.
        let text = "\
- [~] **T99-001a** [INPRG @a/agent · branch `re` · 2026-11-20] · First · `S` · `P0`\n\
- [~] **T99-001b** [INPRG @a/agent · branch `re` · 2026-11-20] · Second · `S` · `P0` · Deps: T99-001a\n\
- [~] **T99-001c** [INPRG @other · branch `elsewhere` · 2026-11-20] · Third · `S` · `P0` · Deps: T99-001a\n";
        let s = one(text);
        let w: Vec<_> = s
            .findings
            .iter()
            .map(|f| (f.id.as_str(), f.severity))
            .collect();
        assert_eq!(w, [("T99-001c", Severity::Warning)], "{:?}", s.findings);
    }

    #[test]
    fn done_dependencies_are_fine() {
        let s = one("- [x] **T99-001** · Dep · `S` · `P0`\n\
- [~] **T99-002** [INPRG @alice · branch `b` · 2026-11-20] · Claimed · `S` · `P1` · Deps: T99-001\n");
        assert!(s.findings.is_empty(), "{:?}", s.findings);
    }

    #[test]
    fn rendering_plain_and_github_annotations() {
        let s = one("- [~] **T99-001** · No tag · `S` · `P0`\n");
        let plain = render_findings(&s.findings, false);
        assert!(
            plain.starts_with("docs/roadmap/T99-test.md:1: error: T99-001: marked claimed"),
            "{plain}"
        );
        let gh = render_findings(&s.findings, true);
        assert!(
            gh.starts_with("::error file=docs/roadmap/T99-test.md,line=1::T99-001:"),
            "{gh}"
        );
    }

    #[test]
    fn dates_round_trip_with_the_claim_module() {
        for days in [-1, 0, 11_016, 20_088, 20_778, 25_000] {
            let text = crate::claim::date_from_unix_days(days);
            assert_eq!(parse_date(&text), Some(days), "{text}");
        }
        assert_eq!(parse_date("2026-11-21"), Some(20_778));
        for bad in [
            "",
            "2026-11",
            "2026-11-21-01",
            "26-11-21",
            "2026-00-10",
            "2026-02-29",
            "abcd-ef-gh",
            "2026-1-21",
        ] {
            assert_eq!(parse_date(bad), None, "{bad:?}");
        }
        assert!(parse_date("2028-02-29").is_some());
    }

    #[test]
    fn standing_follows_the_rule_7_days_then_48_hours() {
        let st = |claimed: &str, push: Option<&str>| standing(claimed, push, "2026-11-21", 7);
        assert_eq!(st("2026-11-21", None), (Standing::Active, 0));
        assert_eq!(st("2026-11-15", None), (Standing::Active, 6));
        assert_eq!(st("2026-11-14", None), (Standing::Stale, 7));
        assert_eq!(st("2026-11-13", None), (Standing::Stale, 8));
        assert_eq!(st("2026-11-12", None), (Standing::Releasable, 9));
        // A recent push rescues an old claim; an old push does not rescue a new... it cannot be older.
        assert_eq!(st("2026-10-01", Some("2026-11-20")), (Standing::Active, 1));
        assert_eq!(
            st("2026-11-20", Some("2026-10-01")),
            (Standing::Active, 1),
            "claim date wins if later"
        );
        assert_eq!(
            st("garbage", None).0,
            Standing::Active,
            "never accuse on bad data"
        );
        assert_eq!(
            st("2026-11-25", None),
            (Standing::Active, 0),
            "future date: idle is never negative"
        );
    }

    struct Fake(BTreeMap<&'static str, &'static str>);

    impl LastPush for Fake {
        fn last_push(&self, branch: &str) -> Option<String> {
            self.0.get(branch).map(|d| (*d).to_owned())
        }
    }

    fn claim(id: &str, branch: &str, date: &str) -> Claim {
        Claim {
            id: id.into(),
            file: "T99.md".into(),
            line: 1,
            handle: "alice".into(),
            branch: branch.into(),
            date: date.into(),
        }
    }

    #[test]
    fn report_sorts_by_idle_time_and_labels_each_row() {
        let claims = [
            claim("T99-001", "fresh", "2026-11-20"),
            claim("T99-002", "old-but-pushed", "2026-11-01"),
            claim("T99-003", "ghost", "2026-11-05"),
            claim("T99-004", "quiet", "2026-11-13"),
        ];
        let pushes = Fake([("old-but-pushed", "2026-11-19"), ("quiet", "2026-11-14")].into());
        let text = report(&claims, "2026-11-21", 7, &pushes);
        let order: Vec<&str> = text
            .lines()
            .filter(|l| l.starts_with("| T99"))
            .map(|l| &l[2..9])
            .collect();
        assert_eq!(
            order,
            ["T99-003", "T99-004", "T99-002", "T99-001"],
            "{text}"
        );
        assert!(
            text.contains(
                "| T99-003 | @alice | `ghost` | 2026-11-05 | no branch yet | 16 | RELEASABLE"
            ),
            "{text}"
        );
        assert!(
            text.contains("| T99-004 | @alice | `quiet` | 2026-11-13 | 2026-11-14 | 7 | STALE"),
            "{text}"
        );
        assert!(
            text.contains(
                "| T99-002 | @alice | `old-but-pushed` | 2026-11-01 | 2026-11-19 | 2 | active"
            ),
            "{text}"
        );
        assert!(text.contains("## Claims on 2026-11-21"));
    }

    #[test]
    fn report_for_no_claims_says_so() {
        let text = report(&[], "2026-11-21", 7, &NoLastPush);
        assert!(text.contains("No task is claimed right now."), "{text}");
        assert!(!text.contains('|'));
    }

    #[test]
    fn without_lookups_idle_time_is_the_age_of_the_claim() {
        let text = report(
            &[claim("T99-001", "b", "2026-11-10")],
            "2026-11-21",
            7,
            &NoLastPush,
        );
        assert!(text.contains("| no branch yet | 11 | RELEASABLE"), "{text}");
    }
}
