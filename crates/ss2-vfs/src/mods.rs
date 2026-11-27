//! Mod descriptors and mod mount order (roadmap T02-004; format: `docs/re/mods.md`, `console-config.md`).
//!
//! A *mod* is a descriptor plus content archives:
//!
//! * `Content/<Mod>.mod`: the game modules to load ([`ModManifest`]; a reimplementation only
//!   needs the list, it has no DLLs);
//! * `Content/<Mod>.nfo`: display name, author and thumbnail ([`Nfo`], [`ModInfo`]);
//! * `Content/<Mod>/*.gro`: **overlay archives**, mounted after the base archives and the patches
//!   so that their entries replace base entries by path ([`mount_mod`]);
//! * `Content/CurrentMod.txt` or, when it is absent, `Content/DefaultMod.txt`: which mod the game
//!   starts ([`selected_mod`]).
//!
//! The `.nfo` reader is the general `.nfo` reader of the game (levels, episodes, player models
//! and demos use the same syntax), so [`Nfo`] keeps every `KEY = "value"` pair it finds.

use std::path::{Path, PathBuf};

use ss2_core::{Error, Result};

use crate::{ArchiveVfs, EntryKind, LayeredVfs, Vfs, VfsPath, archive_load_order};

/// A game module named by a `.mod` file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModModule {
    /// The module name without extension, as written (`Sam2Game`).
    pub name: String,
    /// `true` for a name written with the `<TOOLS>` prefix: loaded by the editor only.
    pub tools_only: bool,
}

/// The content of a `.mod` file: `MODULES = <name> <name> … ;`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModManifest {
    /// The modules in file order.
    pub modules: Vec<ModModule>,
}

impl ModManifest {
    /// Parse the text of a `.mod` file.
    ///
    /// The file is `MODULES =` followed by whitespace-separated module names and a closing `;`.
    /// Line ends may be CRLF or LF; a UTF-8 byte-order mark is skipped.
    ///
    /// # Errors
    /// [`Error::Invalid`] if `MODULES`, `=` or the closing `;` is missing, or the list is empty.
    pub fn parse(text: &str) -> Result<Self> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut tokens = Tokens::new(text);
        match tokens.word() {
            Some(w) if w.eq_ignore_ascii_case("MODULES") => {}
            other => {
                return Err(Error::invalid(
                    ".mod file",
                    format!("expected `MODULES`, found {other:?}"),
                ));
            }
        }
        if !tokens.eat('=') {
            return Err(Error::invalid(".mod file", "expected `=` after `MODULES`"));
        }
        let mut modules = Vec::new();
        loop {
            tokens.skip_space();
            if tokens.eat(';') {
                break;
            }
            let Some(word) = tokens.word() else {
                return Err(Error::invalid(
                    ".mod file",
                    "the module list is not closed by `;`",
                ));
            };
            let (tools_only, name) = match word.strip_prefix("<TOOLS>") {
                Some(rest) => (true, rest),
                None => (false, word),
            };
            if name.is_empty() {
                return Err(Error::invalid(".mod file", "empty module name"));
            }
            modules.push(ModModule {
                name: name.to_string(),
                tools_only,
            });
        }
        if modules.is_empty() {
            return Err(Error::invalid(".mod file", "no module is listed"));
        }
        Ok(Self { modules })
    }

    /// Names of the modules the **game** loads (the `<TOOLS>` ones excluded).
    pub fn game_modules(&self) -> impl Iterator<Item = &str> {
        self.modules
            .iter()
            .filter(|m| !m.tools_only)
            .map(|m| m.name.as_str())
    }
}

/// A parsed `.nfo` file: an optional kind word and the `KEY = "value"` pairs in file order.
///
/// Unknown keys are kept; callers pick what they need with [`Nfo::get`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Nfo {
    /// The word on a line of its own before the pairs (`LEVEL`, `EPISODE`); `None` for mod,
    /// player-model and demo files.
    pub kind: Option<String>,
    /// Keys (upper case as written) and values (quotes removed), in file order.
    pub entries: Vec<(String, String)>,
}

