//! The **level list**: the ordered levels of a campaign, each with its display name, movies, kind and exits, validated once so that every later
//! question ("which level follows?") has an answer.
//!
//! **Spec**: the campaign of the retail install is 42 levels in 7 episodes (`game-modes.md`), each level's `.nfo` names its intro, outro and
//! (first level of an episode) episode movie (`cutscenes.md`), and a chapter with a next level ends the level, plays the outro and loads the next
//! (`help-digest.md`). The data does **not** say how the next level is chosen (`level-logic.md`: what loads it is not stated), so the order is the
//! numbering of the ids. **Chosen**: everything else here (limits, kinds, alternative exits, the validation).

use std::collections::BTreeMap;

use crate::level_progress::crc32;

/// The most levels a list may have.
pub const MAX_LEVELS: usize = 1024;
/// The longest level or episode id, in bytes.
pub const MAX_ID_LEN: usize = 64;
/// The longest display name or movie reference, in bytes.
pub const MAX_TEXT_LEN: usize = 256;
/// The most alternative exits a level may have (so an exit number, 0 for the main exit, fits in 3 bits).
pub const MAX_ALT_EXITS: usize = 7;

/// What kind of level this is. **Chosen.** Only `Main` levels complete the campaign.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LevelKind {
    /// A level of the main story.
    #[default]
    Main,
    /// A level that is reached through a secret exit.
    Secret,
    /// An extra level that is not part of the story.
    Bonus,
}

/// Where a level's **main** exit leads. **Chosen.**
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Next {
    /// The entry that follows this one in the list (an error on the last entry).
    #[default]
    Following,
    /// The level with this id.
    Level(String),
    /// The campaign ends here.
    End,
}

/// One level of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelEntry {
    /// The level's id (`1_1_jungle`): unique in the list.
    pub id: String,
    /// The episode it belongs to (`01_mdigbo`).
    pub episode: String,
    /// The display name as the data gives it (the `.nfo` `NAME`, which in the retail install is a translation key with a default text).
    pub name: String,
    /// Main, secret or bonus.
    pub kind: LevelKind,
    /// Where the main exit (exit 0) leads.
    pub next: Next,
    /// Where the alternative exits (exits 1 and up) lead, by level id: secret exits.
    pub alt_exits: Vec<String>,
    /// The movie shown when the episode starts (`EPISODE_MOVIE`), if this level starts one.
    pub episode_movie: Option<String>,
    /// The movie shown before the level (`INTRO_MOVIE`).
    pub intro_movie: Option<String>,
    /// The movie shown after the level (`OUTRO_MOVIE`).
    pub outro_movie: Option<String>,
}

impl LevelEntry {
    /// A `Main` level whose main exit leads to the following entry, with no movies.
    #[must_use]
    pub fn new(id: &str, episode: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            episode: episode.to_string(),
            name: name.to_string(),
            kind: LevelKind::Main,
            next: Next::Following,
            alt_exits: Vec::new(),
            episode_movie: None,
            intro_movie: None,
            outro_movie: None,
        }
    }

    /// Set the kind.
    #[must_use]
    pub fn with_kind(mut self, kind: LevelKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set where the main exit leads.
    #[must_use]
    pub fn with_next(mut self, next: Next) -> Self {
        self.next = next;
        self
    }

    /// Add an alternative exit to the level with this id.
    #[must_use]
    pub fn with_alt_exit(mut self, target: &str) -> Self {
        self.alt_exits.push(target.to_string());
        self
    }

    /// Set the intro movie.
    #[must_use]
    pub fn with_intro(mut self, movie: &str) -> Self {
        self.intro_movie = Some(movie.to_string());
        self
    }

    /// Set the outro movie.
    #[must_use]
    pub fn with_outro(mut self, movie: &str) -> Self {
        self.outro_movie = Some(movie.to_string());
        self
    }

    /// Set the episode movie.
    #[must_use]
    pub fn with_episode_movie(mut self, movie: &str) -> Self {
        self.episode_movie = Some(movie.to_string());
        self
    }

    /// How many exits the level has (the main exit and the alternative ones).
    #[must_use]
    pub fn exit_count(&self) -> usize {
        1 + self.alt_exits.len()
    }
}

