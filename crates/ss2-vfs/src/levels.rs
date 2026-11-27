//! The levels of an install: every level `.nfo` under [`LEVELS_DIR`], paired with the `.wld` next
//! to it (RFC 0013, `docs/re/misc-types.md`).
//!
//! A level folder holds the world and its description, for example
//! `content/serioussam2/levels/01_mdigbo/1_1_jungle/` with `1_1_jungle.wld` and a `.nfo` whose
//! first line is the kind word `LEVEL`. The level's id is the folder's name (the `.nfo` file name
//! differs in places) and its group is the folder above.
//!
//! ```
//! use ss2_vfs::{MemoryVfs, levels::discover_levels};
//!
//! let vfs = MemoryVfs::from_files([
//!     ("content/serioussam2/levels/multiplayer/dm_yard/dm_yard.wld", &b""[..]),
//!     ("content/serioussam2/levels/multiplayer/dm_yard/yard.nfo",
//!      &b"LEVEL\nNAME = \"The Yard\"\nMAP_TYPE = \"DEATHMATCH; COOP;\"\n"[..]),
//! ]).unwrap();
//! let levels = discover_levels(&vfs).unwrap();
//! assert_eq!(levels[0].id, "dm_yard");
//! assert_eq!(levels[0].name, "The Yard");
//! assert_eq!(levels[0].map_types, ["DEATHMATCH", "COOP"]);
//! assert_eq!(levels[0].wld.as_str(), "content/serioussam2/levels/multiplayer/dm_yard/dm_yard.wld");
//! ```

use ss2_core::Result;

use crate::{EntryKind, Nfo, Vfs, VfsPath};

/// Where the game keeps its levels (lower case, as [`VfsPath`] normalizes it).
pub const LEVELS_DIR: &str = "content/serioussam2/levels";

/// One playable level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelInfo {
    /// The level's folder name, e.g. `"1_1_jungle"` (lower case).
    pub id: String,
    /// The folder above it, e.g. `"01_mdigbo"` or `"multiplayer"`; empty directly under
    /// [`LEVELS_DIR`].
    pub group: String,
    /// The `.nfo` `NAME`; empty when missing.
    pub name: String,
    /// The `MAP_TYPE` entries, upper case, in file order, e.g. `["SINGLE", "COOP"]`.
    pub map_types: Vec<String>,
    /// The world file in the same folder.
    pub wld: VfsPath,
    /// The level's `.nfo`, for keys this list does not copy (thumbnail, movies, loading screen).
    pub nfo: VfsPath,
}

impl LevelInfo {
    /// `true` if `MAP_TYPE` lists `kind` (compared without case), e.g. `"SINGLE"`.
    pub fn has_map_type(&self, kind: &str) -> bool {
        self.map_types.iter().any(|t| t.eq_ignore_ascii_case(kind))
    }
}

/// Every level of `vfs`, sorted by group, then id.
///
/// A level `.nfo` (kind word `LEVEL`) without a `.wld` beside it is left out. When a folder
/// holds several worlds, the one named like the folder wins, otherwise the first by name. A vfs
/// without [`LEVELS_DIR`] has no levels.
///
/// # Errors
/// A directory that cannot be listed or an `.nfo` that cannot be read or parsed.
pub fn discover_levels(vfs: &dyn Vfs) -> Result<Vec<LevelInfo>> {
    let root = VfsPath::new(LEVELS_DIR)?;
    let mut levels = Vec::new();
    if vfs.exists(&root) {
        walk(vfs, &root, &root, &mut levels)?;
    }
    levels.sort_by(|a, b| (&a.group, &a.id).cmp(&(&b.group, &b.id)));
    Ok(levels)
}

