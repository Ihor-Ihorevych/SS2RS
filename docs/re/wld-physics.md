# `.wld` physics: mechanisms, bodies, hulls, joints (T01-012i)

- **Roadmap task:** T01-012i (part i of the world spec T01-012)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the object structure (mechanism, part tree, bodies, hulls, joints, motion handlers, the collision grid cells), for the class list and counts and for the relations below (checked on all 59 worlds with `tools/re/wld_physics_verify.py`; members come from the files' own type tables, see `wld-physics-schema.md`); **medium** for the roles of members that are only numbers or floats; **low** for physical meaning (units, how the solver uses limits, masses, damping): none of that is specified here. Scope: all 59 distinct worlds.

Prerequisites: `wld-root.md` (domains, grids), `wld-geometry.md` (hulls, mechanisms of static models), `wld-visibility.md` (the grid cell layout), `physics.md` (what the engine's physics contains: no middleware), `wld-entities.md`.

## Summary

- Physics data in a world is a set of **mechanisms**: collision objects are described by **mechanisms** (`CMechanism`, 24,651 over all worlds; `wld-geometry.md` counts 21,420 entities that own one), each a **tree of parts**; every part carries either a **hull** (a static collision shape, 21,623 parts) or a **body** (a simulated or moved object, 4,252 parts), never both. **Hulls** (30,085) are the shapes: model hulls (the model's collision mesh), primitives, force fields, fluids, dummies. **Bodies** (4,529) are dynamic (2,076), hybrid (1,259) or kinematic (1,194); each carries its hulls.
- **Joints** (594 halves of 8 classes; the hinge occurs in 16 worlds, the most widespread) connect bodies; each joint is stored as **two halves** (a primary and a secondary that points at it).
- The **collision grid** indexes every hull by the same quadtree cells as the renderable grids (`wld-visibility.md`); the cells of a hull are stored in the hull itself (`CHull.3` to `.7`).
- The engine's own system is described in `physics.md`; this spec is only the stored data. The simulation (integration, contacts, joint limits, damping) is **not** specified.

## Evidence

`tools/re/wld_physics_verify.py` (read-only, `SS2_DATA_DIR`; about 15 minutes) and `tools/re/wld_entity_schema.py --match …` over the retail install with all patches:

- **Mechanisms:** 24,651, each with a renderable (`CMechanism.6`) and **exactly one root part**; all 25,875 parts have a parent index that is −1 (root, 24,651) or the index of an earlier part of the same mechanism.
- **Parts:** 21,623 have a hull and no body, 4,252 a body and no hull; none have both or neither; every referenced hull or body is an object of the world.
- **Hulls:** 30,085; 27,820 name a mechanism (`CHull.9`) and 27,709 of those are listed by that mechanism's parts (directly or through their body); the other 111 are not analysed. `CHull.10` equals the hull's part position in 27,501 of the 27,709.
- **Bodies:** `CPhysicsDomain.1` is exactly the list of all bodies in all 59 worlds; `.2` is a subset (below). 6,683 hulls name a body (`CHull.1`), always a body of the world.
- **Collision grid:** in all 59 worlds every cell rectangle stored in a hull (`CHull.3` to `.7`) has a link to that hull at the node reached by the walk of `wld-visibility.md`, the used links are exactly the (hull, cell) pairs, and no hull covers more than 2 × 2 cells.
- **Motion handlers:** 538 `CMechanismMotionHandler`, each named back by its mechanism (`CMechanism.3`): 538 of 538.

## The classes

| Group | Classes (instances) |
|---|---|
| Mechanism | `CMechanism` 24,651 (a parts list, not a class of its own: `CMechanismPart` is stored inside it) |
| Bodies (derive `CBody`, which derives `CAspect`) | `CDynamicBody` 2,076, `CHybridBody` 1,259, `CKinematicBody` 1,194 |
| Hulls (derive `CHull`, which derives `CAspect`) | `CModelHull` 22,019, `CPrimitiveHull` 7,804, `CDummyHull` 201, `CForceFieldHull` 38, `CFluidHull` 23 |
| Joints (derive `CJoint`) | `CHingeJoint` 259, `CConeJoint` 97, `CBallJoint` 78, `CSliderJoint` 55, `CGenericMotorJoint` 43, `CWheelJoint` 26, `CLinearMotionJoint` 20, `CFixedJoint` 16 (594 halves; hinge in 16 worlds) |
| Motion handlers (derive `CMotionHandler`) | `CMechanismMotionHandler` 538 (45 worlds); `CSpineAdjuster` 1,825 (44 worlds; a bone adjuster of characters, not physics: the count equals the number of legged characters) |
| Editor leftovers | `CTestJointEntity` 93 (an entity with a joint and a joint renderable), `CJointRenderable` 136, `CGenericMotorJointTemplate` 129, `CMechanismTemplateHolder` 33, `CEditMechanism…Template` 16 to 18 each in 2 or 3 worlds, `CPrimitiveHullTemplate` 36 |
| Other | `CCollisionMesh` 743 (owned by meshes, `CMesh.2`: the collision layer of a model's mesh, `mdl.md`), `CCollisionDomain`, `CCollisionGrid`, `CPhysicsDomain` (59 each) |

## Mechanism, parts, hulls, bodies

```
CMechanism
  .1  list of CMechanismPart (1 to 25 parts)
  .6  -> CModelRenderable   the renderable the mechanism moves (all mechanisms)
  .3  -> CMechanismMotionHandler (538 mechanisms) or null
  .7  -> CMaterialAttributes: external .mat (1,542) or an embedded CSamMaterial (77); null in 23,032
  .9  IDENT  Default | Static | empty;   .4 FLOAT 0 .. 45,000;   .10 QuatVect, .12 Vector3f   placement data (wld-geometry.md)
CMechanismPart
  .1  IDENT name ('' in 22,057, Main 1,854, Root 637, others 1,327)
  .7  INDEX  parent part: -1 (the root) or the index of an earlier part
  .5  -> hull   (static part)        .3  -> body   (moving part)         exactly one of the two is set
  .2  -> joint  (458 parts)          .12 QuatVect  placement relative to the parent;   .8 null;   .11 BOOL
CBody (CDynamicBody | CHybridBody | CKinematicBody; the three classes add no members)
  CAspect: .2 the first child aspect, .5 the owning entity, .4/.7 placement, .6 flags (as in wld-root.md)
  .1  -> first joint of the body (570 bodies have joints)
  .5  FLOAT 1 .. 45,012 (probably the mass);  .6 .. .10 Vector3f (30.0 in a sample: size or inertia, not determined);  .11 INDEX 3 or 6;  .12 Time;  .13/.14 FLOAT 0 or 1
CHull (CModelHull | CPrimitiveHull | CFluidHull | CForceFieldHull | CDummyHull)
  .1  -> body that carries the hull (6,683 hulls) or null (static hulls)
  .2  IDENT collision layer (below)
  .3 .4 .5 .6 .7  collision grid cell rectangle x0 x1 z0 z1 and level (wld-visibility.md)
  .9  -> the mechanism;  .10 INDEX the hull's part position;  .8 BOOL 0
CModelHull.1      -> CModelInstance naming the model whose mesh is the collision shape (wld-geometry.md)
CPrimitiveHull    .1 CPrimitiveDesc (PrimitiveType 0 to 3 + three floats), .2 material attributes (external .mat 3,538, embedded 373), .3 FLOAT, .4 BOOL
CFluidHull        .1 density-like float (1000 or 500), .2 0.85, .3 Box3f;   CForceFieldHull  .4 3000 to 6000, .5 FLOAT, .6 Vector3f, .2, .3 as the fluid
```

- **A body carries one or several hulls.** 3,806 of 4,529 bodies have exactly one hull pointing at them; the other 723 carry several (6,683 hulls name a body). The body's first child aspect is the hull in 928 of the single-hull cases only: do not use `CAspect.2` to find the hulls; use `CHull.1`.
- **Dynamic bodies and `CPhysicsDomain.2`.** `.2` holds all 1,259 hybrid and 1,194 kinematic bodies and **205 of the 2,076 dynamic bodies** (118 of props, 61 of static model entities, 26 of turret characters). All 1,871 dynamic bodies that are not in `.2` belong to props. `.2` is therefore most likely the bodies that are active in the stored state (inference); `CBody.11`, `.13` and `.14` do not explain it (6, 1.0, 1.0 in all dynamic bodies).
- **Collision layers** (`CHull.2`, an identifier of the file): `solid` 23,241, `prop` 3,192, `item` 1,092, `playerdetector` 1,031, `barrier` 811, `item_no_gravity` 146, `character` 120, `killer` 104, `ignoreall` 97, `detector` 80, `bouncer` 60, empty 45, `fluid` 23, `character_through_barrier` 21, `custom_barrier` 14, `character_blocked_by_custom_barrier` 8. Which layer collides with which is not stored in the worlds (the `CLayerMap` objects are map kinds of meshes, `help-digest.md`, and unrelated to hull layers): it is a rule of the engine or of a database.
- **Hull shapes.** Primitive hulls use `PrimitiveType` 0 to 3 (the names are not in the data; T01-060). The 30,085 hulls cover cell rectangles of 2 × 2 (11,348), 2 × 1 (6,823), 1 × 2 (6,675) and 1 × 1 (5,239) cells.

## Joints

```
CJoint (every joint class)
  .1  -> owner entity;  .2 JointFlags (0 or 1);  .3 -> body (the half's own body)
  .5  -> second body (set in 295 of 594 halves);  .6 -> partner half (209 halves);  .4 -> a joint (next in a chain);  .7 FLOAT -1 or 1e7
class members: two QuatVect anchor frames (one per body), limits (CJointLimit: minimum, maximum, tolerance 1e-5, flags), arrays of 3 to 12 floats
```

- **Two halves.** A joint between two bodies is stored as **two objects of the same kind** (167 of the 209 partners share the class): the secondary half has `.6` → the primary half and `.5` → the primary's body; **no pair is mutual** (the primary's `.6` is null in all 209 cases). In the test vector below the hinge of a turret is the pair of halves 2272 and 2273.
- **Body to joints.** `CBody.1` names a joint that names the body in 561 of 570 bodies with joints; the chain through `CJoint.4` has one element in 483 and equals the set of joints naming the body (`.3` or `.5`) in 366 of 570: the chain rule is not fully reconciled.
- **Joint classes** with their own members (from the schema): hinge (two frames, a limit, 8 floats), cone (two frames, a limit, 3 vectors, two angles of 0.035, 0.087 or 0.52 radians, a quaternion, 11 floats), ball (two frames, 3 floats), slider (two frames, a limit, 7 floats), generic motor (five limits, frames, 12 floats), wheel (two limits, two vectors, floats 1e6 and 5e4), linear motion (frames, vectors, floats 3,000 to 1.8e6 and 15,000 to 4.5e6), fixed (no members of its own). Their physical meaning is not specified.
- **Editor objects.** `CTestJointEntity` (93, in 16 worlds) and `CJointRenderable` (136) exist for editing and testing joints; `CGenericMotorJointTemplate`, the `CEditMechanism…` and `CMechanismTemplateHolder` objects are mechanism templates embedded by the editor in 2 to 8 worlds (T01-057).

## Reader invariants

1. Walk `CMechanism.1` as a tree by `CMechanismPart.7` (parent index; the root is −1 and parents come first); a part has a hull or a body, not both.
2. A hull with a body (`CHull.1`) moves with it; a hull without one is static. A body can carry several hulls: collect them through `CHull.1`.
3. Find a hull's grid cells in the hull (`CHull.3` to `.7`); the grid is a quadtree with child slot `bx + 2*bz` (`wld-visibility.md`).
4. Read joints as halves: follow `.6` from the secondary to the primary; a half with `.5` set connects two bodies.
5. Use `CPhysicsDomain.1` for all bodies; read `.2` as the bodies active in the stored state (inference).
6. Fail on an unknown hull, body or joint class; the classes in `wld-physics-schema.md` are the complete list of the retail data.

## Test vector

`Content/SeriousSam2/Levels/Multiplayer/DM_Yodeller/DM_Yodeller.wld` (4.5 MB): an `STurretCharacterEntity` (object 248) owns a `CDynamicBody` 1619 (`CAspect.5` = 248) whose hull is the `CPrimitiveHull` 1768 (`CHull.1` = 1619) and whose `CBody.1` is the joint 2273. The hinge joint pair: half 2272 is `CHingeJoint` with `.1` = 248, flags `0x1`, `.3` = 1619, `.5` and `.6` null, limit −3.229 to 3.229; half 2273 has `.3` = 1620 (`CDynamicBody`), `.5` = 1619, `.6` = 2272 and a limit of −0.08727 to 0.3491 (−5 to 20 degrees). `CPhysicsDomain.1` lists both bodies.

## Open questions

- Units and meaning of the body floats (`CBody.5` to `.14`), the primitive types, the layer interactions, the joint limit and array members, and how a motion handler drives a mechanism (`CMechanismMotionHandler.2`, `.3`): measurement and the solver spec (T01-038, T01-039, T01-057).
- The 111 hulls whose mechanism does not list them, the 208 hulls whose position differs from `CHull.10`, the chain rule of `CBody.1`/`CJoint.4`, and the 42 halves whose partner has another class.
- Which dynamic bodies are in `CPhysicsDomain.2` and why (props only are left out).
- The embedded mechanism templates (T01-057), the collision meshes owned by meshes (`mdl.md`, `bmf.md`), and the 3,231 mechanisms beyond the 21,420 owned by the entities of `wld-geometry.md` (their owners were not analysed).
