# tools/re — reproducible evidence for the specs in docs/re/

Plain Python 3 scripts (standard library only) that read an install's `.gro` archives **read-only**. They exist so a reviewer can re-run the numbers quoted in the specs.

```
python -I tools/re/validate_ctsemeta.py "C:\path\to\Serious Sam 2"        # frame validation counts
python -I tools/re/walk_objects.py "C:\path\to\Serious Sam 2"             # object-data decode: how many files decode to EDOB
python -I tools/re/inventory.py "C:\path\to\Serious Sam 2" inventory.md   # regenerate docs/re/inventory.md
```

They never write game content. Scripts that analyze the original *binaries* do not belong here (CLEANROOM.md).

Texture evidence (docs/re/tex.md):

```
python -I tools/re/tex_verify.py "C:\path\to\Serious Sam 2"       # size checks for every .tex, flag/wrap/dimension statistics
```

`ctse_values.py` is the value decoder these scripts share (objects -> Python values).

Mesh and extra-record evidence (docs/re/bmf.md, ctsemeta-objects.md):

```
python -I tools/re/bmf_verify.py "C:\path\to\Serious Sam 2"    # per-surface layout checks for every .bmf
python -I tools/re/edob_verify.py "C:\path\to\Serious Sam 2"   # the EDOB extra-record rule for every non-world file
```

Skeleton evidence (docs/re/skl.md):

```
python -I tools/re/skl_verify.py "C:\path\to\Serious Sam 2"    # hierarchy rules and the local-transform / inverse-bind check
```

Model evidence (docs/re/mdl.md):

```
python -I tools/re/mdl_verify.py "C:\path\to\Serious Sam 2"    # structure, embedded meshes, palettes, children, references (about 5 minutes)
```

Animation evidence (docs/re/ans.md):

```
python -I tools/re/ans_verify.py "C:\path\to\Serious Sam 2"    # keyframe rules for every envelope, bone ids, events (about 5 minutes)
```

Material evidence (docs/re/mtr.md):

```
python -I tools/re/mtr_verify.py "C:\path\to\Serious Sam 2"    # preset structure, shader class codes, texture slots, modifiers (about 5 minutes)
```

World evidence (docs/re/ctsemeta-objects.md, "CMetaHandle" and "The EDOB section"):

```
python -I tools/re/wld_verify.py "C:\path\to\Serious Sam 2"    # all 59 worlds: objects, extra records, CMetaHandle values (about 20 minutes)
```

`walk_objects.py` now reports 9,269 of 9,269 file entries decoding (about 11 minutes).

World root evidence (docs/re/wld-root.md):

```
python -I tools/re/wld_root_verify.py "C:\path\to\Serious Sam 2" [N]    # relations of the CWorld root in all 59 worlds (about 20 minutes); N = only the N smallest
```

World geometry evidence (docs/re/wld-geometry.md):

```
python -I tools/re/wld_geometry_verify.py "C:\path\to\Serious Sam 2" [N]    # static models, hulls, collision grid cells in all 59 worlds (about 20 minutes)
```

World lighting evidence (docs/re/wld-lighting.md):

```
python -I tools/re/wld_lighting_verify.py "C:\path\to\Serious Sam 2" [N]    # lights, baked lightmaps, flares in all 59 worlds (about 20 minutes)
```

Entity parameter evidence (docs/re/ep.md, docs/re/ep-schema.md):

```
python -I tools/re/ep_schema.py "C:\path\to\Serious Sam 2" --check [--worlds N]    # structure, references, entity links (all worlds: about 20 minutes)
python -I tools/re/ep_schema.py "C:\path\to\Serious Sam 2" --markdown docs/re/ep-schema.md    # regenerate the schema reference
```

Font evidence (docs/re/fnt.md):

```
python -I tools/re/fnt_verify.py "C:\path\to\Serious Sam 2" [--png OUT_DIR]    # glyph tables, code coverage; optional rendered samples
```

`texpng.py` decodes a `CStaticTexture` (tex.md) to RGBA and writes PNG files; the other scripts import it.