/// Visit `dir` and everything below it, collecting levels.
fn walk(vfs: &dyn Vfs, root: &VfsPath, dir: &VfsPath, out: &mut Vec<LevelInfo>) -> Result<()> {
    let mut entries = vfs.list(dir)?;
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    let worlds: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::File && e.name.ends_with(".wld"))
        .map(|e| e.name.as_str())
        .collect();
    for e in &entries {
        let path = dir.join(&e.name)?;
        if e.kind == EntryKind::Dir {
            walk(vfs, root, &path, out)?;
            continue;
        }
        if !e.name.ends_with(".nfo") {
            continue;
        }
        let nfo = Nfo::parse(&vfs.read(&path)?)?;
        if nfo.kind.as_deref() != Some("LEVEL") {
            continue; // an episode or another description
        }
        let id = dir.file_name().unwrap_or_default().to_string();
        let own = format!("{id}.wld");
        let Some(world) = worlds
            .iter()
            .find(|w| **w == own)
            .or_else(|| worlds.first())
        else {
            continue;
        };
        let group = match dir.parent() {
            Some(parent) if parent != *root => parent.file_name().unwrap_or_default().to_string(),
            _ => String::new(),
        };
        let map_types = nfo
            .get("MAP_TYPE")
            .unwrap_or("")
            .split(';')
            .map(|t| t.trim().to_ascii_uppercase())
            .filter(|t| !t.is_empty())
            .collect();
        out.push(LevelInfo {
            wld: dir.join(world)?,
            id,
            group,
            name: nfo.get("NAME").unwrap_or("").to_string(),
            map_types,
            nfo: path,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryVfs;

    const L: &str = "content/serioussam2/levels";

    fn level_nfo(name: &str, map_type: &str) -> Vec<u8> {
        format!("LEVEL\nNAME = \"{name}\"\nMAP_TYPE = \"{map_type}\"\n").into_bytes()
    }

    fn vfs(files: &[(&str, Vec<u8>)]) -> MemoryVfs {
        let mut v = MemoryVfs::new();
        for (p, b) in files {
            v.insert(p, b.clone()).unwrap();
        }
        v
    }

    #[test]
    fn levels_are_found_named_and_sorted() {
        let v = vfs(&[
            (&format!("{L}/02_magnor/2_1_a/2_1_a.wld"), vec![]),
            (
                &format!("{L}/02_magnor/2_1_a/2_1_a.nfo"),
                level_nfo("A", "SINGLE; COOP;"),
            ),
            (&format!("{L}/01_mdigbo/1_2_b/1_2_b.wld"), vec![]),
            (
                &format!("{L}/01_mdigbo/1_2_b/other_name.nfo"),
                level_nfo("B", "SINGLE"),
            ),
            (&format!("{L}/01_mdigbo/1_1_c/1_1_c.wld"), vec![]),
            (
                &format!("{L}/01_mdigbo/1_1_c/1_1_c.nfo"),
                level_nfo("C", "single;"),
            ),
            (
                &format!("{L}/01_mdigbo/01_mdigbo.nfo"),
                b"EPISODE\nNAME = \"M'Digbo\"\n".to_vec(),
            ),
        ]);
        let got = discover_levels(&v).unwrap();
        let ids: Vec<(&str, &str, &str)> = got
            .iter()
            .map(|l| (l.group.as_str(), l.id.as_str(), l.name.as_str()))
            .collect();
        assert_eq!(
            ids,
            [
                ("01_mdigbo", "1_1_c", "C"),
                ("01_mdigbo", "1_2_b", "B"),
                ("02_magnor", "2_1_a", "A")
            ]
        );
        assert_eq!(got[2].map_types, ["SINGLE", "COOP"]);
        assert_eq!(got[0].map_types, ["SINGLE"], "upper-cased");
        assert!(got[0].has_map_type("single"));
        assert!(!got[0].has_map_type("COOP"));
        assert_eq!(
            got[1].nfo.as_str(),
            format!("{L}/01_mdigbo/1_2_b/other_name.nfo")
        );
        assert_eq!(
            got[1].wld.as_str(),
            format!("{L}/01_mdigbo/1_2_b/1_2_b.wld")
        );
    }

    #[test]
    fn a_level_without_a_world_is_left_out_and_a_missing_folder_has_no_levels() {
        let v = vfs(&[(&format!("{L}/g/x/x.nfo"), level_nfo("X", "SINGLE"))]);
        assert!(discover_levels(&v).unwrap().is_empty());
        assert!(discover_levels(&MemoryVfs::new()).unwrap().is_empty());
    }

    #[test]
    fn the_world_named_like_the_folder_wins_and_a_missing_name_is_empty() {
        let v = vfs(&[
            (&format!("{L}/g/lvl/aaa.wld"), vec![]),
            (&format!("{L}/g/lvl/lvl.wld"), vec![]),
            (
                &format!("{L}/g/lvl/lvl.nfo"),
                b"LEVEL\nMAP_TYPE = \"DM\"\n".to_vec(),
            ),
            (&format!("{L}/g/two/b.wld"), vec![]),
            (&format!("{L}/g/two/a.wld"), vec![]),
            (&format!("{L}/g/two/two.nfo"), level_nfo("Two", "")),
        ]);
        let got = discover_levels(&v).unwrap();
        assert_eq!(got[0].wld.as_str(), format!("{L}/g/lvl/lvl.wld"));
        assert_eq!(got[0].name, "");
        assert_eq!(
            got[1].wld.as_str(),
            format!("{L}/g/two/a.wld"),
            "first by name"
        );
        assert!(got[1].map_types.is_empty());
    }

    #[test]
    fn a_level_directly_under_the_levels_folder_has_an_empty_group() {
        let v = vfs(&[
            (&format!("{L}/solo/solo.wld"), vec![]),
            (&format!("{L}/solo/solo.nfo"), level_nfo("Solo", "SINGLE")),
        ]);
        let got = discover_levels(&v).unwrap();
        assert_eq!((got[0].group.as_str(), got[0].id.as_str()), ("", "solo"));
    }

    #[test]
    fn the_levels_of_the_real_install() {
        let dir = ss2_testkit::require_game_data!();
        let v = crate::mount_install(&dir).unwrap();
        let levels = discover_levels(&v).unwrap();
        // misc-types.md: 58 level descriptions; game-modes.md: 42 of them single player.
        assert_eq!(levels.len(), 58);
        assert_eq!(
            levels.iter().filter(|l| l.has_map_type("SINGLE")).count(),
            42
        );
        for l in &levels {
            assert!(v.exists(&l.wld), "{} names a missing world", l.id);
            assert!(!l.name.is_empty(), "{} has no name", l.id);
        }
        let first = &levels[0];
        assert_eq!(
            (first.group.as_str(), first.id.as_str()),
            ("01_mdigbo", "1_1_jungle")
        );
        let dock = levels.iter().find(|l| l.id == "dm_dockingbay").unwrap();
        // This folder also holds `dockingbay.wld`; the world named like the folder is the level.
        assert!(
            dock.wld
                .as_str()
                .ends_with("dm_dockingbay/dm_dockingbay.wld"),
            "{}",
            dock.wld
        );
        assert!(dock.has_map_type("DEATHMATCH") || dock.has_map_type("DM"));
    }
}