/// Why a list was refused. Checked in this order: shape, duplicates, dangling links, cycles, unreachable levels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListError {
    /// The list has no level.
    Empty,
    /// More than [`MAX_LEVELS`] levels.
    TooMany,
    /// An id (of a level, an episode or a link target) is empty, too long or has a character outside letters, digits, `_`, `-` and `.`.
    BadId {
        /// The offending text.
        id: String,
    },
    /// A name or a movie reference is too long or has a control character.
    BadText {
        /// The level whose text it is.
        id: String,
        /// Which field.
        field: &'static str,
    },
    /// A level has more than [`MAX_ALT_EXITS`] alternative exits.
    TooManyExits {
        /// The level.
        id: String,
    },
    /// Two levels have the same id.
    DuplicateId {
        /// The id.
        id: String,
    },
    /// A link names a level that is not in the list.
    DanglingNext {
        /// The level with the link.
        from: String,
        /// The id it names.
        to: String,
    },
    /// The last entry's main exit is `Following`, but nothing follows it.
    NoFollowingLevel {
        /// The last level.
        id: String,
    },
    /// The links lead back to a level that is already on the way (this level is on the cycle).
    Cycle {
        /// A level on the cycle.
        id: String,
    },
    /// No path of exits leads to this level from the first one.
    Unreachable {
        /// The first such level in list order.
        id: String,
    },
    /// A question about a level that is not in the list.
    UnknownLevel {
        /// The id asked for.
        id: String,
    },
    /// A question about an exit that the level does not have.
    NoSuchExit {
        /// The level.
        id: String,
        /// The exit number asked for.
        exit: u8,
    },
}

impl std::fmt::Display for ListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("the level list is empty"),
            Self::TooMany => write!(f, "more than {MAX_LEVELS} levels"),
            Self::BadId { id } => write!(f, "`{id}` is not a valid id"),
            Self::BadText { id, field } => write!(f, "level `{id}`: the {field} is not valid"),
            Self::TooManyExits { id } => write!(f, "level `{id}` has too many exits"),
            Self::DuplicateId { id } => write!(f, "level `{id}` is listed twice"),
            Self::DanglingNext { from, to } => {
                write!(
                    f,
                    "level `{from}` leads to `{to}`, which is not in the list"
                )
            }
            Self::NoFollowingLevel { id } => {
                write!(
                    f,
                    "level `{id}` is the last one but leads to the following level"
                )
            }
            Self::Cycle { id } => write!(f, "the exits make a cycle through level `{id}`"),
            Self::Unreachable { id } => write!(f, "level `{id}` cannot be reached from the first"),
            Self::UnknownLevel { id } => write!(f, "no level `{id}`"),
            Self::NoSuchExit { id, exit } => write!(f, "level `{id}` has no exit {exit}"),
        }
    }
}

impl std::error::Error for ListError {}

/// Where an exit leads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destination<'a> {
    /// To this level.
    Level(&'a LevelEntry),
    /// The campaign ends.
    End,
}

/// A validated list of levels. The first entry is where the campaign starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelList {
    entries: Vec<LevelEntry>,
    /// Per entry, per exit: the entry it leads to, or `None` for the end of the campaign.
    exits: Vec<Vec<Option<usize>>>,
    by_id: BTreeMap<String, usize>,
}

impl LevelList {
    /// Validate and build a list.
    ///
    /// # Errors
    /// The first problem found, see [`ListError`].
    pub fn new(entries: Vec<LevelEntry>) -> Result<Self, ListError> {
        if entries.is_empty() {
            return Err(ListError::Empty);
        }
        if entries.len() > MAX_LEVELS {
            return Err(ListError::TooMany);
        }
        for e in &entries {
            check_entry(e)?;
        }
        let mut by_id = BTreeMap::new();
        for (i, e) in entries.iter().enumerate() {
            if by_id.insert(e.id.clone(), i).is_some() {
                return Err(ListError::DuplicateId { id: e.id.clone() });
            }
        }
        let mut exits = Vec::with_capacity(entries.len());
        for (i, e) in entries.iter().enumerate() {
            let mut out = Vec::with_capacity(e.exit_count());
            out.push(match &e.next {
                Next::Following if i + 1 < entries.len() => Some(i + 1),
                Next::Following => {
                    return Err(ListError::NoFollowingLevel { id: e.id.clone() });
                }
                Next::Level(to) => Some(resolve(&by_id, &e.id, to)?),
                Next::End => None,
            });
            for to in &e.alt_exits {
                out.push(Some(resolve(&by_id, &e.id, to)?));
            }
            exits.push(out);
        }
        if let Some(at) = find_cycle(&exits) {
            return Err(ListError::Cycle {
                id: entries[at].id.clone(),
            });
        }
        let reached = reachable(&exits);
        if let Some(i) = reached.iter().position(|r| !r) {
            return Err(ListError::Unreachable {
                id: entries[i].id.clone(),
            });
        }
        Ok(Self {
            entries,
            exits,
            by_id,
        })
    }

    /// The levels in order.
    #[must_use]
    pub fn entries(&self) -> &[LevelEntry] {
        &self.entries
    }

    /// How many levels there are (at least one).
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Always false: a list has at least one level.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// The level the campaign starts with.
    #[must_use]
    pub fn first(&self) -> &LevelEntry {
        &self.entries[0]
    }

