# testdata/asset-export

Tiny `.tex` and `.bmf` fixtures for the `asset-export` tool (T11-014). **Provenance:** hand-made by the
builders in `tools/asset-export/src/fixtures.rs` (the testkit's `MetaBuilder`, written from the public
specs `docs/re/tex.md` and `docs/re/bmf.md`); no Croteam content, no excerpt of any game file. A test
(`tools/asset-export/tests/cli.rs`) checks that the files equal what the builders produce;
`UPDATE_FIXTURES=1 cargo test -p asset-export --test cli` rewrites them.

| File | What |
|---|---|
| `bgr_8x4.tex` | 24-bit BGR ramp, 8 x 4 |
| `bgra_4x4.tex` | 32-bit BGRA with varying alpha, 4 x 4 |
| `dxt1_4x4.tex` | DXT1, 4 x 4, three mip levels (red, green, blue blocks) |
| `cube_4x4.tex` | DXT1 cube map, six flat-colour faces, three levels each |
| `quad.bmf` | one LOD, one quad (positions, normals, one UV set) |
| `rich.bmf` | two LODs; three surfaces (embedded empty material, tangents, a skinned one) |