impl Nfo {
    /// Parse the bytes of a `.nfo` file: UTF-8 (a byte-order mark is skipped), or Latin-1 when
    /// the bytes are not valid UTF-8 (the install has one such file).
    ///
    /// Syntax: an optional kind word on its own line, then assignments `KEY = "value"` with an
    /// optional closing `;`; the value may be on the next line; a value that is not quoted runs to
    /// the end of its line or to a `;`. Keys are compared case-insensitively by [`Nfo::get`].
    ///
    /// # Errors
    /// [`Error::Invalid`] on an assignment without `=`, or a quoted value that is not closed.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
        let text = match std::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
        };
        let mut tokens = Tokens::new(&text);
        let mut nfo = Self::default();
        tokens.skip_space();
        // A kind word is a word followed by a line end (not by `=`).
        let save = tokens.pos;
        if let Some(w) = tokens.word() {
            let rest_of_line = tokens.rest_of_line_is_blank();
            if rest_of_line {
                nfo.kind = Some(w.to_string());
            } else {
                tokens.pos = save;
            }
        }
        loop {
            tokens.skip_space();
            if tokens.at_end() {
                break;
            }
            if tokens.eat(';') {
                continue;
            }
            let Some(key) = tokens.word() else {
                return Err(Error::invalid(".nfo file", "expected a key"));
            };
            let key = key.to_string();
            if !tokens.eat('=') {
                return Err(Error::invalid(
                    ".nfo file",
                    format!("expected `=` after `{key}`"),
                ));
            }
            tokens.skip_space();
            let value = if tokens.peek() == Some('"') {
                tokens.quoted().ok_or_else(|| {
                    Error::invalid(".nfo file", format!("the value of `{key}` is not closed"))
                })?
            } else {
                tokens.unquoted()
            };
            nfo.entries.push((key, value));
            tokens.eat(';');
        }
        Ok(nfo)
    }

    /// The first value of `key` (compared case-insensitively).
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }
}

/// What the mod list shows about one mod.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModInfo {
    /// The file stem of the `.mod` file, **lower case** like every name the VFS lists (`serioussam2`,\n    /// `insamnity2`; the display name is `name`): also the name of the
    /// mod's folder `Content/<id>/`.
    pub id: String,
    /// `NAME` of the `.nfo` (`Serious Sam 2`); the id when there is no `.nfo`.
    pub name: String,
    /// `AUTHOR`, if present.
    pub author: Option<String>,
    /// `THUMBNAIL`, a `.tex` path, if present and a valid path.
    pub thumbnail: Option<VfsPath>,
    /// The parsed `.mod` file.
    pub manifest: ModManifest,
}

/// Parse `Content/CurrentMod.txt` or `Content/DefaultMod.txt`: `#` comment lines and one
/// statement `CURRENTMOD = <file> ;`. Returns the file name as written (`SeriousSam2.mod`).
///
/// # Errors
/// [`Error::Invalid`] if the statement is missing or malformed.
pub fn parse_current_mod(text: &str) -> Result<String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut tokens = Tokens::new(text);
    tokens.skip_space_and_hash_comments();
    match tokens.word() {
        Some(w) if w.eq_ignore_ascii_case("CURRENTMOD") => {}
        other => {
            return Err(Error::invalid(
                "mod selection file",
                format!("expected `CURRENTMOD`, found {other:?}"),
            ));
        }
    }
    if !tokens.eat('=') {
        return Err(Error::invalid(
            "mod selection file",
            "expected `=` after `CURRENTMOD`",
        ));
    }
    tokens.skip_space();
    let Some(file) = tokens.word() else {
        return Err(Error::invalid("mod selection file", "no mod file name"));
    };
    let file = file.to_string();
    if !tokens.eat(';') {
        return Err(Error::invalid(
            "mod selection file",
            "the statement is not closed by `;`",
        ));
    }
    Ok(file)
}

fn content_file(name: &str) -> VfsPath {
    VfsPath::new(&format!("content/{name}")).expect("static path is valid")
}

fn mod_id_of(file_name: &str) -> &str {
    file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem)
}

