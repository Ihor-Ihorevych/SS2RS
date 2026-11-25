//! The mods of an install folder: finding them, reading what they need, and mounting a load
//! order of them on the VFS stack (roadmap T13-001; rules in `docs/MOD_LOADING.md`).
//!
//! A mod lives in `Content/<id>/` (its `.gro` overlay archives) and is described by
//! `Content/<id>.mod` and `Content/<id>.nfo`. The descriptors sit **loose** in the install folder
//! for some mods (`Sam2Renovation`) and **only inside the mod's own archive** for others
//! (`InSamnity2`), so a scan has to look in both (`docs/re/mods.md`, "Folder layout").

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use ss2_core::{Error, Result};

use crate::modcheck::{ascii, plural};
use crate::modload::{
    BASE_MOD_ID, LoadOrderError, LoadPlan, ModNode, REQUIRES_MOD_KEY, normalize_mod_id,
    parse_mod_list, resolve_load_order,
};
use crate::mods::mod_folder;
use crate::{
    ArchiveVfs, DirectoryVfs, EntryKind, LayeredVfs, ModInfo, ModManifest, Nfo, Stat, Vfs, VfsPath,
    installed_mod_names, mod_archive_names, mount_install, selected_mod,
};

/// A descriptor or `.nfo` larger than this is not read (the same limit as the mod check).
const MAX_DESCRIPTOR_BYTES: u64 = 1 << 20;

/// Where the descriptor (`.mod` and `.nfo`) of a mod was read from; the first of these that has
/// the file wins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DescriptorSource {
    /// A loose file of the install folder.
    Loose,
    /// A copy inside one of the mod's own archives (the archive's file name).
    ModArchive(String),
    /// The base game's archives and patches (the game's own `SeriousSam2.mod`).
    BaseArchives,
}

/// What kind of problem a [`ModProblem`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProblemKind {
    /// The mod has archives but no `.mod` file anywhere: it cannot be loaded.
    NoDescriptor,
    /// The `.mod` file cannot be read or parsed: the mod cannot be loaded.
    BadDescriptor,
    /// The `.nfo` file cannot be read or parsed: the mod is listed under its id.
    BadNfo,
    /// An archive of the mod cannot be opened: the mod cannot be mounted.
    UnreadableArchive,
}

/// Something wrong with one mod of the install; the scan goes on with the others.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModProblem {
    /// The mod's id (lower case).
    pub id: String,
    /// What kind of problem it is.
    pub kind: ProblemKind,
    /// What is wrong, in one sentence.
    pub message: String,
}

/// One mod of the install.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstalledMod {
    /// Display data and the parsed `.mod` file.
    pub info: ModInfo,
    /// The ids of the mods this one needs (`REQUIRES_MOD`, normalized).
    pub requires: Vec<String>,
    /// The folder `Content/<id>/` as it is on disk, if the mod has one.
    pub folder: Option<PathBuf>,
    /// The overlay archives that carry content, in mount order (later wins).
    pub archives: Vec<String>,
    /// Archives that hold only a `dummy.txt` (they mark an optional pack and are not mounted).
    pub placeholders: Vec<String>,
    /// Archives that could not be opened.
    pub unreadable: Vec<String>,
    /// Where the descriptor came from.
    pub descriptor: DescriptorSource,
}

impl InstalledMod {
    /// The mod's id (lower case).
    pub fn id(&self) -> &str {
        &self.info.id
    }

    /// `true` for the game itself.
    pub fn is_base(&self) -> bool {
        self.info.id == BASE_MOD_ID
    }

    /// The game modules (native code) the mod names besides the stock `Sam2Game`. SS2RS cannot
    /// load any of them: such a mod can only have its content used.
    pub fn native_modules(&self) -> Vec<&str> {
        self.info
            .manifest
            .game_modules()
            .filter(|m| !m.eq_ignore_ascii_case("Sam2Game"))
            .collect()
    }

    /// `true` if the mod needs no module of its own.
    pub fn is_data_only(&self) -> bool {
        self.native_modules().is_empty()
    }
}

/// What a scan of an install found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InstalledMods {
    /// The mods that can be listed, sorted by id (the base game is one of them).
    pub mods: Vec<InstalledMod>,
    /// What is wrong with mods that could not be listed or mounted, sorted by id.
    pub problems: Vec<ModProblem>,
    /// The id named by `Content/CurrentMod.txt`, or by `Content/DefaultMod.txt` without it.
    pub selected: Option<String>,
}

