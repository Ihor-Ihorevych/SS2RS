//! `cargo xtask claim <ID>` and `cargo xtask release <ID> --reason ...`: the `[INPRG]` protocol of
//! `AGENTS.md` section 1, automated.
//!
//! A claim is a one-line edit of the task in `docs/roadmap/T*.md`, committed (signed off) and
//! pushed straight to `main`. If the push is rejected somebody else pushed first: this tool drops
//! its own commit, re-reads the roadmap, and either retries (the task is still free) or reports
//! that the task is no longer available. **It never overwrites another claim.**
//!
//! The planning and line-editing functions are pure and unit-tested; the git steps run against
//! real temporary repositories (with a bare "remote") in the tests.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{Context, anyhow, bail};
use regex::Regex;

use crate::roadmap::{self, RoadmapTask};

/// The task cannot be claimed or released right now (already taken, blocked, finished, ...).
/// `main` turns this into exit status 3, distinct from real errors (2), so scripts can pick another
/// task instead of treating it as a failure.
#[derive(Debug)]
pub struct NotAvailable(pub String);

impl std::fmt::Display for NotAvailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for NotAvailable {}

fn unavailable<T>(msg: impl Into<String>) -> anyhow::Result<T> {
    Err(NotAvailable(msg.into()).into())
}

/// How many times a rejected push is retried (each retry re-reads the roadmap first).
const MAX_ATTEMPTS: usize = 3;

/// Where and how to talk to git.
#[derive(Clone, Debug)]
pub struct Ctx {
    /// Repository working tree.
    pub repo: PathBuf,
    /// Remote name (`origin`).
    pub remote: String,
    /// Branch claims are committed to (`main`).
    pub branch: String,
}

impl Ctx {
    /// `origin` / `main` in the repository at `repo`.
    pub fn new(repo: impl Into<PathBuf>) -> Self {
        Self {
            repo: repo.into(),
            remote: "origin".into(),
            branch: "main".into(),
        }
    }
}

/// A request to claim a task.
#[derive(Clone, Debug)]
pub struct ClaimReq {
    /// Task ID such as `T02-006`.
    pub id: String,
    /// GitHub handle (`name` or `name/agent`), without `@` and without any personal data.
    pub who: String,
    /// Work branch name; derived from the ID and title when `None`.
    pub branch: Option<String>,
    /// `YYYY-MM-DD` written into the tag.
    pub date: String,
    /// Show the edit but change nothing.
    pub dry_run: bool,
    /// Claim even though dependencies are not ticked (use only with merged stubs/mocks).
    pub allow_open_deps: bool,
}

/// A request to give a claim back.
#[derive(Clone, Debug)]
pub struct ReleaseReq {
    /// Task ID.
    pub id: String,
    /// Who is releasing.
    pub who: String,
    /// Why (goes into the commit message).
    pub reason: String,
    /// Release a claim that belongs to somebody else (stale claims, AGENTS.md rule 8).
    pub force: bool,
    /// Show the edit but change nothing.
    pub dry_run: bool,
}

/// What happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Done {
    /// The task line after the change.
    pub line: String,
    /// Commit message used.
    pub message: String,
    /// `true` if it was only previewed.
    pub dry_run: bool,
    /// The work branch to create (claims only).
    pub branch: Option<String>,
}

fn id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^T\d\d-\d{3}[a-z]?$").unwrap())
}

fn handle_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[A-Za-z0-9._-]+(/[A-Za-z0-9._-]+)?$").unwrap())
}

fn tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[INPRG @([^ \]]+)[^\]]*\]").unwrap())
}

/// Validate and normalize a handle: strips one leading `@`; rejects anything that looks like an
/// email or other personal data (the protocol allows a GitHub handle or `handle/agent` only).
pub fn normalize_handle(raw: &str) -> anyhow::Result<String> {
    let handle = raw.trim().strip_prefix('@').unwrap_or(raw.trim());
    if handle_re().is_match(handle) {
        Ok(handle.to_owned())
    } else {
        bail!(
            "{raw:?} is not a GitHub handle (use `name` or `name/agent`; no emails or other personal data)"
        )
    }
}

/// Validate a task ID.
pub fn check_id(id: &str) -> anyhow::Result<()> {
    if id_re().is_match(id) {
        Ok(())
    } else {
        bail!("{id:?} is not a task ID like T02-006")
    }
}

/// `T00-028` + title `"cargo xtask claim <ID> ..."` becomes `T00-028-cargo-xtask-claim-id`.
pub fn branch_name(id: &str, title: &str) -> String {
    let words: Vec<String> = title
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .take(4)
        .map(str::to_ascii_lowercase)
        .collect();
    let mut name = if words.is_empty() {
        id.to_owned()
    } else {
        format!("{id}-{}", words.join("-"))
    };
    name.truncate(60);
    name.trim_end_matches('-').to_owned()
}