    /// The position of a level in the list.
    #[must_use]
    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.by_id.get(id).copied()
    }

    /// The level with this id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&LevelEntry> {
        self.index_of(id).map(|i| &self.entries[i])
    }

    /// Where an exit of a level leads (exit 0 is the main exit).
    ///
    /// # Errors
    /// [`ListError::UnknownLevel`] or [`ListError::NoSuchExit`].
    pub fn destination(&self, id: &str, exit: u8) -> Result<Destination<'_>, ListError> {
        let i = self
            .index_of(id)
            .ok_or_else(|| ListError::UnknownLevel { id: id.to_string() })?;
        match self.exits[i].get(usize::from(exit)) {
            Some(Some(to)) => Ok(Destination::Level(&self.entries[*to])),
            Some(None) => Ok(Destination::End),
            None => Err(ListError::NoSuchExit {
                id: id.to_string(),
                exit,
            }),
        }
    }

    /// The same as [`Self::destination`] with positions: `Some(index)` or `None` for the end.
    pub(super) fn destination_index(&self, from: usize, exit: u8) -> Option<Option<usize>> {
        self.exits.get(from)?.get(usize::from(exit)).copied()
    }

    /// The ids of the levels met by always taking the main exit from the first level, up to the end of the campaign.
    #[must_use]
    pub fn main_path(&self) -> Vec<&str> {
        let mut path = Vec::new();
        let mut at = Some(0usize);
        while let Some(i) = at {
            path.push(self.entries[i].id.as_str());
            at = self.exits[i][0];
        }
        path
    }

    /// A number that changes when the ids, their order or any exit changes (CRC-32 over each id and where its exits lead). A save keeps it so that
    /// it is not read back with another list.
    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut bytes = Vec::new();
        for (e, out) in self.entries.iter().zip(&self.exits) {
            bytes.extend_from_slice(e.id.as_bytes());
            bytes.push(0);
            bytes.push(u8::try_from(out.len()).unwrap_or(u8::MAX));
            for to in out {
                let v = to.map_or(u16::MAX, |t| u16::try_from(t).unwrap_or(u16::MAX - 1));
                bytes.extend_from_slice(&v.to_le_bytes());
            }
        }
        crc32(&bytes)
    }
}

pub(super) fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}

fn valid_text(t: &str) -> bool {
    t.len() <= MAX_TEXT_LEN && !t.chars().any(char::is_control)
}

fn check_entry(e: &LevelEntry) -> Result<(), ListError> {
    for id in [&e.id, &e.episode] {
        if !valid_id(id) {
            return Err(ListError::BadId { id: id.clone() });
        }
    }
    if !valid_text(&e.name) {
        return Err(bad_text(e, "name"));
    }
    for (field, movie) in [
        ("episode movie", &e.episode_movie),
        ("intro movie", &e.intro_movie),
        ("outro movie", &e.outro_movie),
    ] {
        if let Some(m) = movie
            && (m.is_empty() || !valid_text(m))
        {
            return Err(bad_text(e, field));
        }
    }
    if e.alt_exits.len() > MAX_ALT_EXITS {
        return Err(ListError::TooManyExits { id: e.id.clone() });
    }
    let targets = e.alt_exits.iter().chain(match &e.next {
        Next::Level(to) => Some(to),
        _ => None,
    });
    for to in targets {
        if !valid_id(to) {
            return Err(ListError::BadId { id: to.clone() });
        }
    }
    Ok(())
}

fn bad_text(e: &LevelEntry, field: &'static str) -> ListError {
    ListError::BadText {
        id: e.id.clone(),
        field,
    }
}

fn resolve(by_id: &BTreeMap<String, usize>, from: &str, to: &str) -> Result<usize, ListError> {
    by_id
        .get(to)
        .copied()
        .ok_or_else(|| ListError::DanglingNext {
            from: from.to_string(),
            to: to.to_string(),
        })
}

/// A node that lies on a cycle, found by a depth-first search in list order (iterative: a long list does not use the call stack).
fn find_cycle(exits: &[Vec<Option<usize>>]) -> Option<usize> {
    // 0 = not visited, 1 = on the current path, 2 = done.
    let mut state = vec![0u8; exits.len()];
    for start in 0..exits.len() {
        if state[start] != 0 {
            continue;
        }
        state[start] = 1;
        let mut stack: Vec<(usize, usize)> = vec![(start, 0)];
        while let Some(&(v, i)) = stack.last() {
            if let Some(&edge) = exits[v].get(i) {
                if let Some(top) = stack.last_mut() {
                    top.1 += 1;
                }
                if let Some(w) = edge {
                    match state[w] {
                        1 => return Some(w),
                        0 => {
                            state[w] = 1;
                            stack.push((w, 0));
                        }
                        _ => {}
                    }
                }
            } else {
                state[v] = 2;
                stack.pop();
            }
        }
    }
    None
}

/// Which entries can be reached from the first one.
fn reachable(exits: &[Vec<Option<usize>>]) -> Vec<bool> {
    let mut seen = vec![false; exits.len()];
    let mut stack = vec![0usize];
    seen[0] = true;
    while let Some(v) = stack.pop() {
        for w in exits[v].iter().flatten() {
            if !seen[*w] {
                seen[*w] = true;
                stack.push(*w);
            }
        }
    }
    seen
}

#[cfg(test)]
#[path = "list_tests.rs"]
mod tests;
