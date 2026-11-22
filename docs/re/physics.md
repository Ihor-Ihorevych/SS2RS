# Physics and collision: no middleware, and what the engine's own system contains (T01-023)

- **Roadmap task:** T01-023
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** that the game uses **no third-party physics library** (imports, strings and class names all agree); **high** for the feature inventory (class names and method names, with the data classes cross-checked in `classes.md`); **low** for how the solver works (not examined; method names only suggest it).

Prerequisites: `classes.md`, `help-digest.md` (mechanism and body types), `misc-types.md` (`.mch`).

## Summary

**Serious Sam 2 does not use Havok, PhysX/Novodex, ODE, Newton, Bullet, Tokamak or any other physics middleware.** Physics and collision are part of the engine itself (`Engine.dll`, namespace `SeriousEngine`): a **mechanism** system (`CMechanism` made of parts with bodies, hulls and joints), a **collision domain** with a grid, and a **physics domain** with a multi-threaded task pool. For our reimplementation this means two things:

1. There is no external library whose behavior we could reuse or must match bit by bit; physics behavior has to be **measured** from the game (T01-024, T01-038/039) and re-created in `ss2-physics`.
2. There is no third-party physics license to consider for the original; the open-source physics options (for example Rapier) are our free choice (T07).

## Evidence

All three checks looked at names only:

- **Imports.** `Engine.dll` imports `Core.dll`, `steam_api.dll`, `XINPUT9_1_0.dll`, `USER32`, `GDI32`, `WSOCK32`, `WINMM`, `KERNEL32`, `MSVCR71`. `Sam2Game.dll` imports `Core`, `Engine`, `Shaders`, `ProcRender`, `steam_api`, `MSVCR71`, `KERNEL32`. No physics DLL is imported or shipped in `Bin/` (the DLL list is in `inventory.md`).
- **Strings.** None of the modules contains a library-specific identifier (`Havok`, `hkp…`, `Nx…`, `PhysX`, `Novodex`, `dWorld…`/`dBody…` as ODE functions, `Newton…`, `btRigidBody`, …). The few raw hits of short words such as `ode_` or `bullet` are inside ordinary words (`mode_`, `Bullet` ammunition names).
- **Own classes.** The modules **export their C++ symbols with decorated names** (for example `ActivatePhysicsInRange` of `CMechanism`, in namespace `SeriousEngine`), and the physics classes are all Croteam classes: they also appear in the data (`classes-catalog.md`, `.mch` mechanism templates, `.wld` mechanism and body objects).
- Copyright and version strings in the install name the third-party code that *is* present (see the last section); none is a physics library.

## What the engine's physics contains (names found in `Engine.dll`)

| Area | Classes | Notes |
|---|---|---|
| **Mechanisms** | `CMechanism` (60 exported methods), `CMechanismPart`, `CMechanismPartParameters` (`…Wheel`), `CMechanismPartMassProperties`, `CMechanismMotionHandler`, templates `CMechanismTemplate`, `CMechanismPartTemplate`, `CMechanismShapeTemplate` | a tree of parts built from a template (`.mch`) or from a mesh (`CreateMeshMechanism`); parts are created with a body, hulls and a joint to the parent; mass is distributed over parts; bone-following parts evaluate animation |
| **Bodies** | `CBody` base; `CDynamicBody`, `CHybridBody`, `CKinematicBody` | forces, torques, impulses, velocity, inertia and inverse inertia, mass properties; gravity switch, bounce multiplier, friction multiplier, **random friction**, **damping**, **quick sleep** and a sleep flag |
| **Hulls** (collision shapes) | `CHull` base; `CPrimitiveHull`, `CModelHull`, `CFluidHull`, `CForceFieldHull`, `CDummyHull` | primitive hulls (sphere, box, capsule, cylinder, cone, ellipsoid per the help file), model hulls from meshes, **fluid** hulls with a **buoyancy** adjustment, **force-field** hulls (`Fluid`, `Force field` entities), dummy hulls; hulls have a collision **category** and a "can collide with" test, ray checks and move checks |
| **Collision mesh** | `CCollisionMesh` with `…Node`, `…Edge`, `…Surface` | triangle-mesh collision for static scenery (`GetTrianglesNearBox`); the collision layer of meshes |
| **Joints** | `CJoint` base with `CFixedJoint`, `CBallJoint`, `CConeJoint`, `CHingeJoint`, `CSliderJoint`, `CLinearMotionJoint`, `CAngularMotionJoint`, `CGenericMotorJoint`, `CContactJoint`, `CBalanceJoint`, `CManipulatorJoint`, `CWheelJoint`, `CHovercraftJoint`, `CAircraftJoint`, `CTurretJoint`, `CParachutingJoint`, each with a template class | limits (`CJointLimit`), break force, desired position (motors), stored and restored joint solutions (apparently for warm starting a solver), a joint list per body |
| **Domains** | `CCollisionDomain` (add/remove/move hulls, active hulls, **find collisions**, hulls in range, a grid), `CPhysicsDomain` | the world owns both (`wld-root.md`); activation by range: `ActivatePhysicsInRange` on hulls, parts and aspects, so distant objects sleep |
| **Threads** | `CPhysicsThreadPool`, `CPhysicsThread`, the service `PhysicsTaskPool` | physics runs on a pool of worker threads |
| **Data side** | `CDestructionConfig`, `CDestructionData`, `CDestructionHolder`, `CDFX*` | destruction: breakable objects with effects and substitute models (`help-digest.md`; T01-046); **ragdolls** are mechanism templates named `Ragdoll`, `RagdollSimple` in `.mch` (`misc-types.md`) |