impl InstalledMods {
    /// The mod with this id (any case, with or without `.mod`).
    pub fn get(&self, id: &str) -> Option<&InstalledMod> {
        let id = normalize_mod_id(id);
        self.mods.iter().find(|m| m.info.id == id)
    }

    /// The nodes for [`crate::resolve_load_order`].
    pub fn nodes(&self) -> Vec<ModNode> {
        self.mods
            .iter()
            .map(|m| ModNode::new(m.info.id.clone(), m.requires.iter().cloned()))
            .collect()
    }

    /// What the install starts with: the selected mod, or nothing when that is the base game
    /// (or when no file selects one).
    pub fn default_request(&self) -> Vec<String> {
        match &self.selected {
            Some(id) if id != BASE_MOD_ID => vec![id.clone()],
            _ => Vec::new(),
        }
    }

    /// The load order for `requested`, see [`crate::resolve_load_order`].
    ///
    /// # Errors
    /// As [`crate::resolve_load_order`]; a mod that cannot be loaded (a problem of kind
    /// `NoDescriptor` or `BadDescriptor`) is unknown.
    pub fn plan(&self, requested: &[String]) -> std::result::Result<LoadPlan, LoadOrderError> {
        resolve_load_order(&self.nodes(), requested)
    }
}

impl InstalledMods {
    /// A plain ASCII listing: one line per mod, then the problems.
    pub fn render(&self) -> String {
        let mut out = format!(
            "Mods in the install: {}; the game starts: {}
",
            self.mods.len(),
            self.selected
                .as_deref()
                .map_or_else(|| "(not selected)".to_owned(), ascii)
        );
        for m in &self.mods {
            let native = m.native_modules();
            let module = if native.is_empty() {
                "data only".to_owned()
            } else {
                format!("native module {} (content only)", ascii(&native.join(", ")))
            };
            let needs = if m.requires.is_empty() {
                "-".to_owned()
            } else {
                ascii(&m.requires.join(", "))
            };
            let mut archives = format!("{} archive{}", m.archives.len(), plural(m.archives.len()));
            if !m.placeholders.is_empty() {
                archives.push_str(&format!(
                    " (+{} placeholder{})",
                    m.placeholders.len(),
                    plural(m.placeholders.len())
                ));
            }
            let _ = writeln!(
                out,
                "  {}: \"{}\"; {archives}; {module}; needs: {needs}",
                ascii(m.id()),
                ascii(&m.info.name)
            );
        }
        for p in &self.problems {
            let _ = writeln!(out, "  PROBLEM {}: {}", ascii(&p.id), ascii(&p.message));
        }
        out
    }
}

/// Look at the install folder `dir` (the folder with `All_PC_01.gro`) and list its mods.
///
/// Reads the base archives, the loose files of `Content/`, and the `.mod` / `.nfo` files and
/// archive directories of every `Content/<id>/` folder that holds `.gro` archives. A mod that is
/// damaged becomes a [`ModProblem`]; the others are still listed.
///
/// # Errors
/// As [`crate::mount_install`]: the install itself cannot be read.
pub fn scan_installed_mods(dir: impl AsRef<Path>) -> Result<InstalledMods> {
    let dir = dir.as_ref();
    let base = mount_install(dir)?;
    // A folder that cannot be mounted is no reason to refuse the archives: it only means that
    // no descriptor is loose.
    let loose = DirectoryVfs::new(dir).ok();
    let mut problems = Vec::new();

    // Every id that has a `.mod` file in the base archives or loose, or a folder with archives.
    let content = VfsPath::new("content")?;
    let mut ids: BTreeSet<String> = BTreeSet::new();
    let mut listed: Vec<&dyn Vfs> = vec![&base];
    listed.extend(loose.as_ref().map(|l| l as &dyn Vfs));
    for vfs in listed {
        match vfs.list(&content) {
            Ok(entries) => ids.extend(
                entries
                    .into_iter()
                    .filter(|e| e.kind == EntryKind::File)
                    .filter_map(|e| e.name.strip_suffix(".mod").map(str::to_owned)),
            ),
            Err(Error::NotFound(_)) => {}
            Err(error) => problems.push(ModProblem {
                id: String::new(),
                kind: ProblemKind::BadDescriptor,
                message: format!("Content/ cannot be listed ({error})"),
            }),
        }
    }
    ids.extend(installed_mod_names(dir).iter().map(|n| n.to_lowercase()));

    let mut mods = Vec::new();
    for id in ids {
        if let Some(found) = scan_one(dir, &id, &base, loose.as_ref(), &mut problems) {
            mods.push(found);
        }
    }
    problems.sort_by(|a, b| (&a.id, a.kind, &a.message).cmp(&(&b.id, b.kind, &b.message)));

    let mut stack = base;
    if let Some(loose) = loose {
        stack.mount("loose", loose);
    }
    Ok(InstalledMods {
        mods,
        problems,
        selected: selected_mod(&stack).ok(),
    })
}
/// Read `path` from the first of `sources` that has it. `None`: nowhere; `Some(Err(..))`: found
/// but unreadable or too large.
fn read_first(
    sources: &[(DescriptorSource, &dyn Vfs)],
    path: &VfsPath,
) -> Option<(DescriptorSource, std::result::Result<Vec<u8>, String>)> {
    for (source, vfs) in sources {
        match vfs.stat(path) {
            Ok(Stat {
                kind: EntryKind::File,
                size,
            }) => {
                let bytes = if size > MAX_DESCRIPTOR_BYTES {
                    Err(format!(
                        "{path} is too large to be a descriptor ({size} bytes)"
                    ))
                } else {
                    vfs.read(path)
                        .map_err(|e| format!("{path} cannot be read ({e})"))
                };
                return Some((source.clone(), bytes));
            }
            Ok(_) | Err(Error::NotFound(_)) => {}
            Err(error) => {
                return Some((
                    source.clone(),
                    Err(format!("{path} cannot be read ({error})")),
                ));
            }
        }
    }
    None
}

