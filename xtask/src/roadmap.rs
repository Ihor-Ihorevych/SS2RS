//! Parses `docs/roadmap/T*.md` into tasks and mirrors them as GitHub issues.
//!
//! The markdown files stay the source of truth; issues are a convenience mirror. The sync is
//! idempotent (matched by task ID) and a dry run unless `--apply` is given.

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{Context, bail};
use regex::Regex;

const MAX_ISSUE_TITLE: usize = 100;

/// One roadmap item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoadmapTask {
    /// Stable ID such as `T02-006`.
    pub id: String,
    /// Short title without markdown markup.
    pub title: String,
    /// `S`, `M`, `L` or `XL`.
    pub size: Option<String>,
    /// `P0`..`P3` (absent for `PHASE 2` tasks).
    pub prio: Option<String>,
    /// Marked `PHASE 2` (not scheduled before M8).
    pub phase2: bool,
    /// Marked as a good first task.
    pub good_first: bool,
    /// A time-boxed decision task.
    pub spike: bool,
    /// A reverse-engineering specification task.
    pub spec: bool,
    /// Claimed (`- [~]` or an `[INPRG ...]` tag).
    pub in_progress: bool,
    /// Finished (`- [x]`).
    pub done: bool,
    /// Task IDs after `Deps:`.
    pub deps: Vec<String>,
    /// The item's markdown (first line plus indented continuation lines).
    pub body: String,
    /// The roadmap file the item came from.
    pub source: String,
}

fn item_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^- \[([ x~])\] \*\*(T\d\d-\d{3}[a-z]?)\*\*(.*)$").unwrap())
}

fn id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"T\d\d-\d{3}[a-z]?").unwrap())
}

fn token_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"`([^`]+)`").unwrap())
}

fn inprg_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s*\[INPRG[^\]]*\]").unwrap())
}

fn spec_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\bSPEC\b").unwrap())
}

fn build(state: char, id: &str, first: &str, continuation: &[&str], source: &str) -> RoadmapTask {
    let full = std::iter::once(first)
        .chain(continuation.iter().copied())
        .collect::<Vec<_>>()
        .join("\n");
    let cleaned = inprg_re().replace_all(first, "");
    let title = cleaned
        .split(" · ")
        .map(|s| s.replace("**", "").replace('`', ""))
        .map(|s| s.trim().to_string())
        .find(|s| !s.is_empty())
        .unwrap_or_default();

    let (mut size, mut prio, mut phase2) = (None, None, false);
    for cap in token_re().captures_iter(first) {
        let token = &cap[1];
        if size.is_none() && matches!(token, "S" | "M" | "L" | "XL") {
            size = Some(token.to_string());
        } else if prio.is_none() && matches!(token, "P0" | "P1" | "P2" | "P3") {
            prio = Some(token.to_string());
        } else if token == "PHASE 2" {
            phase2 = true;
        }
    }

    let deps = full
        .split_once("Deps:")
        .map(|(_, rest)| {
            let segment = rest.split(" · ").next().unwrap_or("");
            let segment = segment.split('\n').next().unwrap_or("");
            id_re()
                .find_iter(segment)
                .map(|m| m.as_str().to_string())
                .collect()
        })
        .unwrap_or_default();

    let mut body = format!("- [{state}] **{id}**{first}");
    for line in continuation {
        body.push('\n');
        body.push_str(line);
    }

    RoadmapTask {
        id: id.to_string(),
        title,
        size,
        prio,
        phase2,
        good_first: full.contains('🟢'),
        spike: full.contains("SPIKE"),
        spec: spec_re().is_match(&full),
        in_progress: state == '~' || full.contains("[INPRG"),
        done: state == 'x',
        deps,
        body,
        source: source.to_string(),
    }
}