**Features the game uses** (T01-023's list): rigid bodies with joints (dynamic, hybrid and kinematic bodies), **ragdolls**, **destruction**, buoyancy and fluids, force fields, vehicles (wheeled, hovercraft, aircraft, turret, parachute, balance joints), triangle-mesh collision for level geometry plus primitive hulls for movers, ray and swept-move queries, sleeping and activation by distance, breakable joints, motors, multithreaded stepping. **Features not seen**: cloth, soft bodies, particles with physics beyond simple collision (particles have their own bounce rule, `pfx.md`), vehicles with suspension tables beyond the wheel joint.

The solver itself (how bodies are integrated, how contacts are resolved, whether joints are solved by an iterative constraint method) is **not** specified here. The quantities named in the joint classes (`GetJik`, `GetMaxm`, `GetNuc`) and the "store/restore solutions" calls point to a constraint solver with warm starting, but that is a guess from names.

## Exports are a source of names

Every shipped module exports its decorated C++ symbols (about 8,000 in `Engine.dll`, belonging to 453 classes). They give **class and method names** (but not signatures in a readable form, and no behavior). They are useful as a vocabulary (T01-026 entity classes, T01-063 class layouts) and for checking that a feature exists; they must not be used as a source of code (`conventions.md`).

## Other third-party code in the install (for the license review)

| Component | Where | Evidence |
|---|---|---|
| zlib (deflate/inflate 1.1.3) | `Core.dll` | the library's copyright banner; `CzlibCompressor`; the `.gro` archives are zip files |
| Ogg Vorbis | `CodecOGG.dll` | `CVorbisSoundFeeder`, `COGGStreamingSound` |
| Lua 5.1 | `ScriptLua.dll` | Lua's version string; the engine has a Lua scripting module besides the macro language |
| Direct3D 9 extensions | `d3dx9_25/26/27.dll` | shipped redistributables |
| XInput | `XINPUT9_1_0.dll` (imported) | controller input on Windows |
| Steamworks | `steam_api.dll`, `Steam.dll`, `steamclient.dll` | Valve's runtime, not part of the game |
| `amp11lib.dll` | `Bin/` | name only; purpose not examined |
| Microsoft C runtime 7.1 | `msvcr71.dll`, `msvcp71.dll` | redistributable |

None of this changes the license of SS2RS (it reimplements behavior and uses permissive Rust crates, `DEPENDENCIES.md`); it matters for a future decision about whether the original's `Lua` scripts (T13) can be supported.

## Open questions

1. The numeric behavior: integration step, solver iterations, gravity, damping defaults, contact friction and bounce rules: measured in T01-024 (player movement) and T01-038/039 (instrumentation), then specified.
2. Which data feeds the global physics parameters (`CWorld.SetupGlobalPhysicsParams` and the `CPhysicsDomain` members; `wld-root.md` lists the members, not their meaning).
3. Whether `ScriptLua.dll` is used by the shipped game at all (`inventory.md` lists no `.lua` files).

## Test vectors

Counts to reproduce with a symbol listing of `Engine.dll` (names only): 453 exported classes; `CMechanism` 60 methods; each body class 19; `CBody` 30; `CJoint` 21; `CHull` 21; 16 joint kinds each with a template class; 5 hull kinds.