/// One mod: its archives, then its descriptor from the first place that has one.
fn scan_one(
    dir: &Path,
    id: &str,
    base: &dyn Vfs,
    loose: Option<&DirectoryVfs>,
    problems: &mut Vec<ModProblem>,
) -> Option<InstalledMod> {
    let mut problem = |kind: ProblemKind, message: String| {
        problems.push(ModProblem {
            id: id.to_owned(),
            kind,
            message,
        });
    };

    let mut folder = None;
    let mut opened: Vec<(String, ArchiveVfs)> = Vec::new();
    let mut placeholders = Vec::new();
    let mut unreadable = Vec::new();
    if let Ok(names) = mod_archive_names(dir, id) {
        folder = mod_folder(dir, id).ok();
        for name in names {
            let path = folder.as_ref().map(|f| f.join(&name));
            match path.map(ArchiveVfs::from_file) {
                Some(Ok(archive)) => {
                    let only_dummy = archive.file_count() == 1
                        && archive
                            .exists(&VfsPath::new("dummy.txt").expect("static path is valid"));
                    if only_dummy || archive.file_count() == 0 {
                        placeholders.push(name);
                    } else {
                        opened.push((name, archive));
                    }
                }
                Some(Err(error)) => {
                    problem(
                        ProblemKind::UnreadableArchive,
                        format!("the archive {name} cannot be opened ({error})"),
                    );
                    unreadable.push(name);
                }
                None => unreadable.push(name),
            }
        }
    }

    // Where a descriptor may be, best first: a loose file, the mod's own archives (the last
    // mounted first), the base game's archives.
    let mut sources: Vec<(DescriptorSource, &dyn Vfs)> = Vec::new();
    sources.extend(loose.map(|l| (DescriptorSource::Loose, l as &dyn Vfs)));
    for (name, archive) in opened.iter().rev() {
        sources.push((DescriptorSource::ModArchive(name.clone()), archive));
    }
    sources.push((DescriptorSource::BaseArchives, base));

    let mod_path = VfsPath::new(&format!("content/{id}.mod"));
    let Some((descriptor, bytes)) = mod_path
        .as_ref()
        .ok()
        .and_then(|path| read_first(&sources, path))
    else {
        problem(
            ProblemKind::NoDescriptor,
            format!("there is no Content/{id}.mod, so the mod cannot be loaded"),
        );
        return None;
    };
    let manifest = match bytes {
        Ok(bytes) => ModManifest::parse(&String::from_utf8_lossy(&bytes)),
        Err(message) => Err(Error::invalid(".mod file", message)),
    };
    let manifest = match manifest {
        Ok(manifest) => manifest,
        Err(error) => {
            problem(ProblemKind::BadDescriptor, format!("{id}.mod: {error}"));
            return None;
        }
    };

    let nfo = VfsPath::new(&format!("content/{id}.nfo"))
        .ok()
        .and_then(|path| read_first(&sources, &path))
        .and_then(|(_, bytes)| {
            match bytes
                .map_err(|m| Error::invalid(".nfo file", m))
                .and_then(|b| Nfo::parse(&b))
            {
                Ok(nfo) => Some(nfo),
                Err(error) => {
                    problem(ProblemKind::BadNfo, format!("{id}.nfo: {error}"));
                    None
                }
            }
        })
        .unwrap_or_default();
    Some(InstalledMod {
        requires: nfo
            .get(REQUIRES_MOD_KEY)
            .map(parse_mod_list)
            .unwrap_or_default(),
        info: ModInfo {
            name: nfo.get("NAME").map_or_else(|| id.to_owned(), str::to_owned),
            author: nfo.get("AUTHOR").map(str::to_owned),
            thumbnail: nfo.get("THUMBNAIL").and_then(|t| VfsPath::new(t).ok()),
            id: id.to_owned(),
            manifest,
        },
        folder,
        archives: opened.into_iter().map(|(name, _)| name).collect(),
        placeholders,
        unreadable,
        descriptor,
    })
}