/// The mod the game starts: the id (lower-case file stem, `serioussam2`) named by `Content/CurrentMod.txt`,
/// or by `Content/DefaultMod.txt` when there is no `CurrentMod.txt`.
///
/// # Errors
/// [`Error::NotFound`] if neither file exists; [`Error::Invalid`] if the file is malformed.
pub fn selected_mod(vfs: &dyn Vfs) -> Result<String> {
    for name in ["CurrentMod.txt", "DefaultMod.txt"] {
        let path = content_file(name);
        if vfs.exists(&path) {
            let bytes = vfs.read(&path)?;
            let text = String::from_utf8_lossy(&bytes);
            let file = parse_current_mod(&text)?;
            return Ok(mod_id_of(&file).to_lowercase());
        }
    }
    Err(Error::NotFound(
        "Content/CurrentMod.txt and Content/DefaultMod.txt".to_string(),
    ))
}

/// Every mod of the install: one entry per `Content/*.mod` file, sorted by id (lower case).
///
/// `vfs` should see the archives (the base game's `SeriousSam2.mod` lives in `Patch_02_068.gro`)
/// **and** the loose files of the install folder (the mods' own `.mod` and `.nfo` files).
///
/// # Errors
/// [`Error::NotFound`] if `Content/` does not exist; the error of any unreadable or malformed
/// `.mod` file. A missing or malformed `.nfo` is **not** an error: the entry then has the id as
/// its name and no author or thumbnail.
pub fn discover_mods(vfs: &dyn Vfs) -> Result<Vec<ModInfo>> {
    let content = VfsPath::new("content")?;
    let mut mods = Vec::new();
    for entry in vfs.list(&content)? {
        if entry.kind != EntryKind::File || !entry.name.ends_with(".mod") {
            continue;
        }
        let text = String::from_utf8_lossy(&vfs.read(&content.join(&entry.name)?)?).into_owned();
        let manifest = ModManifest::parse(&text).map_err(|e| match e {
            Error::Invalid { what, detail } => Error::Invalid {
                what,
                detail: format!("{}: {detail}", entry.name),
            },
            other => other,
        })?;
        let id = mod_id_of(&entry.name).to_string();
        let nfo = vfs
            .read(&content_file(&format!("{id}.nfo")))
            .ok()
            .and_then(|b| Nfo::parse(&b).ok())
            .unwrap_or_default();
        mods.push(ModInfo {
            name: nfo.get("NAME").map_or_else(|| id.clone(), str::to_string),
            author: nfo.get("AUTHOR").map(str::to_string),
            thumbnail: nfo.get("THUMBNAIL").and_then(|t| VfsPath::new(t).ok()),
            id,
            manifest,
        });
    }
    mods.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(mods)
}

/// The folder `Content/<mod_id>/` of the install, found case-insensitively.
pub(crate) fn mod_folder(install_dir: &Path, mod_id: &str) -> Result<PathBuf> {
    let content = install_dir.join("Content");
    for entry in std::fs::read_dir(&content).map_err(|e| Error::io(&content, e))? {
        let entry = entry.map_err(|e| Error::io(&content, e))?;
        if entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(mod_id)
            && entry.path().is_dir()
        {
            return Ok(entry.path());
        }
    }
    Err(Error::NotFound(format!(
        "mod folder Content/{mod_id} in {}",
        install_dir.display()
    )))
}

/// The overlay archives of a mod in **mount order** (earlier ones are overridden by later ones):
/// the `.gro` files of `Content/<mod_id>/`, naturally ordered (so the `z_`-prefixed update packs
/// come last).
///
/// # Errors
/// [`Error::NotFound`] if the mod has no folder; [`Error::Io`] if it cannot be read.
pub fn mod_archive_names(install_dir: impl AsRef<Path>, mod_id: &str) -> Result<Vec<String>> {
    let dir = mod_folder(install_dir.as_ref(), mod_id)?;
    let mut names = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| Error::io(&dir, e))? {
        let entry = entry.map_err(|e| Error::io(&dir, e))?;
        let path = entry.path();
        let is_gro = path
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("gro"));
        if is_gro && path.is_file() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    Ok(archive_load_order(names))
}