/// Today's date in UTC as `YYYY-MM-DD`.
pub fn today_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    date_from_unix_days(i64::try_from(secs / 86_400).unwrap_or(0))
}

/// Civil date for a day count since 1970-01-01 (Howard Hinnant's algorithm).
pub fn date_from_unix_days(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// `- [ ] **ID** rest` becomes `- [~] **ID** [INPRG @who · branch `b` · date] rest`.
pub fn claim_line(line: &str, id: &str, who: &str, branch: &str, date: &str) -> Option<String> {
    let rest = line.strip_prefix(&format!("- [ ] **{id}**"))?;
    Some(format!(
        "- [~] **{id}** [INPRG @{who} · branch `{branch}` · {date}]{rest}"
    ))
}

/// The inverse of [`claim_line`]: returns the restored line and the handle that held the claim.
pub fn release_line(line: &str, id: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix(&format!("- [~] **{id}**"))?;
    let caps = tag_re().captures(rest)?;
    let whole = caps.get(0)?;
    let owner = caps.get(1)?.as_str().to_owned();
    let restored = format!(
        "- [ ] **{id}**{}{}",
        rest[..whole.start()].trim_end_matches(' '),
        &rest[whole.end()..]
    );
    Some((restored, owner))
}

/// Replace line `index` of `text` with `new_line`, keeping that line's own ending (`\n`, `\r\n`
/// or none) and every other byte of the file untouched.
pub fn replace_line(text: &str, index: usize, new_line: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len() + new_line.len());
    let mut found = false;
    for (i, raw) in text.split_inclusive('\n').enumerate() {
        if i == index {
            let content = raw.trim_end_matches(['\r', '\n']);
            out.push_str(new_line);
            out.push_str(&raw[content.len()..]);
            found = true;
        } else {
            out.push_str(raw);
        }
    }
    found.then_some(out)
}

/// Why a claim is not possible, or `None` if it is.
pub fn claim_blocker(
    task: &RoadmapTask,
    all: &[RoadmapTask],
    allow_open_deps: bool,
) -> Option<String> {
    if task.done {
        return Some(format!("{} is already done", task.id));
    }
    if task.in_progress {
        let first = task.body.lines().next().unwrap_or("");
        let owner = tag_re()
            .captures(first)
            .map_or_else(|| "someone".to_owned(), |c| format!("@{}", &c[1]));
        return Some(format!("{} is already claimed by {owner}", task.id));
    }
    if task.phase2 {
        return Some(format!(
            "{} is PHASE 2: not scheduled until milestone M8",
            task.id
        ));
    }
    if task.size.as_deref() == Some("XL") {
        return Some(format!(
            "{} is XL: split it first (append new IDs, never renumber)",
            task.id
        ));
    }
    if !allow_open_deps {
        let open: Vec<String> = task
            .deps
            .iter()
            .filter(|d| !all.iter().any(|t| &t.id == *d && t.done))
            .cloned()
            .collect();
        if !open.is_empty() {
            return Some(format!(
                "{} depends on unfinished {} (use --allow-open-deps only if merged stubs/mocks exist)",
                task.id,
                open.join(", ")
            ));
        }
    }
    None
}

/// A task line found on disk.
struct Located {
    path: PathBuf,
    text: String,
    line_index: usize,
    line: String,
}

fn locate(repo: &Path, id: &str) -> anyhow::Result<Located> {
    let dir = repo.join("docs/roadmap");
    let marker = format!("**{id}**");
    let mut hits = Vec::new();
    for entry in
        std::fs::read_dir(&dir).with_context(|| format!("cannot read {}", dir.display()))?
    {
        let path = entry?.path();
        let is_track = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('T') && n.ends_with(".md"));
        if !is_track {
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        for (i, raw) in text.split_inclusive('\n').enumerate() {
            let line = raw.trim_end_matches(['\r', '\n']);
            // A task item looks like `- [ ] **ID** ...`: the marker sits right after the checkbox.
            if line.starts_with("- [") && line.find(&marker) == Some(6) {
                hits.push(Located {
                    path: path.clone(),
                    text: text.clone(),
                    line_index: i,
                    line: line.to_owned(),
                });
            }
        }
    }
    match hits.len() {
        0 => unavailable(format!("{id} does not exist in docs/roadmap/")),
        1 => Ok(hits.remove(0)),
        n => bail!("{id} appears on {n} roadmap lines: fix the duplicate ID first"),
    }
}

// ---------------------------------------------------------------------------------------------
// git