/// One archive of a mounted mod.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MountedArchive {
    /// The mod's id.
    pub mod_id: String,
    /// The archive's file name.
    pub archive: String,
    /// The layer's name in [`MountedMods::vfs`] (`<mod id>/<archive>`).
    pub layer: String,
}

/// The base game with a load order of mods on top.
pub struct MountedMods {
    /// The mount stack: base archives and patches, then each mod's archives in load order.
    /// [`LayeredVfs::source_of`] says which layer provides a path.
    pub vfs: LayeredVfs,
    /// The load order that was mounted.
    pub plan: LoadPlan,
    /// The archives that were mounted on top of the base, in mount order.
    pub archives: Vec<MountedArchive>,
}

/// Mount the base archives of `dir`, then the archives of every mod of `plan`, in plan order.
///
/// # Errors
/// As [`crate::mount_install`]; [`Error::NotFound`] if a planned mod is not in `installed`;
/// [`Error::Invalid`] if a planned mod has an archive that cannot be opened (a mod is mounted
/// whole or not at all) or that is not valid.
pub fn mount_mods(
    dir: impl AsRef<Path>,
    installed: &InstalledMods,
    plan: &LoadPlan,
) -> Result<MountedMods> {
    let dir = dir.as_ref();
    let mut vfs = mount_install(dir)?;
    let mut archives = Vec::new();
    for planned in &plan.mods {
        for (name, archive) in open_mod_archives(dir, installed, &planned.id)? {
            let layer = format!("{}/{name}", planned.id);
            vfs.mount(layer.clone(), archive);
            archives.push(MountedArchive {
                mod_id: planned.id.clone(),
                archive: name,
                layer,
            });
        }
    }
    Ok(MountedMods {
        vfs,
        plan: plan.clone(),
        archives,
    })
}

/// The archives of the planned mod `id`, opened, in mount order.
pub(crate) fn open_mod_archives(
    dir: &Path,
    installed: &InstalledMods,
    id: &str,
) -> Result<Vec<(String, ArchiveVfs)>> {
    let Some(m) = installed.get(id) else {
        return Err(Error::NotFound(format!(
            "the mod `{id}` is not in the install"
        )));
    };
    if let Some(name) = m.unreadable.first() {
        return Err(Error::invalid(
            "mod archive",
            format!(
                "{}/{name} cannot be opened, so the mod is not mounted",
                m.id()
            ),
        ));
    }
    if m.archives.is_empty() {
        return Ok(Vec::new());
    }
    let folder = mod_folder(dir, m.id())?;
    m.archives
        .iter()
        .map(|name| Ok((name.clone(), ArchiveVfs::from_file(folder.join(name))?)))
        .collect()
}

#[cfg(test)]
pub(crate) mod fixture {
    use std::path::Path;

    use ss2_testkit::fixtures::ZipBuilder;

    pub(crate) fn write_gro(dir: &Path, name: &str, files: &[(&str, &str)]) {
        std::fs::create_dir_all(dir).unwrap();
        let mut b = ZipBuilder::new();
        for (n, d) in files {
            b = b.add(*n, d.as_bytes().to_vec());
        }
        std::fs::write(dir.join(name), b.finish().unwrap()).unwrap();
    }

    pub(crate) const BASE_MOD: &str = "MODULES = \r\n  Sam2Game\r\n  <TOOLS>Sam2GameGui\r\n;\r\n";

