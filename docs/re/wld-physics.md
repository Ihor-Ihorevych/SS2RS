# `.wld` physics: mechanisms, bodies, hulls, joints (T01-012i)

- **Roadmap task:** T01-012i (part i of the world spec T01-012)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the object structure (mechanism, part tree, bodies, hulls, joints, motion handlers, the collision grid cells), for the class list and counts and for the relations below (checked on all 59 worlds with `tools/re/wld_physics_verify.py`; members come from the files' own type tables, see `wld-physics-schema.md`); **medium** for the roles of members that are only numbers or floats; **low** for physical meaning (units, how the solver uses limits, masses, damping): none of that is specified here, except as updated by **T01-066** below. **T01-066** (this task): **high** (binary, `Engine.dll`'s `CBody`/`CDynamicBody` accessor methods — `GetMass`, `SetMassProperties`, `GetVelocity`, `GetAngVelocity`, `GetForce`/`SetForce`/`AddForce`, `GetTorque`/`SetTorque`/`AddTorque`, `SetFrictionMultiplier`, `SetBounceMultiplier`, `IsKinematic`, `IsDynamic`, `IsHybrid`, all read in full — cross-checked against `wld-physics-schema.md`'s own member order and observed value ranges) for the roles of `CBody.5` to `.14`; **high** (data script, all 59 worlds, extending `wld_physics_verify.py`) that `CBody.1`/`CJoint.4`/`.6` is a single per-body linked list (not `.4` alone) and that it accounts for all 570 bodies with joints; **high** (same script) for which hulls are the 111 not listed by their mechanism and which entity classes own the 3,231 mechanisms beyond the 21,420 of `wld-geometry.md`; **high** (binary, `CForceFieldHull::GetFieldForce/GetFieldVelocity/GetFieldDirection`, `CFluidHull::GetFluidDensity`, read in full) for `CFluidHull.1` and `CForceFieldHull.4`-`.6`; **medium** (binary, `CMechanismMotionHandler::GetLerpRatio`, read in full) for how a motion handler drives a mechanism; **medium** (binary, `CHull::CanCollideWith`, `CCollisionDomain::FindCollisions`, `SeriousEngine::cldSetHullCategoryPairFlags`/`cldSetHullCategoryDefaultFlags`/`cldCopyCategory`, read in full, plus a name search of every shipped module) for the collision layer finding below; **low** still for `CBody.12`, `CFluidHull.2`, `CForceFieldHull.2`, the embedded editor mechanism templates and the joint classes' own numeric members beyond the two bodies and the chain. Scope: all 59 distinct worlds.

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
  .1  -> first joint of the body (570 bodies have joints; see "Joints" below for the chain through .4/.6)
  .5  FLOAT 1 .. 45,012 the body's mass (CDynamicBody::GetMass/CPrimitiveDesc::SetMassProperties; high, binary)
  .6  Vector3f the diagonal of the moment-of-inertia tensor (Ixx, Iyy, Izz), derived from the mass and the shape's CPrimitiveDesc by SetMassProperties (high, binary): CDynamicBody::GetInvI/GetI read it off-diagonal-free, i.e. the engine's rigid bodies never have a full (rotated) inertia tensor, only an axis-aligned diagonal one
  .7  Vector3f linear velocity (CDynamicBody::GetVelocity/SetVelocity; high, binary)
  .8  Vector3f angular velocity (CDynamicBody::GetAngVelocity/SetAngVelocity; high, binary)
  .9  Vector3f the accumulated force for the current step (CDynamicBody::GetForce/SetForce/AddForce, also written by CBody::AddImpulse; high, binary)
  .10 Vector3f the accumulated torque for the current step (CDynamicBody::GetTorque/SetTorque/AddTorque, also written by CBody::AddImpulse; high, binary)
  .11 INDEX 3 (CHybridBody) or 6 (CDynamicBody and CKinematicBody): CDynamicBody::GetDynamicDOF, the body's degrees of freedom (high, binary); a body being **kinematic** is a separate bit of the shared CAspect.6 flags (CBody::IsKinematic/IsDynamic test that bit, not .11): a hybrid body (3 DOF) is simulated as a single free axis (used by movers that slide or rotate on one axis) while dynamic and kinematic bodies both carry 6 DOF and are told apart only by the flag
  .12 Time (role not resolved; a per-body timestamp near the sleep-flag accessors, most likely when the body fell asleep or was last active — low, inference)
  .13 FLOAT the friction multiplier (0 or 1 observed; CBody::SetFrictionMultiplier; high, binary — scales whatever friction the touched material/layer would otherwise give, 1 default, 0 = frictionless)
  .14 FLOAT the bounce multiplier (0 or 1 observed; CBody::SetBounceMultiplier; high, binary — scales the restitution the touched material/layer would otherwise give, 1 default, 0 = no bounce)
CHull (CModelHull | CPrimitiveHull | CFluidHull | CForceFieldHull | CDummyHull)
  .1  -> body that carries the hull (6,683 hulls) or null (static hulls)
  .2  IDENT collision layer (below)
  .3 .4 .5 .6 .7  collision grid cell rectangle x0 x1 z0 z1 and level (wld-visibility.md)
  .9  -> the mechanism;  .10 INDEX the hull's part position;  .8 BOOL 0
CModelHull.1      -> CModelInstance naming the model whose mesh is the collision shape (wld-geometry.md)
CPrimitiveHull    .1 CPrimitiveDesc (PrimitiveType: 0 sphere, 1 box, 2 capsule, 3 cylinder — answered by T01-060, see wld-geometry.md), .2 material attributes (external .mat 3,538, embedded 373), .3 FLOAT, .4 BOOL
CFluidHull        .1 fluid density (1000, 500 or 980; CFluidHull::GetFluidDensity, high, binary), .2 (0.85 or 1.25; role not resolved — no accessor found; a buoyancy adjustment per physics.md's T01-023 reading, medium-low), .3 Box3f the fluid volume's local bounding box (CFluidHull::SetBox)
CForceFieldHull   .4 the field's force magnitude 2,000 to 100,000 (CForceFieldHull::GetFieldForce, high, binary), .5 the field's velocity 20 to 500 (CForceFieldHull::GetFieldVelocity, high, binary — most likely the speed the field accelerates a touched body towards, a cap rather than the force's own speed), .6 Vector3f the field's push direction (CForceFieldHull::GetFieldDirection, high, binary), .2 (0.5 or 0.85; role not resolved — shares its value range with CFluidHull.2, medium-low that it is the same kind of adjustment factor), .3 Box3f the field volume's local bounding box (`environment.md`)
```

- **A body carries one or several hulls.** 3,806 of 4,529 bodies have exactly one hull pointing at them; the other 723 carry several (6,683 hulls name a body). The body's first child aspect is the hull in 928 of the single-hull cases only: do not use `CAspect.2` to find the hulls; use `CHull.1`.
- **Dynamic bodies and `CPhysicsDomain.2`.** `.2` holds all 1,259 hybrid and 1,194 kinematic bodies and **205 of the 2,076 dynamic bodies** (118 of props, 61 of static model entities, 26 of turret characters). All 1,871 dynamic bodies that are not in `.2` belong to props. `.2` is therefore most likely the bodies that are active in the stored state (inference); `CBody.11`, `.13` and `.14` do not explain it (6, 1.0, 1.0 in all dynamic bodies).
- **Collision layers** (`CHull.2`, an identifier of the file): `solid` 23,241, `prop` 3,192, `item` 1,092, `playerdetector` 1,031, `barrier` 811, `item_no_gravity` 146, `character` 120, `killer` 104, `ignoreall` 97, `detector` 80, `bouncer` 60, empty 45, `fluid` 23, `character_through_barrier` 21, `custom_barrier` 14, `character_blocked_by_custom_barrier` 8. Which layer collides with which is not stored in the worlds (the `CLayerMap` objects are map kinds of meshes, `help-digest.md`, and unrelated to hull layers): it is a rule of the engine or of a database.
  - **T01-066: there is no layer-vs-layer interaction table to apply, and a loader does not need one.** `Engine.dll`'s `CHull::CanCollideWith` — the only gate `CCollisionDomain::FindCollisions` calls before testing two hulls — never reads `CHull.2` at all (binary, read in full). It rejects a pair only when (a) the two hulls share the same body (`CHull.1`, so a body's own hulls never collide with each other), or (b) both hulls belong to the same mechanism and the same mechanism part (self), or (c) both belong to the same mechanism at different parts and that pair of parts is in the mechanism's own "ignore" bit-matrix (a per-mechanism table, not named by any layer), or (d) the hulls belong to two different mechanisms and one mechanism's own ignore-list names the other (`CMechanism::IsIgnored`). The engine does expose a genuine, generic, interned-by-name pairwise category flag table (`SeriousEngine::cldSetHullCategoryPairFlags`, `cldSetHullCategoryDefaultFlags`, `cldCopyCategory`) that could implement a "solid blocks prop but not item" kind of table — but a name search of every shipped module (`Engine.dll`, `Sam2Game.dll`, `Sam2GameGui.dll`, `Sam2.exe`) found **no caller of any of the three**, so in the retail build that table is never populated and every one of the 16 named layers physically collides with every other (subject only to the body/mechanism rules above). A loader should therefore let all hulls collide regardless of `CHull.2`, and should implement `killer`/`barrier`/`detector`/`playerdetector`/`item`/`bouncer`/… as **gameplay rules keyed by the entity that owns the touched hull** (`CHull.1` → `CBody` → `CAspect.5`), not as a layer mask. One real, narrow exception was found by name: `Sam2Game.dll`'s `CPuppetEntity`'s ground/slope-alignment code literally string-compares a touched hull's `CHull.2` against `"solid"` and `"prop"` to decide whether to use it for step/slope detection — the only place any layer name is read as a string anywhere in the shipped binaries (binary, read in full; medium confidence that this is also the walker's own ground rule, since `CLeggedPuppetEntity` is a `CPuppetEntity`).
  - The `ray category` mechanism (`cldSetRayCategory`/`CPrimitiveHull`'s own ray-category use of `CHull.2`) is a **separate** per-query filter for ray casts (hit-scans), not a collision gate; not examined further here.
- **Hull shapes.** Primitive hulls use `PrimitiveType` 0 to 3, now named (sphere, box, capsule, cylinder) by T01-060; see `wld-geometry.md`. The 30,085 hulls cover cell rectangles of 2 × 2 (11,348), 2 × 1 (6,823), 1 × 2 (6,675) and 1 × 1 (5,239) cells.
- **T01-066: the 111 hulls not listed by their mechanism** are all `CPrimitiveHull`: 81 carry no body at all (free-standing primitives whose own `CHull.9` names a mechanism that simply never lists them — most likely stray editor leftovers), and the other 30 carry a body owned by an `SAnimatedMoverEntity` (data script, all 59 worlds; high confidence, exact count).
- **T01-066: the 3,231 mechanisms beyond the 21,420 `CStaticModelEntity`s of `wld-geometry.md`** are not a separate mechanism kind: walking every mechanism's renderable (`CMechanism.6`) back to its owning entity (`CAspect.5`) shows they belong to ordinary moving or pickup entities that also carry a model with a physical hull — `SPropEntity` (1,726), `SDoorEntity` (176), and a long tail of ammo/health/armor/treasure pickups (`SShellsItemEntity` 143, `SHealthItemXSEntity` 119, `SHealthItemMEntity` 105, `SBulletsItemEntity` 69, `SArmorItemXSEntity` 65, `SRocketsItemEntity` 61, `SHandGrenadesItemEntity` 57, and more, each a few dozen) (data script, all 59 worlds; high confidence).

## Joints

```
CJoint (every joint class)
  .1  -> owner entity;  .2 JointFlags (0 or 1);  .3 -> body (the half's own body)
  .5  -> second body (set in 295 of 594 halves);  .6 -> partner half (209 halves);  .4 -> a joint (next in a chain);  .7 FLOAT -1 or 1e7
class members: two QuatVect anchor frames (one per body), limits (CJointLimit: minimum, maximum, tolerance 1e-5, flags), arrays of 3 to 12 floats
```

- **Two halves.** A joint between two bodies is stored as **two objects of the same kind** (167 of the 209 partners share the class): the secondary half has `.6` → the primary half and `.5` → the primary's body; **no pair is mutual** (the primary's `.6` is null in all 209 cases). In the test vector below the hinge of a turret is the pair of halves 2272 and 2273.
- **T01-066: the chain rule of `CBody.1`/`CJoint.4` is resolved: `.4` and `.6` are the same kind of field, picked by which of a joint's two body slots the body occupies.** `Engine.dll`'s `CJoint::GetNextInBody(body)` (binary, read in full) returns `.4` when `body` is that joint's `.3`, and `.6` when `body` is that joint's `.5` — so `CJoint.6` is not solely "the partner-half pointer" (that reading was a special case): it is also the generic "next joint of this body's list" pointer used from the secondary-body side. `CBody.1` is the head of a **singly-linked list of every joint touching that body**, walked by following `.4` while the body is the current joint's `.3` and switching to `.6` once the body is the current joint's `.5`. Re-running `tools/re/wld_physics_verify.py` with this alternating walk (added by this task) matches the full set of joints naming the body (`.3` or `.5`) in **570 of 570** bodies with joints, over all 59 worlds — not 366 of 570 as the single-field walk gave (data script, high confidence).
- **Joint classes** with their own members (from the schema): hinge (two frames, a limit, 8 floats), cone (two frames, a limit, 3 vectors, two angles of 0.035, 0.087 or 0.52 radians, a quaternion, 11 floats), ball (two frames, 3 floats), slider (two frames, a limit, 7 floats), generic motor (five limits, frames, 12 floats), wheel (two limits, two vectors, floats 1e6 and 5e4), linear motion (frames, vectors, floats 3,000 to 1.8e6 and 15,000 to 4.5e6), fixed (no members of its own). Their physical meaning beyond the two bodies and the chain (limit units, the float arrays) is **not resolved by T01-066** either; no accessor for them was found in the time available.
- **Editor objects.** `CTestJointEntity` (93, in 16 worlds) and `CJointRenderable` (136) exist for editing and testing joints; `CGenericMotorJointTemplate`, the `CEditMechanism…` and `CMechanismTemplateHolder` objects are mechanism templates embedded by the editor in 2 to 8 worlds (T01-057). Their own numeric members (`CEditMechanismTemplate.2` 100 or 50, the generic-motor template's fixed ±3e38 limits) were not investigated further by T01-066: they read as editor defaults/placeholders, not run-time data.
- **T01-066: how a motion handler drives a mechanism.** `Engine.dll`'s `CMechanismMotionHandler::GetLerpRatio` (binary, read in full) computes `clamp((currentTime − startTime) × rate, 0, 1)`, where `startTime` is read from the handler's own stored `Time` (`CMechanismMotionHandler.2`) and `rate` is a single **global** engine constant, not a per-handler value: every mechanism motion handler interpolates over the same fixed duration. `Evaluate` (the function that actually moves the mechanism's parts, not read in full: it is large and was not fully traced in this task) presumably uses this ratio to blend between two placements per part. `CMechanismMotionHandler.3` (BOOL, always 0 in the data) was not resolved. Medium confidence: the ratio formula and the single shared global rate are confirmed; which two placements it blends between, and why `.3` is always 0 in retail, are not.

## Reader invariants

1. Walk `CMechanism.1` as a tree by `CMechanismPart.7` (parent index; the root is −1 and parents come first); a part has a hull or a body, not both.
2. A hull with a body (`CHull.1`) moves with it; a hull without one is static. A body can carry several hulls: collect them through `CHull.1`.
3. Find a hull's grid cells in the hull (`CHull.3` to `.7`); the grid is a quadtree with child slot `bx + 2*bz` (`wld-visibility.md`).
4. Read joints as halves: follow `.6` from the secondary to the primary; a half with `.5` set connects two bodies.
5. Use `CPhysicsDomain.1` for all bodies; read `.2` as the bodies active in the stored state (inference).
6. Fail on an unknown hull, body or joint class; the classes in `wld-physics-schema.md` are the complete list of the retail data.
7. **T01-066:** to list every joint touching a body, start at `CBody.1` and alternate: follow the current joint's `.4` while the body is that joint's `.3`, or its `.6` once the body is that joint's `.5` (confirmed over all 570 bodies with joints, all 59 worlds).
8. **T01-066:** do not gate collision on `CHull.2` (the layer name) — the retail game does not use it for that; reject a hull pair only when they share a body (`CHull.1`), or share a mechanism part, or their mechanism(s) say to ignore each other (`CMechanism`'s own per-part and per-mechanism ignore lists). Use the owning entity (`CHull.1` → `CBody` → `CAspect.5`), not the layer name, to tell `killer`/`barrier`/`detector`/`playerdetector`/etc. apart for gameplay.

## Test vector

`Content/SeriousSam2/Levels/Multiplayer/DM_Yodeller/DM_Yodeller.wld` (4.5 MB): an `STurretCharacterEntity` (object 248) owns a `CDynamicBody` 1619 (`CAspect.5` = 248) whose hull is the `CPrimitiveHull` 1768 (`CHull.1` = 1619) and whose `CBody.1` is the joint 2273. The hinge joint pair: half 2272 is `CHingeJoint` with `.1` = 248, flags `0x1`, `.3` = 1619, `.5` and `.6` null, limit −3.229 to 3.229; half 2273 has `.3` = 1620 (`CDynamicBody`), `.5` = 1619, `.6` = 2272 and a limit of −0.08727 to 0.3491 (−5 to 20 degrees). `CPhysicsDomain.1` lists both bodies.

## Open questions

Resolved by **T01-066** (moved out of this list): the roles and units of the `CBody` members, the names of the `PrimitiveType`s (T01-060), the roles of `CFluidHull.1` and `CForceFieldHull.4`-`.6`, the collision layer interaction question (there isn't one in the retail binaries — see "Collision layers" above), the chain rule of `CBody.1`/`CJoint.4`/`.6`, the 111 hulls not listed by their mechanism, how a motion handler computes its blend ratio, and the mechanisms beyond the 21,420 owned by `wld-geometry.md`'s entities.

Still open:

- The physical meaning of the joint limit and per-class array members (hinge's 8 floats, cone's 11, generic motor's 12, …): no accessor was found for them; likely needs instrumented capture (T01-038/039) rather than more binary reading.
- `CBody.12` (a `Time`, role not resolved — most likely a sleep/last-active timestamp, not confirmed), `CFluidHull.2` and `CForceFieldHull.2` (both a small float in the 0.5–1.25 range on the same two classes, most likely a buoyancy/field-strength adjustment, not confirmed by an accessor).
- The 208 hulls whose position differs from `CHull.10`, and the 42 joint halves whose partner has another class.
- Which dynamic bodies are in `CPhysicsDomain.2` and why (data confirms the split by owner entity class — props, static models, turret characters — but not the underlying rule; `CBody.11`, `.13`, `.14` do not explain it).
- The embedded mechanism templates' own numeric members (`CEditMechanismTemplate.2`, the generic-motor template's fixed limits) beyond "they are editor defaults" (T01-057), the collision meshes owned by meshes (`mdl.md`, `bmf.md`), and `CMechanismMotionHandler.3` and the body of `Evaluate` that actually applies the lerp ratio to a mechanism's parts.
- The ray-category use of `CHull.2` (hit-scans, `cldSetRayCategory`) was located but not traced further; it is a separate mechanism from hull-to-hull collision.