/// Parse one roadmap file. An item starts at `- [ ] **Txx-nnn**` and continues over the
/// following lines that are indented by two spaces.
pub fn parse_roadmap(text: &str, source_file: &str) -> Vec<RoadmapTask> {
    let mut tasks = Vec::new();
    let mut current: Option<(char, String, String, Vec<&str>)> = None;
    let flush = |current: &mut Option<(char, String, String, Vec<&str>)>,
                 tasks: &mut Vec<RoadmapTask>| {
        if let Some((state, id, first, rest)) = current.take() {
            tasks.push(build(state, &id, &first, &rest, source_file));
        }
    };
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(cap) = item_re().captures(line) {
            flush(&mut current, &mut tasks);
            let state = cap[1].chars().next().unwrap_or(' ');
            current = Some((state, cap[2].to_string(), cap[3].to_string(), Vec::new()));
        } else if line.starts_with("  ") && !line.trim().is_empty() {
            if let Some((_, _, _, rest)) = current.as_mut() {
                rest.push(line);
            }
        } else {
            flush(&mut current, &mut tasks);
        }
    }
    flush(&mut current, &mut tasks);
    tasks
}

impl RoadmapTask {
    /// `"T02-006: title"`, shortened to at most 100 characters on a word boundary.
    pub fn issue_title(&self) -> String {
        let prefix = format!("{}: ", self.id);
        let full = format!("{prefix}{}", self.title);
        if full.chars().count() <= MAX_ISSUE_TITLE {
            return full;
        }
        let budget = MAX_ISSUE_TITLE - prefix.chars().count() - 1;
        let cut: String = self.title.chars().take(budget).collect();
        let cut = cut
            .rsplit_once(' ')
            .map(|(head, _)| head.to_string())
            .unwrap_or(cut);
        format!("{prefix}{}…", cut.trim_end())
    }

    /// GitHub labels for this task.
    pub fn labels(&self) -> Vec<String> {
        let mut labels = vec![format!("track:{}", &self.id[..3])];
        if let Some(size) = &self.size {
            labels.push(format!("size:{size}"));
        }
        if let Some(prio) = &self.prio {
            labels.push(format!("prio:{prio}"));
        }
        for (on, name) in [
            (self.phase2, "phase-2"),
            (self.good_first, "good-first"),
            (self.spike, "spike"),
            (self.spec, "spec"),
            (self.in_progress, "in-progress"),
        ] {
            if on {
                labels.push(name.to_string());
            }
        }
        labels
    }
}

/// Tasks that still need an issue: not done, and no existing issue title starts with `"<ID>:"`.
pub fn tasks_to_create<'a>(
    tasks: &'a [RoadmapTask],
    existing_issue_titles: &[String],
) -> Vec<&'a RoadmapTask> {
    tasks
        .iter()
        .filter(|t| !t.done)
        .filter(|t| {
            let prefix = format!("{}:", t.id);
            !existing_issue_titles
                .iter()
                .any(|title| title.starts_with(&prefix))
        })
        .collect()
}

/// Load every `T*.md` file in `dir`, in file-name order.
pub fn load_all(dir: &Path) -> anyhow::Result<Vec<RoadmapTask>> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| format!("cannot read {}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('T') && n.ends_with(".md"))
        })
        .collect();
    files.sort();
    let mut all = Vec::new();
    for path in files {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        all.extend(parse_roadmap(&text, &name));
    }
    Ok(all)
}

fn label_color(label: &str) -> &'static str {
    match label {
        "prio:P0" => "b60205",
        "prio:P1" => "d93f0b",
        "prio:P2" => "fbca04",
        "prio:P3" => "0e8a16",
        "phase-2" => "5319e7",
        "good-first" => "7057ff",
        "spike" => "bfdadc",
        "spec" => "bfd4f2",
        "in-progress" => "fef2c0",
        l if l.starts_with("size:") => "c5def5",
        _ => "1d76db",
    }
}