/// Mount the overlay archives of mod `mod_id` on top of `layers` (which should already hold the
/// base archives and the patches, see [`crate::mount_install`]). Returns the names of the
/// archives mounted, in order.
///
/// Placeholder archives (a single `dummy.txt`, which only mark that an optional pack is
/// installed) are skipped. The layer names are `<mod_id>/<archive>`. Like
/// [`crate::mount_install`], this reuses archive indexes when available and an unreadable
/// archive fails the mount.
///
/// # Errors
/// As [`mod_archive_names`]; [`Error::Invalid`] naming the first archive that is not a valid ZIP.
pub fn mount_mod(
    layers: &mut LayeredVfs,
    install_dir: impl AsRef<Path>,
    mod_id: &str,
) -> Result<Vec<String>> {
    let install_dir = install_dir.as_ref();
    let dir = mod_folder(install_dir, mod_id)?;
    let mut mounted = Vec::new();
    let cache_dir = ArchiveVfs::cache_dir();
    for name in mod_archive_names(install_dir, mod_id)? {
        let archive = ArchiveVfs::from_cached_file(&dir.join(&name), cache_dir.as_deref())?;
        let only_dummy = archive.file_count() == 1
            && archive.exists(&VfsPath::new("dummy.txt").expect("static path is valid"));
        if only_dummy || archive.file_count() == 0 {
            continue;
        }
        layers.mount(format!("{mod_id}/{name}"), archive);
        mounted.push(name);
    }
    Ok(mounted)
}

