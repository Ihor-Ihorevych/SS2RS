# Example mods

Invented, original content only (no Croteam material). Each folder is a loose mod folder that
`ss2-doctor --mod-dir` accepts; a test (`crates/ss2-vfs/tests/example_mods.rs`) runs the doctor on
every folder here and fails unless it exits 0 with no finding. Walk-through: `docs/modding/tutorial.md`.

| Folder | What it shows |
|---|---|
| `HelloMod/` | the smallest complete mod: `.mod`, `.nfo`, one episode description and one generated 8 x 8 texture |

The texture is made by `tools/asset-export/tests/example_mod.rs`
(`UPDATE_FIXTURES=1 cargo test -p asset-export --test example_mod` rewrites it). License of everything
in this folder: CC0-1.0.