fn gh(args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .context("failed to run the GitHub CLI (gh)")?;
    if !output.status.success() {
        bail!(
            "gh {} failed: {}",
            args.first().copied().unwrap_or(""),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn issue_body(task: &RoadmapTask) -> String {
    format!(
        "Roadmap task `{id}` from `docs/roadmap/{src}`.\n\n{body}\n\n---\nThis issue mirrors the roadmap; \
         the markdown file is the source of truth. Claim with an `[INPRG]` commit on `main` (see AGENTS.md).",
        id = task.id,
        src = task.source,
        body = task.body,
    )
}

/// Print what would happen (dry run) or create the missing labels and issues (`apply`).
pub fn sync(dir: &Path, apply: bool) -> anyhow::Result<()> {
    let tasks = load_all(dir)?;
    let (done, claimed) = (
        tasks.iter().filter(|t| t.done).count(),
        tasks.iter().filter(|t| t.in_progress).count(),
    );
    let existing: Vec<String> = match gh(&[
        "issue",
        "list",
        "--state",
        "all",
        "--limit",
        "2000",
        "--json",
        "title",
        "--jq",
        ".[].title",
    ]) {
        Ok(text) => text.lines().map(str::to_string).collect(),
        Err(e) => {
            eprintln!("warning: could not list existing issues ({e:#}); assuming none");
            Vec::new()
        }
    };
    let missing = tasks_to_create(&tasks, &existing);
    println!(
        "{} tasks ({done} done, {claimed} claimed); {} existing issues; {} to create",
        tasks.len(),
        existing.len(),
        missing.len()
    );
    let mut per_track = std::collections::BTreeMap::<&str, usize>::new();
    for t in &missing {
        *per_track.entry(&t.id[..3]).or_default() += 1;
    }
    for (track, n) in &per_track {
        println!("  {track}: {n}");
    }
    if !apply {
        for t in missing.iter().take(5) {
            println!("  would create: {}", t.issue_title());
        }
        println!("dry run: nothing was created. Re-run with --apply to create issues and labels.");
        return Ok(());
    }
    let mut labels: Vec<String> = missing.iter().flat_map(|t| t.labels()).collect();
    labels.sort();
    labels.dedup();
    for label in &labels {
        gh(&[
            "label",
            "create",
            label,
            "--color",
            label_color(label),
            "--force",
        ])?;
    }
    for (i, task) in missing.iter().enumerate() {
        let mut args = vec![
            "issue".to_string(),
            "create".to_string(),
            "--title".to_string(),
            task.issue_title(),
            "--body".to_string(),
            issue_body(task),
        ];
        for label in task.labels() {
            args.push("--label".to_string());
            args.push(label);
        }
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        gh(&refs)?;
        println!(
            "[{}/{}] created {}",
            i + 1,
            missing.len(),
            task.issue_title()
        );
        std::thread::sleep(std::time::Duration::from_millis(1200)); // stay under GitHub's content-creation rate limit
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const LINE: &str = "- [ ] **T02-006** · CTSEMETA container reader · `M` · `P0` · crate `ss2-meta`\n  Deps: T01-004, T01-005 · Do: read chunks.";

    fn one(text: &str) -> RoadmapTask {
        let tasks = parse_roadmap(text, "T02-data-formats.md");
        assert_eq!(tasks.len(), 1, "{tasks:?}");
        tasks.into_iter().next().unwrap()
    }

    #[test]
    fn parses_single_line_item() {
        let t = one("- [ ] **T02-006** · CTSEMETA container reader · `M` · `P0` · Deps: T01-004");
        assert_eq!(t.id, "T02-006");
        assert_eq!(t.title, "CTSEMETA container reader");
        assert_eq!(
            (t.size.as_deref(), t.prio.as_deref()),
            (Some("M"), Some("P0"))
        );
        assert_eq!(t.deps, vec!["T01-004".to_string()]);
        assert!(!t.phase2 && !t.in_progress && !t.done);
    }

    #[test]
    fn parses_item_with_indented_description() {
        let t = one(LINE);
        assert_eq!(t.title, "CTSEMETA container reader");
        assert!(t.body.contains("Do: read chunks."), "{}", t.body);
        assert_eq!(t.deps, vec!["T01-004".to_string(), "T01-005".to_string()]);
    }

    #[test]
    fn indented_description_stops_at_next_item_or_blank() {
        let text = "- [ ] **T00-001** · A · `S` · `P0`\n  more A\n\n- [ ] **T00-002** · B · `S` · `P0`\n  more B\n";
        let tasks = parse_roadmap(text, "x.md");
        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].body.contains("more A") && !tasks[0].body.contains("more B"));
    }

    #[test]
    fn phase2_has_no_prio_label() {
        let t = one("- [ ] **T03-006** · Android build pipeline · `L` · `PHASE 2`");
        assert!(t.phase2 && t.prio.is_none());
        let labels = t.labels();
        assert!(labels.contains(&"phase-2".to_string()));
        assert!(!labels.iter().any(|l| l.starts_with("prio:")), "{labels:?}");
    }

    #[test]
    fn lettered_sub_ids_inside_text_are_not_tasks() {
        let text = "- [ ] **T09-015** · **Weapon tasks** (`T09-015a…`, each `L` · `P1`)\n";
        let tasks = parse_roadmap(text, "T09-gameplay.md");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "T09-015");
    }

    #[test]
    fn issue_title_and_labels() {
        let t =
            one("- [ ] **T01-004** · SPEC `CTSEMETA` container · `L` · `P0` · Deps: T01-001 · 🟢");
        assert_eq!(t.issue_title(), "T01-004: SPEC CTSEMETA container");
        let labels = t.labels();
        for want in ["track:T01", "size:L", "prio:P0", "spec", "good-first"] {
            assert!(
                labels.contains(&want.to_string()),
                "{want} missing in {labels:?}"
            );
        }
    }

    #[test]
    fn spike_label() {
        let t = one("- [ ] **T07-006** · **SPIKE**: collision engine · `M` · `P0`");
        assert!(t.spike && t.labels().contains(&"spike".to_string()));
    }

    #[test]
    fn claimed_item_is_in_progress() {
        let t = one(
            "- [~] **T02-006** [INPRG @a/agent · branch `x` · 2026-11-22] · CTSEMETA reader · `M` · `P0`",
        );
        assert!(t.in_progress);
        assert_eq!(t.title, "CTSEMETA reader");
        assert!(t.labels().contains(&"in-progress".to_string()));
        assert_eq!(
            (t.size.as_deref(), t.prio.as_deref()),
            (Some("M"), Some("P0"))
        );
    }

    #[test]
    fn done_item_is_marked_done() {
        let t = one("- [x] **T00-001** · Workspace skeleton · `S` · `P0`");
        assert!(t.done);
    }

    #[test]
    fn long_titles_are_truncated_on_a_word_boundary() {
        let long = "word ".repeat(60);
        let t = one(&format!("- [ ] **T00-001** · {long}· `S` · `P0`"));
        assert!(
            t.issue_title().chars().count() <= 100,
            "{}",
            t.issue_title()
        );
        assert!(t.issue_title().ends_with('…'));
    }

    #[test]
    fn second_run_creates_nothing() {
        let tasks = parse_roadmap(
            "- [ ] **T00-001** · A · `S` · `P0`\n- [ ] **T00-002** · B · `S` · `P0`\n",
            "x.md",
        );
        let first: Vec<String> = tasks_to_create(&tasks, &[])
            .iter()
            .map(|t| t.issue_title())
            .collect();
        assert_eq!(first.len(), 2);
        assert!(tasks_to_create(&tasks, &first).is_empty());
    }

    #[test]
    fn existing_issue_is_matched_by_id_prefix_even_if_title_changed() {
        let tasks = parse_roadmap("- [ ] **T00-001** · New title · `S` · `P0`\n", "x.md");
        assert!(tasks_to_create(&tasks, &["T00-001: Old title".to_string()]).is_empty());
        assert_eq!(
            tasks_to_create(&tasks, &["T00-0010: Other".to_string()]).len(),
            1
        );
    }

    #[test]
    fn done_tasks_are_not_created() {
        let tasks = parse_roadmap("- [x] **T00-001** · A · `S` · `P0`\n", "x.md");
        assert!(tasks_to_create(&tasks, &[]).is_empty());
    }

    #[test]
    fn parses_all_real_roadmap_files() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/roadmap");
        let mut all = Vec::new();
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if name.starts_with('T') && name.ends_with(".md") {
                all.extend(parse_roadmap(
                    &std::fs::read_to_string(&path).unwrap(),
                    &name,
                ));
            }
        }
        assert!(all.len() >= 370, "only {} tasks parsed", all.len());
        let ids: HashSet<_> = all.iter().map(|t| t.id.clone()).collect();
        assert_eq!(ids.len(), all.len(), "duplicate task IDs");
        assert!(
            all.iter().all(|t| !t.title.is_empty()),
            "a task has an empty title"
        );
    }
}