/// A tiny tokenizer shared by the three text formats.
struct Tokens<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> Tokens<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, pos: 0 }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.text.len()
    }

    fn peek(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    fn skip_space(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    /// Skip whitespace and whole `#` comment lines.
    fn skip_space_and_hash_comments(&mut self) {
        loop {
            self.skip_space();
            if self.peek() == Some('#') {
                match self.text[self.pos..].find('\n') {
                    Some(n) => self.pos += n + 1,
                    None => self.pos = self.text.len(),
                }
            } else {
                break;
            }
        }
    }

    /// Consume `c` (after optional whitespace) if it is next.
    fn eat(&mut self, c: char) -> bool {
        self.skip_space();
        if self.peek() == Some(c) {
            self.pos += c.len_utf8();
            true
        } else {
            false
        }
    }

    /// A run of characters that are not whitespace, `=` or `;`.
    fn word(&mut self) -> Option<&'a str> {
        self.skip_space();
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_whitespace() || c == '=' || c == ';' {
                break;
            }
            self.pos += c.len_utf8();
        }
        (self.pos > start).then(|| &self.text[start..self.pos])
    }

    /// `true` if only blanks remain on the current line (the position is not moved).
    fn rest_of_line_is_blank(&self) -> bool {
        self.text[self.pos..]
            .chars()
            .take_while(|&c| c != '\n')
            .all(|c| c == ' ' || c == '\t' || c == '\r')
    }

    /// A `"…"` string without escapes; the position must be on the opening quote.
    fn quoted(&mut self) -> Option<String> {
        let start = self.pos + 1;
        let end = self.text[start..].find('"')? + start;
        self.pos = end + 1;
        Some(self.text[start..end].to_string())
    }

    /// An unquoted value: to the end of the line or to a `;`, trimmed.
    fn unquoted(&mut self) -> String {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == '\n' || c == ';' {
                break;
            }
            self.pos += c.len_utf8();
        }
        self.text[start..self.pos].trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryVfs;
    use ss2_testkit::fixtures::ZipBuilder;

    fn p(s: &str) -> VfsPath {
        VfsPath::new(s).unwrap()
    }

    #[test]
    fn parses_the_game_mod_file_byte_for_byte() {
        let m = ModManifest::parse("MODULES = \r\n  Sam2Game\r\n  <TOOLS>Sam2GameGui\r\n;\r\n")
            .unwrap();
        assert_eq!(
            m.modules,
            [
                ModModule {
                    name: "Sam2Game".into(),
                    tools_only: false
                },
                ModModule {
                    name: "Sam2GameGui".into(),
                    tools_only: true
                }
            ]
        );
        assert_eq!(m.game_modules().collect::<Vec<_>>(), ["Sam2Game"]);
    }

    #[test]
    fn mod_file_accepts_lf_bom_and_one_line() {
        let a =
            ModManifest::parse("\u{feff}MODULES = InSamnity2Game <TOOLS>Sam2GameGui ;").unwrap();
        assert_eq!(a.modules.len(), 2);
        assert_eq!(a.modules[0].name, "InSamnity2Game");
    }

    #[test]
    fn mod_file_errors_are_specific() {
        for (text, needle) in [
            ("", "MODULES"),
            ("MODS = A ;", "MODULES"),
            ("MODULES A ;", "`=`"),
            ("MODULES = A B", "not closed"),
            ("MODULES = ;", "no module"),
            ("MODULES = <TOOLS> ;", "empty module name"),
        ] {
            let err = ModManifest::parse(text).unwrap_err().to_string();
            assert!(err.contains(needle), "{text:?}: {err}");
        }
    }

    #[test]
    fn nfo_of_a_mod_has_no_kind() {
        let n = Nfo::parse(
            b"NAME = \"Renovation\" ;\r\nAUTHOR = \"Arthur 'Ar2R-devil-PiNKy' Sidorin\" ;\r\nTHUMBNAIL = \"Content/Sam2Renovation/Sam2Renovation_thumbnail.tex\" ;\r\n",
        )
        .unwrap();
        assert_eq!(n.kind, None);
        assert_eq!(n.get("name"), Some("Renovation"));
        assert_eq!(n.get("AUTHOR"), Some("Arthur 'Ar2R-devil-PiNKy' Sidorin"));
        assert_eq!(n.entries.len(), 3);
    }

    #[test]
    fn nfo_of_a_level_has_a_kind_and_quoted_semicolons() {
        let n = Nfo::parse(
            b"LEVEL\r\nMAP_TYPE=\"SINGLE; COOP;\"\r\nNAME=\"TTRS:1_1_Mdigbo_Jungle.Name=JUNGLE\"\r\nETA=\"00:10:00\"\r\nINTRO_MOVIE=\"a.ogv\" \r\nOUTRO_MOVIE=\"b.ogv\"\r\n",
        )
        .unwrap();
        assert_eq!(n.kind.as_deref(), Some("LEVEL"));
        assert_eq!(n.get("MAP_TYPE"), Some("SINGLE; COOP;"));
        assert_eq!(n.get("NAME"), Some("TTRS:1_1_Mdigbo_Jungle.Name=JUNGLE"));
        assert_eq!(n.get("OUTRO_MOVIE"), Some("b.ogv"));
        assert_eq!(n.get("MISSING"), None);
    }

    #[test]
    fn nfo_accepts_bom_latin1_and_a_value_on_the_next_line() {
        let bom = Nfo::parse(b"\xEF\xBB\xBFNAME = \"X\"").unwrap();
        assert_eq!(bom.get("NAME"), Some("X"));
        let latin1 = Nfo::parse(b"NAME = \"Caf\xE9\"").unwrap();
        assert_eq!(latin1.get("NAME"), Some("Caf\u{e9}"));
        let wrapped = Nfo::parse(b"EPISODE\nNAME=\n  \"Next line\"\nTHUMBNAIL=\"t.tex\"").unwrap();
        assert_eq!(wrapped.kind.as_deref(), Some("EPISODE"));
        assert_eq!(wrapped.get("NAME"), Some("Next line"));
        let bare = Nfo::parse(b"NAME = \"A\"\nTIMESTAMP = 2005-01-02 10:00\n").unwrap();
        assert_eq!(bare.get("TIMESTAMP"), Some("2005-01-02 10:00"));
    }

    #[test]
    fn nfo_errors() {
        assert!(Nfo::parse(b"NAME \"x\"").is_err());
        assert!(Nfo::parse(b"NAME = \"open").is_err());
        assert_eq!(Nfo::parse(b"").unwrap(), Nfo::default());
    }

    #[test]
    fn current_mod_file_skips_comments() {
        let text = "# This file contains file name of default mod.\r\n# File is parsed both by Jam\r\n\r\nCURRENTMOD = SeriousSam2.mod ;\r\n";
        assert_eq!(parse_current_mod(text).unwrap(), "SeriousSam2.mod");
        assert!(parse_current_mod("# only a comment").is_err());
        assert!(parse_current_mod("CURRENTMOD = A.mod").is_err());
        assert!(parse_current_mod("CURRENTMOD A.mod ;").is_err());
    }

    fn vfs_with(files: &[(&str, &str)]) -> MemoryVfs {
        let mut v = MemoryVfs::new();
        for (n, d) in files {
            v.insert(n, d.as_bytes().to_vec()).unwrap();
        }
        v
    }

    #[test]
    fn current_mod_wins_over_default_mod() {
        let default_only =
            vfs_with(&[("Content/DefaultMod.txt", "CURRENTMOD = SeriousSam2.mod ;")]);
        assert_eq!(selected_mod(&default_only).unwrap(), "serioussam2");
        let both = vfs_with(&[
            ("Content/DefaultMod.txt", "CURRENTMOD = SeriousSam2.mod ;"),
            ("Content/CurrentMod.txt", "CURRENTMOD = InSamnity2.mod ;"),
        ]);
        assert_eq!(selected_mod(&both).unwrap(), "insamnity2");
        let none = vfs_with(&[("Content/x.txt", "")]);
        assert!(matches!(selected_mod(&none), Err(Error::NotFound(_))));
        let bad = vfs_with(&[("Content/CurrentMod.txt", "oops")]);
        assert!(matches!(selected_mod(&bad), Err(Error::Invalid { .. })));
    }

    #[test]
    fn discovers_mods_sorted_with_and_without_nfo() {
        let v = vfs_with(&[
            (
                "Content/SeriousSam2.mod",
                "MODULES = Sam2Game <TOOLS>Sam2GameGui ;",
            ),
            (
                "Content/SeriousSam2.nfo",
                "NAME = \"Serious Sam 2\" ;\nAUTHOR = \"Croteam\" ;\nTHUMBNAIL = \"Content/SeriousSam2/t.tex\" ;",
            ),
            ("Content/Sam2Renovation.mod", "MODULES = Sam2Game ;"),
            ("Content/Sam2Renovation.nfo", "NAME = \"Renovation\" ;"),
            ("Content/Bare.mod", "MODULES = Sam2Game ;"),
            ("Content/other.txt", "ignored"),
        ]);
        let mods = discover_mods(&v).unwrap();
        let ids: Vec<_> = mods.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["bare", "sam2renovation", "serioussam2"]);
        assert_eq!(mods[0].name, "bare");
        assert_eq!(mods[0].author, None);
        assert_eq!(mods[1].name, "Renovation");
        assert_eq!(mods[2].name, "Serious Sam 2");
        assert_eq!(mods[2].author.as_deref(), Some("Croteam"));
        assert_eq!(mods[2].thumbnail, Some(p("content/seriousSam2/t.tex")));
        assert_eq!(mods[2].manifest.modules.len(), 2);
    }

    #[test]
    fn a_malformed_mod_file_names_the_file() {
        let v = vfs_with(&[("Content/Broken.mod", "MODULES = A")]);
        let err = discover_mods(&v).unwrap_err().to_string();
        assert!(err.contains("broken.mod"), "{err}");
    }

    fn write_gro(dir: &Path, name: &str, files: &[(&str, &str)]) {
        let mut b = ZipBuilder::new();
        for (n, d) in files {
            b = b.add(*n, d.as_bytes().to_vec());
        }
        std::fs::write(dir.join(name), b.finish().unwrap()).unwrap();
    }

    #[test]
    fn mod_archives_mount_after_the_base_and_replace_by_path() {
        let dir = tempfile::tempdir().unwrap();
        let content = dir.path().join("Content");
        let mod_dir = content.join("InSamnity2");
        std::fs::create_dir_all(&mod_dir).unwrap();
        write_gro(
            dir.path(),
            "All_PC_01.gro",
            &[("Content/x.txt", "base"), ("Content/b.txt", "b")],
        );
        write_gro(
            dir.path(),
            "Patch_02_068.gro",
            &[("Content/x.txt", "patch")],
        );
        write_gro(
            &mod_dir,
            "InSamnity2.gro",
            &[("Content/x.txt", "mod"), ("Content/m.txt", "m")],
        );
        write_gro(
            &mod_dir,
            "z_InSamnity2Update_08.gro",
            &[("content/X.TXT", "update"), ("Content/n.txt", "n")],
        );
        write_gro(&mod_dir, "InSamnity2Players.gro", &[("dummy.txt", "")]);
        std::fs::write(mod_dir.join("ChapterMaxes.txt"), "loose").unwrap();

        let mut vfs = crate::mount_install(dir.path()).unwrap();
        let before = vfs.layer_names().len();
        // The folder is found whatever its case.
        let mounted = mount_mod(&mut vfs, dir.path(), "insamnity2").unwrap();
        assert_eq!(mounted, ["InSamnity2.gro", "z_InSamnity2Update_08.gro"]);
        assert_eq!(vfs.layer_names().len(), before + 2);
        assert_eq!(vfs.read(&p("content/x.txt")).unwrap(), b"update");
        assert_eq!(
            vfs.source_of(&p("content/x.txt")),
            Some("insamnity2/z_InSamnity2Update_08.gro")
        );
        assert_eq!(vfs.read(&p("content/b.txt")).unwrap(), b"b");
        assert_eq!(vfs.read(&p("content/m.txt")).unwrap(), b"m");
        assert_eq!(
            mod_archive_names(dir.path(), "InSamnity2").unwrap(),
            [
                "InSamnity2.gro",
                "InSamnity2Players.gro",
                "z_InSamnity2Update_08.gro"
            ]
        );
    }

    #[test]
    fn a_mod_without_a_folder_or_with_a_bad_archive_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Content/Bad")).unwrap();
        std::fs::write(dir.path().join("Content/Bad/Bad.gro"), b"not a zip").unwrap();
        let mut vfs = LayeredVfs::new();
        let err = mount_mod(&mut vfs, dir.path(), "Nope").unwrap_err();
        assert!(matches!(err, Error::NotFound(_)), "{err:?}");
        let err = mount_mod(&mut vfs, dir.path(), "Bad").unwrap_err();
        assert!(err.to_string().contains("Bad.gro"), "{err}");
        let no_content = tempfile::tempdir().unwrap();
        assert!(matches!(
            mount_mod(&mut vfs, no_content.path(), "X"),
            Err(Error::Io { .. })
        ));
    }

    /// Data-gated: the retail install (and its mods, when present). Skips without `SS2_DATA_DIR`.
    #[test]
    fn the_real_install_has_the_base_mod_selected_and_its_mods_mount() {
        let dir = ss2_testkit::require_game_data!();
        let mut vfs = crate::mount_install(&dir).unwrap();
        vfs.mount("loose", crate::DirectoryVfs::new(&dir).unwrap());

        assert_eq!(selected_mod(&vfs).unwrap(), "serioussam2");
        let mods = discover_mods(&vfs).unwrap();
        let base = mods
            .iter()
            .find(|m| m.id == "serioussam2")
            .expect("base mod");
        assert_eq!(base.name, "Serious Sam 2");
        assert_eq!(base.author.as_deref(), Some("Croteam"));
        assert_eq!(
            base.manifest.game_modules().collect::<Vec<_>>(),
            ["Sam2Game"]
        );
        assert!(
            base.manifest
                .modules
                .iter()
                .any(|m| m.tools_only && m.name == "Sam2GameGui")
        );
        eprintln!(
            "mods: {:?}",
            mods.iter().map(|m| (&m.id, &m.name)).collect::<Vec<_>>()
        );

        for m in mods.iter().filter(|m| m.id != "serioussam2") {
            if let Ok(names) = mod_archive_names(&dir, &m.id) {
                let mut layers = crate::mount_install(&dir).unwrap();
                let mounted = mount_mod(&mut layers, &dir, &m.id).unwrap();
                eprintln!(
                    "{}: {} of {} archives mounted",
                    m.id,
                    mounted.len(),
                    names.len()
                );
                assert!(mounted.len() <= names.len());
            }
        }
    }
}