    /// A small install: the base game, `Alpha` (loose descriptor, two archives and a placeholder)
    /// and `Beta` (descriptor only inside its archive, needs Alpha).
    pub(crate) fn fake_install(root: &Path) {
        write_gro(
            root,
            "All_PC_01.gro",
            &[
                ("Content/x.txt", "base"),
                ("Content/b.txt", "base-b"),
                ("Content/SeriousSam2/level.wld", "wld"),
            ],
        );
        write_gro(
            root,
            "Patch_02_068.gro",
            &[
                ("Content/x.txt", "patch"),
                ("Content/SeriousSam2.mod", BASE_MOD),
                (
                    "Content/SeriousSam2.nfo",
                    "NAME = \"Serious Sam 2\" ;\nAUTHOR = \"Croteam\" ;",
                ),
                (
                    "Content/DefaultMod.txt",
                    "# default\r\nCURRENTMOD = SeriousSam2.mod ;\r\n",
                ),
            ],
        );
        let content = root.join("Content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(
            content.join("Alpha.mod"),
            "MODULES = Sam2Game <TOOLS>Sam2GameGui ;",
        )
        .unwrap();
        std::fs::write(
            content.join("Alpha.nfo"),
            "NAME = \"Alpha Mod\" ;\nAUTHOR = \"A. Author\" ;",
        )
        .unwrap();
        let alpha = content.join("Alpha");
        write_gro(
            &alpha,
            "Alpha.gro",
            &[
                ("Content/x.txt", "alpha"),
                ("Content/a.txt", "alpha-a"),
                ("Content/shared.txt", "same"),
            ],
        );
        write_gro(
            &alpha,
            "z_AlphaUpdate_01.gro",
            &[("Content/a.txt", "alpha-a2")],
        );
        write_gro(&alpha, "AlphaPack.gro", &[("dummy.txt", "")]);
        std::fs::write(alpha.join("ReadMe.txt"), "loose").unwrap();
        let beta = content.join("Beta");
        write_gro(
            &beta,
            "Beta.gro",
            &[
                ("Content/Beta.mod", "MODULES = BetaGame ;"),
                (
                    "Content/Beta.nfo",
                    "NAME = \"Beta\" ;\nREQUIRES_MOD = \"Alpha.mod; \" ;",
                ),
                ("Content/x.txt", "beta"),
                ("Content/shared.txt", "same"),
                ("Content/only-beta.txt", "beta-only"),
            ],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::*;
    use super::*;
    use crate::{Vfs, VfsPath, normalize_mod_id};

    fn p(s: &str) -> VfsPath {
        VfsPath::new(s).unwrap()
    }

    fn install() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fake_install(dir.path());
        dir
    }

    #[test]
    fn the_scan_finds_the_base_game_and_both_mods() {
        let dir = install();
        let found = scan_installed_mods(dir.path()).unwrap();
        let ids: Vec<_> = found.mods.iter().map(InstalledMod::id).collect();
        assert_eq!(ids, ["alpha", "beta", "serioussam2"]);
        assert!(found.problems.is_empty(), "{:?}", found.problems);
        assert_eq!(found.selected.as_deref(), Some("serioussam2"));

        let base = found.get("SeriousSam2.mod").unwrap();
        assert!(base.is_base());
        assert_eq!(base.info.name, "Serious Sam 2");
        assert_eq!(base.descriptor, DescriptorSource::BaseArchives);
        assert!(base.archives.is_empty() && base.folder.is_none());
        assert!(base.is_data_only());

        let alpha = found.get("ALPHA").unwrap();
        assert!(!alpha.is_base());
        assert_eq!(alpha.info.name, "Alpha Mod");
        assert_eq!(alpha.info.author.as_deref(), Some("A. Author"));
        assert_eq!(alpha.descriptor, DescriptorSource::Loose);
        assert_eq!(alpha.archives, ["Alpha.gro", "z_AlphaUpdate_01.gro"]);
        assert_eq!(alpha.placeholders, ["AlphaPack.gro"]);
        assert!(alpha.unreadable.is_empty());
        assert!(alpha.folder.as_ref().unwrap().ends_with("Alpha"));
        assert!(alpha.requires.is_empty());
        assert!(alpha.is_data_only());

        let beta = found.get("beta").unwrap();
        assert_eq!(
            beta.descriptor,
            DescriptorSource::ModArchive("Beta.gro".into())
        );
        assert_eq!(beta.requires, ["alpha"]);
        assert_eq!(beta.native_modules(), ["BetaGame"]);
        assert!(!beta.is_data_only());
        assert!(found.get("nope").is_none());
    }

    #[test]
    fn the_listing_is_plain_ascii_and_complete() {
        let dir = install();
        write_gro(
            &dir.path().join("Content/Orphan"),
            "Orphan.gro",
            &[("Content/o.txt", "o")],
        );
        let text = scan_installed_mods(dir.path()).unwrap().render();
        let expected = "Mods in the install: 3; the game starts: serioussam2
  alpha: \"Alpha Mod\"; 2 archives (+1 placeholder); data only; needs: -
  beta: \"Beta\"; 1 archive; native module BetaGame (content only); needs: alpha
  serioussam2: \"Serious Sam 2\"; 0 archives; data only; needs: -
  PROBLEM orphan: there is no Content/orphan.mod, so the mod cannot be loaded
";
        assert_eq!(text, expected);
        assert_eq!(
            InstalledMods::default().render(),
            "Mods in the install: 0; the game starts: (not selected)
"
        );
    }

    #[test]
    fn a_loose_descriptor_wins_over_the_copy_inside_the_mod() {
        let dir = install();
        std::fs::write(
            dir.path().join("Content/Beta.nfo"),
            "NAME = \"Loose Beta\" ;",
        )
        .unwrap();
        std::fs::write(dir.path().join("Content/Beta.mod"), "MODULES = Sam2Game ;").unwrap();
        let found = scan_installed_mods(dir.path()).unwrap();
        let beta = found.get("beta").unwrap();
        assert_eq!(beta.info.name, "Loose Beta");
        assert_eq!(beta.descriptor, DescriptorSource::Loose);
        assert!(beta.is_data_only());
        assert!(beta.requires.is_empty());
    }

    #[test]
    fn a_current_mod_file_changes_the_selection() {
        let dir = install();
        std::fs::write(
            dir.path().join("Content/CurrentMod.txt"),
            "CURRENTMOD = Alpha.mod ;",
        )
        .unwrap();
        let found = scan_installed_mods(dir.path()).unwrap();
        assert_eq!(found.selected.as_deref(), Some("alpha"));
        assert_eq!(found.default_request(), ["alpha"]);
        let plan = found.plan(&found.default_request()).unwrap();
        assert_eq!(plan.ids(), ["alpha"]);
    }

    #[test]
    fn the_default_request_is_empty_for_the_base_game() {
        let dir = install();
        let found = scan_installed_mods(dir.path()).unwrap();
        assert!(found.default_request().is_empty());
        assert!(found.plan(&[]).unwrap().mods.is_empty());
        std::fs::write(dir.path().join("Content/CurrentMod.txt"), "garbage").unwrap();
        let found = scan_installed_mods(dir.path()).unwrap();
        assert_eq!(found.selected, None);
        assert!(found.default_request().is_empty());
    }

    #[test]
    fn a_plan_pulls_in_what_a_mod_needs() {
        let dir = install();
        let found = scan_installed_mods(dir.path()).unwrap();
        let plan = found.plan(&["Beta".to_string()]).unwrap();
        assert_eq!(plan.ids(), ["alpha", "beta"]);
        assert_eq!(
            found.plan(&["nope".to_string()]).unwrap_err().to_string(),
            "the mod `nope` is not installed"
        );
        let nodes = found.nodes();
        assert_eq!(nodes.len(), 3);
        assert!(nodes.iter().all(|n| n.id == normalize_mod_id(&n.id)));
        assert_eq!(
            nodes.iter().find(|n| n.id == "beta").unwrap().requires,
            ["alpha"]
        );
    }

    #[test]
    fn damaged_mods_are_problems_and_the_others_are_still_listed() {
        let dir = install();
        let content = dir.path().join("Content");
        // No descriptor anywhere.
        write_gro(
            &content.join("Orphan"),
            "Orphan.gro",
            &[("Content/o.txt", "o")],
        );
        // A descriptor that does not parse.
        std::fs::write(content.join("Broken.mod"), "MODULES = A").unwrap();
        // A bad .nfo: the mod is still listed under its id.
        write_gro(
            &content.join("NfoBad"),
            "NfoBad.gro",
            &[("Content/n.txt", "n")],
        );
        std::fs::write(content.join("NfoBad.mod"), "MODULES = Sam2Game ;").unwrap();
        std::fs::write(content.join("NfoBad.nfo"), "NAME = \"open").unwrap();
        // An archive that is not a ZIP.
        write_gro(&content.join("Torn"), "Good.gro", &[("Content/t.txt", "t")]);
        std::fs::write(content.join("Torn/z_Torn.gro"), b"not a zip").unwrap();
        std::fs::write(content.join("Torn.mod"), "MODULES = Sam2Game ;").unwrap();

        let found = scan_installed_mods(dir.path()).unwrap();
        let kinds: Vec<_> = found
            .problems
            .iter()
            .map(|p| (p.id.as_str(), p.kind))
            .collect();
        assert_eq!(
            kinds,
            [
                ("broken", ProblemKind::BadDescriptor),
                ("nfobad", ProblemKind::BadNfo),
                ("orphan", ProblemKind::NoDescriptor),
                ("torn", ProblemKind::UnreadableArchive),
            ]
        );
        assert!(
            found.problems[0].message.contains("not closed"),
            "{:?}",
            found.problems[0]
        );
        assert!(found.problems[3].message.contains("z_Torn.gro"));
        let ids: Vec<_> = found.mods.iter().map(InstalledMod::id).collect();
        assert_eq!(ids, ["alpha", "beta", "nfobad", "serioussam2", "torn"]);
        assert_eq!(found.get("nfobad").unwrap().info.name, "nfobad");
        let torn = found.get("torn").unwrap();
        assert_eq!(torn.archives, ["Good.gro"]);
        assert_eq!(torn.unreadable, ["z_Torn.gro"]);
        // A mod that cannot be loaded is unknown to the load order.
        assert_eq!(
            found.plan(&["orphan".to_string()]).unwrap_err().to_string(),
            "the mod `orphan` is not installed"
        );
        assert_eq!(
            found.plan(&["broken".to_string()]).unwrap_err().to_string(),
            "the mod `broken` is not installed"
        );
        // ...and a mod with an unreadable archive is mounted whole or not at all.
        let plan = found.plan(&["torn".to_string()]).unwrap();
        let err = mount_mods(dir.path(), &found, &plan).err().unwrap();
        assert!(err.to_string().contains("z_Torn.gro"), "{err}");
    }

    #[test]
    fn a_descriptor_of_exactly_one_mebibyte_is_still_read() {
        let dir = install();
        let mut text = String::from("MODULES = Sam2Game ;");
        text.push_str(&" ".repeat((1 << 20) - text.len()));
        assert_eq!(text.len(), 1 << 20);
        std::fs::write(dir.path().join("Content/Edge.mod"), text).unwrap();
        let found = scan_installed_mods(dir.path()).unwrap();
        assert!(
            found.problems.iter().all(|p| p.id != "edge"),
            "{:?}",
            found.problems
        );
        assert!(found.get("edge").is_some());
    }

    #[test]
    fn an_oversized_descriptor_is_a_problem_not_a_read() {
        let dir = install();
        let big = vec![b' '; (1 << 20) + 1];
        std::fs::write(dir.path().join("Content/Huge.mod"), big).unwrap();
        let found = scan_installed_mods(dir.path()).unwrap();
        let huge = found.problems.iter().find(|p| p.id == "huge").unwrap();
        assert_eq!(huge.kind, ProblemKind::BadDescriptor);
        assert!(huge.message.contains("too large"), "{}", huge.message);
    }

    #[test]
    fn mounting_stacks_the_mods_in_plan_order_and_later_wins() {
        let dir = install();
        let found = scan_installed_mods(dir.path()).unwrap();
        let plan = found.plan(&["beta".to_string()]).unwrap();
        let mounted = mount_mods(dir.path(), &found, &plan).unwrap();
        let layers = mounted.vfs.layer_names();
        assert_eq!(
            layers,
            [
                "All_PC_01.gro",
                "Patch_02_068.gro",
                "alpha/Alpha.gro",
                "alpha/z_AlphaUpdate_01.gro",
                "beta/Beta.gro"
            ]
        );
        let archives: Vec<_> = mounted
            .archives
            .iter()
            .map(|a| (a.mod_id.as_str(), a.archive.as_str(), a.layer.as_str()))
            .collect();
        assert_eq!(
            archives,
            [
                ("alpha", "Alpha.gro", "alpha/Alpha.gro"),
                (
                    "alpha",
                    "z_AlphaUpdate_01.gro",
                    "alpha/z_AlphaUpdate_01.gro"
                ),
                ("beta", "Beta.gro", "beta/Beta.gro")
            ]
        );
        assert_eq!(mounted.plan, plan);
        assert_eq!(mounted.vfs.read(&p("content/x.txt")).unwrap(), b"beta");
        assert_eq!(mounted.vfs.read(&p("content/a.txt")).unwrap(), b"alpha-a2");
        assert_eq!(mounted.vfs.read(&p("content/b.txt")).unwrap(), b"base-b");
        assert_eq!(
            mounted.vfs.source_of(&p("content/x.txt")),
            Some("beta/Beta.gro")
        );
        assert_eq!(
            mounted.vfs.source_of(&p("content/a.txt")),
            Some("alpha/z_AlphaUpdate_01.gro")
        );
    }

    #[test]
    fn the_order_decides_who_wins() {
        let dir = install();
        let found = scan_installed_mods(dir.path()).unwrap();
        // `beta` needs `alpha`, so the dependency goes first whatever the request order.
        let plan = found
            .plan(&["beta".to_string(), "alpha".to_string()])
            .unwrap();
        assert_eq!(plan.ids(), ["alpha", "beta"]);
        // With no dependency the request order rules, and the last mod wins.
        let mut nodes = found.nodes();
        nodes.iter_mut().for_each(|n| n.requires.clear());
        let free = crate::resolve_load_order(&nodes, &["beta".into(), "alpha".into()]).unwrap();
        assert_eq!(free.ids(), ["beta", "alpha"]);
        let mounted = mount_mods(dir.path(), &found, &free).unwrap();
        assert_eq!(mounted.vfs.read(&p("content/x.txt")).unwrap(), b"alpha");
    }

    #[test]
    fn an_empty_plan_mounts_only_the_base_and_an_unknown_mod_is_refused() {
        let dir = install();
        let found = scan_installed_mods(dir.path()).unwrap();
        let mounted = mount_mods(dir.path(), &found, &LoadPlan::default()).unwrap();
        assert_eq!(
            mounted.vfs.layer_names(),
            ["All_PC_01.gro", "Patch_02_068.gro"]
        );
        assert!(mounted.archives.is_empty());
        assert_eq!(mounted.vfs.read(&p("content/x.txt")).unwrap(), b"patch");
        let ghost =
            crate::resolve_load_order(&[ModNode::new("ghost", [] as [&str; 0])], &["ghost".into()])
                .unwrap();
        let err = mount_mods(dir.path(), &found, &ghost).err().unwrap();
        assert!(matches!(err, Error::NotFound(_)), "{err:?}");
    }

    #[test]
    fn a_folder_that_is_not_an_install_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(scan_installed_mods(dir.path()).is_err());
    }