fn git(repo: &Path, args: &[&str]) -> anyhow::Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .context("failed to run git")?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Must be on the claims branch, with no uncommitted tracked changes and nothing unpushed.
fn preconditions(ctx: &Ctx) -> anyhow::Result<()> {
    let current = git(&ctx.repo, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if current.trim() != ctx.branch {
        bail!(
            "you are on `{}`; claims are committed directly to `{}`. Run `git switch {}` first (your work branch is untouched)",
            current.trim(),
            ctx.branch,
            ctx.branch
        );
    }
    if !git(
        &ctx.repo,
        &["status", "--porcelain", "--untracked-files=no"],
    )?
    .trim()
    .is_empty()
    {
        bail!("you have uncommitted changes to tracked files; commit or stash them first");
    }
    git(&ctx.repo, &["fetch", &ctx.remote])?;
    let remote_ref = format!("{}/{}", ctx.remote, ctx.branch);
    let ahead = git(
        &ctx.repo,
        &["rev-list", "--count", &format!("{remote_ref}..HEAD")],
    )?;
    if ahead.trim() != "0" {
        bail!(
            "local `{}` has {} unpushed commit(s); push or move them first, a claim must be the only thing pushed",
            ctx.branch,
            ahead.trim()
        );
    }
    sync(ctx)
}

/// Fetch and fast-forward to the remote branch (the local branch has nothing of its own).
fn sync(ctx: &Ctx) -> anyhow::Result<()> {
    git(&ctx.repo, &["fetch", &ctx.remote])?;
    git(
        &ctx.repo,
        &[
            "merge",
            "--ff-only",
            &format!("{}/{}", ctx.remote, ctx.branch),
        ],
    )?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum Attempt {
    Pushed,
    /// Another push got there first; our commit has been dropped again.
    Rejected,
}

/// Write `new_text`, commit it signed off, push. On a rejected push the commit is dropped
/// (`reset --hard HEAD~1`, safe: `preconditions` guaranteed a clean tree and nothing unpushed).
fn commit_and_push(
    ctx: &Ctx,
    loc: &Located,
    new_text: &str,
    message: &str,
) -> anyhow::Result<Attempt> {
    std::fs::write(&loc.path, new_text)
        .with_context(|| format!("cannot write {}", loc.path.display()))?;
    let relative = loc.path.strip_prefix(&ctx.repo).unwrap_or(&loc.path);
    let relative = relative.to_string_lossy().replace('\\', "/");
    git(&ctx.repo, &["add", &relative])?;
    git(&ctx.repo, &["commit", "-q", "-s", "-m", message])?;
    match git(&ctx.repo, &["push", &ctx.remote, &ctx.branch]) {
        Ok(_) => Ok(Attempt::Pushed),
        Err(_) => {
            git(&ctx.repo, &["reset", "-q", "--hard", "HEAD~1"])?;
            Ok(Attempt::Rejected)
        }
    }
}

// ---------------------------------------------------------------------------------------------
// the two commands

/// Claim a task per `AGENTS.md` section 1.
///
/// # Errors
/// [`NotAvailable`] if the task cannot be claimed (taken, blocked, finished, unknown), including
/// when another claim won the race; any other error is a real failure (git, I/O, bad input).
pub fn claim(ctx: &Ctx, req: &ClaimReq) -> anyhow::Result<Done> {
    check_id(&req.id)?;
    let who = normalize_handle(&req.who)?;
    if !req.dry_run {
        // A preview reads the current checkout as it is, on any branch; a real claim must start
        // from an up-to-date, clean `main`.
        preconditions(ctx)?;
    }

    for _ in 0..MAX_ATTEMPTS {
        let loc = locate(&ctx.repo, &req.id)?;
        let all = roadmap::load_all(&ctx.repo.join("docs/roadmap"))?;
        let task = all
            .iter()
            .find(|t| t.id == req.id)
            .ok_or_else(|| anyhow!("{} not parsed from the roadmap", req.id))?;
        if let Some(why) = claim_blocker(task, &all, req.allow_open_deps) {
            return unavailable(why);
        }
        let branch = req
            .branch
            .clone()
            .unwrap_or_else(|| branch_name(&req.id, &task.title));
        let new_line = claim_line(&loc.line, &req.id, &who, &branch, &req.date)
            .ok_or_else(|| anyhow!("{}: unexpected task line format: {}", req.id, loc.line))?;
        let message = format!("{}: [INPRG] claim (@{who}) [skip ci]", req.id);
        let done = Done {
            line: new_line.clone(),
            message: message.clone(),
            dry_run: req.dry_run,
            branch: Some(branch),
        };
        if req.dry_run {
            return Ok(done);
        }
        let new_text = replace_line(&loc.text, loc.line_index, &new_line)
            .ok_or_else(|| anyhow!("line {} vanished", loc.line_index))?;
        if commit_and_push(ctx, &loc, &new_text, &message)? == Attempt::Pushed {
            return Ok(done);
        }
        // Somebody pushed first. Catch up and look again: the task may now be taken.
        sync(ctx)?;
    }
    bail!(
        "the push was rejected {MAX_ATTEMPTS} times in a row (`{}` is busy); run it again",
        ctx.branch
    )
}

/// Give a claim back (`AGENTS.md` section 1, rules 7 and 8).
///
/// # Errors
/// [`NotAvailable`] if the task is not currently claimed, or is claimed by someone else and
/// `force` is not set; other errors as for [`claim`].
pub fn release(ctx: &Ctx, req: &ReleaseReq) -> anyhow::Result<Done> {
    check_id(&req.id)?;
    let who = normalize_handle(&req.who)?;
    let reason = req.reason.trim();
    if reason.is_empty() {
        bail!("--reason is required (it goes into the commit message and tells others why)");
    }
    if reason.contains('\n') {
        bail!("--reason must be a single line");
    }
    if !req.dry_run {
        preconditions(ctx)?;
    }

    for _ in 0..MAX_ATTEMPTS {
        let loc = locate(&ctx.repo, &req.id)?;
        let Some((new_line, owner)) = release_line(&loc.line, &req.id) else {
            return unavailable(format!("{} is not currently claimed", req.id));
        };
        if !req.force && !owner.eq_ignore_ascii_case(&who) {
            return unavailable(format!(
                "{} is claimed by @{owner}, not @{who}; use --force only for a stale claim (no push for 7 days, pinged, then 48 h)",
                req.id
            ));
        }
        let message = format!("{}: [RELEASED] unclaim ({reason}) [skip ci]", req.id);
        let done = Done {
            line: new_line.clone(),
            message: message.clone(),
            dry_run: req.dry_run,
            branch: None,
        };
        if req.dry_run {
            return Ok(done);
        }
        let new_text = replace_line(&loc.text, loc.line_index, &new_line)
            .ok_or_else(|| anyhow!("line {} vanished", loc.line_index))?;
        if commit_and_push(ctx, &loc, &new_text, &message)? == Attempt::Pushed {
            return Ok(done);
        }
        sync(ctx)?;
    }
    bail!(
        "the push was rejected {MAX_ATTEMPTS} times in a row (`{}` is busy); run it again",
        ctx.branch
    )
}

/// The GitHub handle of the logged-in `gh` user.
///
/// # Errors
/// If `gh` is missing or not logged in; pass `--who` instead.
pub fn default_who() -> anyhow::Result<String> {
    let out = Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .output()
        .context("cannot run gh to find your handle; pass --who HANDLE")?;
    if !out.status.success() {
        bail!("gh is not logged in; run `gh auth login` or pass --who HANDLE");
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    // ----- pure functions ------------------------------------------------------------------

    #[test]
    fn handle_rules() {
        assert_eq!(normalize_handle("@lxstyo").unwrap(), "lxstyo");
        assert_eq!(normalize_handle("lxstyo/agent").unwrap(), "lxstyo/agent");
        assert_eq!(
            normalize_handle("Ihor-Ihorevych").unwrap(),
            "Ihor-Ihorevych"
        );
        assert_eq!(
            normalize_handle(" lxstyo\n").unwrap(),
            "lxstyo",
            "gh output ends with a newline"
        );
        for bad in [
            "",
            "@",
            "a b",
            "me@example.com",
            "a/b/c",
            "x;rm",
            "a\nb",
            "é",
        ] {
            assert!(normalize_handle(bad).is_err(), "{bad:?} must be rejected");
        }
    }

    #[test]
    fn id_rules() {
        for ok in ["T00-001", "T02-005b", "T13-999"] {
            assert!(check_id(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "T0-001",
            "T00-1",
            "t00-001",
            "T00-001 ",
            "T00-001bc",
            "x",
        ] {
            assert!(check_id(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn branch_names() {
        assert_eq!(
            branch_name("T00-028", "cargo xtask claim <ID> [--who HANDLE]"),
            "T00-028-cargo-xtask-claim-id"
        );
        assert_eq!(
            branch_name("T02-006", "CTSEMETA container reader"),
            "T02-006-ctsemeta-container-reader"
        );
        assert_eq!(branch_name("T02-006", "!!! ???"), "T02-006");
        assert_eq!(
            branch_name("T02-006", "Ünïcode only é"),
            "T02-006-n-code-only"
        );
        assert!(branch_name("T02-006", &"word ".repeat(40)).len() <= 60);
    }

    #[test]
    fn civil_dates() {
        assert_eq!(date_from_unix_days(0), "1970-01-01");
        assert_eq!(date_from_unix_days(11_016), "2000-02-29");
        assert_eq!(date_from_unix_days(20_088), "2024-12-31");
        assert_eq!(date_from_unix_days(20_778), "2026-11-21");
        assert_eq!(date_from_unix_days(-1), "1969-12-31");
        let today = today_utc();
        assert_eq!(today.len(), 10, "{today}");
        assert!(today.as_str() >= "2026-01-01", "{today}");
    }

    #[test]
    fn claim_line_edits_only_the_marker_and_inserts_the_tag() {
        let line = "- [ ] **T00-028** · Claim tool · `M` · `P0` · Deps: T00-008";
        let got = claim_line(
            line,
            "T00-028",
            "lxstyo",
            "T00-028-claim-tool",
            "2026-11-21",
        )
        .unwrap();
        assert_eq!(
            got,
            "- [~] **T00-028** [INPRG @lxstyo · branch `T00-028-claim-tool` · 2026-11-21] · Claim tool · `M` · `P0` · Deps: T00-008"
        );
        assert!(
            claim_line(&got, "T00-028", "x", "b", "d").is_none(),
            "already claimed"
        );
        assert!(
            claim_line(line, "T00-029", "x", "b", "d").is_none(),
            "wrong id"
        );
        assert!(claim_line("- [ ] **T00-0280** · x", "T00-028", "x", "b", "d").is_none());
    }

    #[test]
    fn release_is_the_exact_inverse_of_claim() {
        let original = "- [ ] **T00-028** · Claim tool · `M` · `P0` · Deps: T00-008";
        let claimed = claim_line(original, "T00-028", "lxstyo/agent", "b", "2026-11-21").unwrap();
        let (restored, owner) = release_line(&claimed, "T00-028").unwrap();
        assert_eq!(restored, original);
        assert_eq!(owner, "lxstyo/agent");
        assert!(release_line(original, "T00-028").is_none(), "not claimed");
        assert!(release_line("- [~] **T00-028** · no tag here", "T00-028").is_none());
    }

    #[test]
    fn replace_line_keeps_endings_and_other_lines() {
        assert_eq!(replace_line("a\nb\nc\n", 1, "B").unwrap(), "a\nB\nc\n");
        assert_eq!(
            replace_line("a\r\nb\r\nc\r\n", 1, "B").unwrap(),
            "a\r\nB\r\nc\r\n"
        );
        assert_eq!(
            replace_line("a\nb", 1, "B").unwrap(),
            "a\nB",
            "no trailing newline"
        );
        assert_eq!(
            replace_line("a\r\nb\nc", 0, "A").unwrap(),
            "A\r\nb\nc",
            "mixed endings"
        );
        assert!(replace_line("a\nb\n", 5, "x").is_none());
    }

    // ----- planning ------------------------------------------------------------------------

    const SEED: &str = "# T99 test track\n\
- [x] **T99-001** · Done first · `S` · `P0`\n\
- [ ] **T99-002** · Depends on a done task · `S` · `P0` · Deps: T99-001\n\
- [ ] **T99-003** · Depends on an open task · `M` · `P1` · Deps: T99-004\n\
- [ ] **T99-004** · Free task · `S` · `P1`\n\
- [ ] **T99-005** · Too big · `XL` · `P1`\n\
- [ ] **T99-006** · Mobile only · `M` · `PHASE 2`\n\
- [~] **T99-007** [INPRG @someone · branch `x` · 2026-11-01] · Already taken · `S` · `P1`\n\
- [ ] **T99-008** · Another free task · `S` · `P2`\n";

    fn parsed() -> Vec<RoadmapTask> {
        roadmap::parse_roadmap(SEED, "T99-test.md")
    }

    fn blocker(id: &str, allow: bool) -> Option<String> {
        let all = parsed();
        let task = all.iter().find(|t| t.id == id).unwrap();
        claim_blocker(task, &all, allow)
    }

    #[test]
    fn blockers() {
        assert_eq!(blocker("T99-002", false), None, "dep is done");
        assert_eq!(blocker("T99-004", false), None);
        assert!(blocker("T99-001", false).unwrap().contains("already done"));
        assert!(blocker("T99-003", false).unwrap().contains("T99-004"));
        assert_eq!(blocker("T99-003", true), None, "--allow-open-deps");
        assert!(blocker("T99-005", false).unwrap().contains("XL"));
        assert!(blocker("T99-006", false).unwrap().contains("PHASE 2"));
        assert!(
            blocker("T99-007", false)
                .unwrap()
                .contains("claimed by @someone")
        );
    }

    // ----- real git ------------------------------------------------------------------------

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// A bare "remote" plus two clones ("a" and "b") of a repository containing the seed roadmap.
    struct Fx {
        root: PathBuf,
        a: PathBuf,
        b: PathBuf,
    }

    impl Drop for Fx {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn run(dir: &Path, args: &[&str]) -> String {
        git(dir, args).unwrap_or_else(|e| panic!("{e:#}"))
    }

    fn fixture(seed: &str) -> Fx {
        fixture_with(seed, "false")
    }

    /// `autocrlf` is the clones' `core.autocrlf` ("true" is the Git for Windows default).
    fn fixture_with(seed: &str, autocrlf: &str) -> Fx {
        let eol = format!("core.autocrlf={autocrlf}");
        let root = std::env::temp_dir().join(format!(
            "ss2-claim-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let remote = root.join("remote.git");
        let (a, b) = (root.join("a"), root.join("b"));
        run(
            &root,
            &[
                "init",
                "-q",
                "--bare",
                "-b",
                "main",
                remote.to_str().unwrap(),
            ],
        );
        run(
            &root,
            &[
                "clone",
                "-q",
                "-c",
                &eol,
                remote.to_str().unwrap(),
                a.to_str().unwrap(),
            ],
        );
        configure(&a, "Alice", autocrlf);
        std::fs::create_dir_all(a.join("docs/roadmap")).unwrap();
        std::fs::write(a.join("docs/roadmap/T99-test.md"), seed).unwrap();
        run(&a, &["add", "."]);
        run(&a, &["commit", "-q", "-m", "seed"]);
        run(&a, &["push", "-q", "-u", "origin", "main"]);
        run(
            &root,
            &[
                "clone",
                "-q",
                "-c",
                &eol,
                remote.to_str().unwrap(),
                b.to_str().unwrap(),
            ],
        );
        configure(&b, "Bob", autocrlf);
        Fx { root, a, b }
    }

    fn configure(dir: &Path, name: &str, autocrlf: &str) {
        run(dir, &["config", "user.name", name]);
        run(
            dir,
            &[
                "config",
                "user.email",
                &format!("{}@example.invalid", name.to_lowercase()),
            ],
        );
        run(dir, &["config", "core.autocrlf", autocrlf]);
        run(dir, &["config", "commit.gpgsign", "false"]);
    }

    fn req(id: &str, who: &str) -> ClaimReq {
        ClaimReq {
            id: id.into(),
            who: who.into(),
            branch: None,
            date: "2026-11-21".into(),
            dry_run: false,
            allow_open_deps: false,
        }
    }

    fn roadmap_text(dir: &Path) -> String {
        std::fs::read_to_string(dir.join("docs/roadmap/T99-test.md")).unwrap()
    }

    fn remote_head(dir: &Path) -> String {
        run(dir, &["fetch", "-q", "origin"]);
        run(dir, &["rev-parse", "origin/main"]).trim().to_owned()
    }

    fn is_unavailable(e: &anyhow::Error) -> bool {
        e.downcast_ref::<NotAvailable>().is_some()
    }

    #[test]
    fn claim_pushes_one_signed_commit_with_the_exact_line() {
        let fx = fixture(SEED);
        let done = claim(&Ctx::new(&fx.a), &req("T99-004", "@alice")).unwrap();
        assert_eq!(done.message, "T99-004: [INPRG] claim (@alice) [skip ci]");
        assert_eq!(done.branch.as_deref(), Some("T99-004-free-task"));

        let text = roadmap_text(&fx.a);
        assert!(text.contains(
            "- [~] **T99-004** [INPRG @alice · branch `T99-004-free-task` · 2026-11-21] · Free task · `S` · `P1`\n"
        ), "{text}");
        assert_eq!(
            text.matches("[INPRG").count(),
            2,
            "the seed's own tag plus exactly one new one"
        );

        // Pushed, signed off, and the tree is clean and in sync.
        assert_eq!(
            remote_head(&fx.a),
            run(&fx.a, &["rev-parse", "HEAD"]).trim()
        );
        let body = run(&fx.a, &["log", "-1", "--format=%B"]);
        assert!(
            body.starts_with("T99-004: [INPRG] claim (@alice) [skip ci]"),
            "{body}"
        );
        assert!(
            body.contains("Signed-off-by: Alice <alice@example.invalid>"),
            "{body}"
        );
        assert_eq!(run(&fx.a, &["status", "--porcelain"]).trim(), "");
        let changed = run(&fx.a, &["show", "--stat", "--format=", "HEAD"]);
        assert!(
            changed.contains("1 file changed, 1 insertion(+), 1 deletion(-)"),
            "{changed}"
        );
    }

    #[test]
    fn the_second_claimant_loses_and_changes_nothing() {
        let fx = fixture(SEED);
        claim(&Ctx::new(&fx.a), &req("T99-004", "alice")).unwrap();
        let before = remote_head(&fx.b);

        // Bob's clone has not fetched yet: the tool must fetch, see the claim, and refuse.
        let err = claim(&Ctx::new(&fx.b), &req("T99-004", "bob")).unwrap_err();
        assert!(is_unavailable(&err), "{err:#}");
        assert!(
            err.to_string().contains("already claimed by @alice"),
            "{err:#}"
        );

        assert_eq!(remote_head(&fx.b), before, "nothing was pushed");
        assert_eq!(
            run(&fx.b, &["rev-parse", "HEAD"]).trim(),
            before,
            "no stray local commit"
        );
        assert!(
            roadmap_text(&fx.b).contains("@alice"),
            "Bob now sees Alice's claim"
        );
        assert!(!roadmap_text(&fx.b).contains("@bob"));
    }

    #[test]
    fn a_rejected_push_drops_our_commit_and_the_retry_succeeds() {
        let fx = fixture(SEED);
        // Bob prepares (fetch + ff) and then Alice pushes a claim: Bob's view is now stale.
        preconditions(&Ctx::new(&fx.b)).unwrap();
        claim(&Ctx::new(&fx.a), &req("T99-004", "alice")).unwrap();

        let ctx = Ctx::new(&fx.b);
        let loc = locate(&fx.b, "T99-008").unwrap();
        let line = claim_line(&loc.line, "T99-008", "bob", "T99-008-x", "2026-11-21").unwrap();
        let new_text = replace_line(&loc.text, loc.line_index, &line).unwrap();
        let head_before = run(&fx.b, &["rev-parse", "HEAD"]);
        let outcome = commit_and_push(
            &ctx,
            &loc,
            &new_text,
            "T99-008: [INPRG] claim (@bob) [skip ci]",
        )
        .unwrap();
        assert_eq!(outcome, Attempt::Rejected);
        assert_eq!(
            run(&fx.b, &["rev-parse", "HEAD"]),
            head_before,
            "our commit was dropped"
        );
        assert_eq!(
            run(&fx.b, &["status", "--porcelain", "--untracked-files=no"]).trim(),
            ""
        );

        // The full command recovers on its own and both claims end up on the remote, in order.
        claim(&ctx, &req("T99-008", "bob")).unwrap();
        run(&fx.a, &["pull", "-q", "--ff-only"]);
        let text = roadmap_text(&fx.a);
        assert!(text.contains("@alice") && text.contains("@bob"), "{text}");
        let log = run(&fx.a, &["log", "--format=%s", "-3"]);
        assert!(log.starts_with("T99-008: [INPRG] claim (@bob)"), "{log}");
        assert!(log.contains("T99-004: [INPRG] claim (@alice)"), "{log}");
    }

    #[test]
    fn unavailable_tasks_are_refused_without_touching_the_remote() {
        let fx = fixture(SEED);
        let before = remote_head(&fx.a);
        for (id, needle) in [
            ("T99-001", "already done"),
            ("T99-003", "depends on unfinished T99-004"),
            ("T99-005", "XL"),
            ("T99-006", "PHASE 2"),
            ("T99-007", "already claimed by @someone"),
            ("T99-404", "does not exist"),
        ] {
            let err = claim(&Ctx::new(&fx.a), &req(id, "alice")).unwrap_err();
            assert!(is_unavailable(&err), "{id}: {err:#}");
            assert!(err.to_string().contains(needle), "{id}: {err:#}");
        }
        assert_eq!(remote_head(&fx.a), before);
        assert_eq!(roadmap_text(&fx.a), SEED);

        // Real errors are not "unavailable".
        let err = claim(&Ctx::new(&fx.a), &req("nonsense", "alice")).unwrap_err();
        assert!(!is_unavailable(&err));
        let err = claim(&Ctx::new(&fx.a), &req("T99-004", "me@example.com")).unwrap_err();
        assert!(!is_unavailable(&err), "{err:#}");

        // Open deps can be overridden deliberately.
        let mut r = req("T99-003", "alice");
        r.allow_open_deps = true;
        claim(&Ctx::new(&fx.a), &r).unwrap();
    }

    #[test]
    fn dry_run_previews_and_changes_nothing() {
        let fx = fixture(SEED);
        let before = remote_head(&fx.a);
        let mut r = req("T99-004", "alice");
        r.dry_run = true;
        let done = claim(&Ctx::new(&fx.a), &r).unwrap();
        assert!(done.dry_run && done.line.starts_with("- [~] **T99-004** [INPRG @alice"));
        assert_eq!(roadmap_text(&fx.a), SEED);
        assert_eq!(remote_head(&fx.a), before);
        assert_eq!(run(&fx.a, &["rev-parse", "HEAD"]).trim(), before);
    }

    #[test]
    fn crlf_files_stay_crlf_and_only_one_line_changes() {
        let crlf = SEED.replace('\n', "\r\n");
        let fx = fixture(&crlf);
        claim(&Ctx::new(&fx.a), &req("T99-008", "alice")).unwrap();
        let text = roadmap_text(&fx.a);
        assert_eq!(text.matches("\r\n").count(), crlf.matches("\r\n").count());
        assert_eq!(
            text.matches('\n').count(),
            text.matches("\r\n").count(),
            "no bare LF introduced"
        );
        let changed: Vec<_> = text
            .lines()
            .zip(crlf.lines())
            .filter(|(n, o)| n != o)
            .collect();
        assert_eq!(changed.len(), 1, "{changed:?}");
    }

    /// Git for Windows defaults to `core.autocrlf=true`: the working tree has CRLF, the repository
    /// has LF. A claim must still change exactly one line in the repository.
    #[test]
    fn works_with_autocrlf_true_like_a_default_windows_checkout() {
        let fx = fixture_with(SEED, "true");
        claim(&Ctx::new(&fx.a), &req("T99-004", "alice")).unwrap();
        claim(&Ctx::new(&fx.b), &req("T99-008", "bob")).unwrap();

        let blob = run(&fx.b, &["show", "HEAD:docs/roadmap/T99-test.md"]);
        assert!(!blob.contains('\r'), "the repository copy must stay LF");
        let stat = run(&fx.b, &["show", "--stat", "--format=", "HEAD"]);
        assert!(stat.contains("1 insertion(+), 1 deletion(-)"), "{stat}");
        assert_eq!(
            run(&fx.b, &["status", "--porcelain", "--untracked-files=no"]).trim(),
            ""
        );
        assert!(blob.contains("@alice") && blob.contains("@bob"));
    }

    #[test]
    fn refuses_a_dirty_tree_and_the_wrong_branch() {
        let fx = fixture(SEED);
        std::fs::write(
            fx.a.join("docs/roadmap/T99-test.md"),
            format!("{SEED}\nedit\n"),
        )
        .unwrap();
        let err = claim(&Ctx::new(&fx.a), &req("T99-004", "alice")).unwrap_err();
        assert!(err.to_string().contains("uncommitted changes"), "{err:#}");
        run(&fx.a, &["checkout", "-q", "--", "."]);

        run(&fx.a, &["switch", "-q", "-c", "my-work"]);
        let err = claim(&Ctx::new(&fx.a), &req("T99-004", "alice")).unwrap_err();
        assert!(err.to_string().contains("you are on `my-work`"), "{err:#}");
        assert_eq!(
            run(&fx.a, &["rev-parse", "--abbrev-ref", "HEAD"]).trim(),
            "my-work",
            "branch untouched"
        );
    }

    #[test]
    fn refuses_to_push_other_unpushed_work_along_with_a_claim() {
        let fx = fixture(SEED);
        std::fs::write(fx.a.join("extra.txt"), "x").unwrap();
        run(&fx.a, &["add", "."]);
        run(&fx.a, &["commit", "-q", "-m", "local only"]);
        let err = claim(&Ctx::new(&fx.a), &req("T99-004", "alice")).unwrap_err();
        assert!(err.to_string().contains("unpushed commit"), "{err:#}");
    }

    #[test]
    fn duplicate_ids_are_a_hard_error() {
        let fx = fixture(&format!(
            "{SEED}- [ ] **T99-008** · A clashing duplicate · `S` · `P2`\n"
        ));
        let err = claim(&Ctx::new(&fx.a), &req("T99-008", "alice")).unwrap_err();
        assert!(!is_unavailable(&err), "{err:#}");
        assert!(err.to_string().contains("duplicate ID"), "{err:#}");
    }

    fn rel(id: &str, who: &str, reason: &str) -> ReleaseReq {
        ReleaseReq {
            id: id.into(),
            who: who.into(),
            reason: reason.into(),
            force: false,
            dry_run: false,
        }
    }

    #[test]
    fn release_restores_the_original_line_byte_for_byte() {
        let fx = fixture(SEED);
        claim(&Ctx::new(&fx.a), &req("T99-004", "alice")).unwrap();
        let done = release(
            &Ctx::new(&fx.a),
            &rel("T99-004", "@Alice", "blocked on a spec"),
        )
        .unwrap();
        assert_eq!(
            done.message,
            "T99-004: [RELEASED] unclaim (blocked on a spec) [skip ci]"
        );
        assert_eq!(
            roadmap_text(&fx.a),
            SEED,
            "back to the seed, nothing else changed"
        );
        assert_eq!(
            remote_head(&fx.a),
            run(&fx.a, &["rev-parse", "HEAD"]).trim()
        );
        let log = run(&fx.a, &["log", "-1", "--format=%B"]);
        assert!(log.contains("Signed-off-by: Alice"), "{log}");
    }

    #[test]
    fn release_rules() {
        let fx = fixture(SEED);
        claim(&Ctx::new(&fx.a), &req("T99-004", "alice")).unwrap();

        // Not the owner: refused without --force, allowed with it.
        let err = release(&Ctx::new(&fx.b), &rel("T99-004", "bob", "stale")).unwrap_err();
        assert!(
            is_unavailable(&err) && err.to_string().contains("claimed by @alice"),
            "{err:#}"
        );
        // A task nobody claimed.
        let err = release(&Ctx::new(&fx.b), &rel("T99-008", "bob", "x")).unwrap_err();
        assert!(
            is_unavailable(&err) && err.to_string().contains("not currently claimed"),
            "{err:#}"
        );
        // Reason is mandatory and single-line.
        assert!(release(&Ctx::new(&fx.b), &rel("T99-004", "alice", "  ")).is_err());
        assert!(release(&Ctx::new(&fx.b), &rel("T99-004", "alice", "a\nb")).is_err());

        let mut forced = rel("T99-004", "bob", "stale: no push for 9 days");
        forced.force = true;
        release(&Ctx::new(&fx.b), &forced).unwrap();
        assert_eq!(roadmap_text(&fx.b), SEED);
    }
}