    /// Data-gated: the retail install and its two mods. Skips without `SS2_DATA_DIR`.
    #[test]
    fn the_real_install_lists_its_mods_and_mounts_them() {
        let dir = ss2_testkit::require_game_data!();
        let found = scan_installed_mods(&dir).unwrap();
        eprintln!(
            "mods: {:?}",
            found.mods.iter().map(InstalledMod::id).collect::<Vec<_>>()
        );
        assert!(found.problems.is_empty(), "{:?}", found.problems);
        let base = found.get(BASE_MOD_ID).expect("the base game");
        assert_eq!(base.info.name, "Serious Sam 2");
        assert_eq!(found.selected.as_deref(), Some(BASE_MOD_ID));
        for m in found.mods.iter().filter(|m| !m.is_base()) {
            let plan = found.plan(&[m.id().to_string()]).unwrap();
            let mounted = mount_mods(&dir, &found, &plan).unwrap();
            assert_eq!(mounted.archives.len(), m.archives.len(), "{}", m.id());
            eprintln!(
                "{}: {} archives, native {:?}",
                m.id(),
                m.archives.len(),
                m.native_modules()
            );
        }
        if let Some(r) = found.get("sam2renovation") {
            assert_eq!(r.info.name, "Renovation");
            assert!(r.is_data_only());
            assert_eq!(r.archives, ["Sam2Renovation.gro"]);
        }
        if let Some(i) = found.get("insamnity2") {
            assert_eq!(i.native_modules(), ["InSamnity2Game"]);
            assert_eq!(i.placeholders.len(), 8);
            assert_eq!(i.archives.len(), 8);
            assert_eq!(
                i.archives.last().map(String::as_str),
                Some("z_InSamnity2Update_12_100.gro")
            );
        }
    }
}
