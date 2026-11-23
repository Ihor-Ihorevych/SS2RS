# Physics spike (T07-006)

Throwaway benchmark behind `docs/rfcs/0008-physics-engine.md`: our query layer (`ss2-physics`) and `rapier3d` 0.36 on the same scenes (a triangle-mesh terrain with 100,000 seeded rays, 2,000 moving
spheres, boxes dropped on the terrain, a ragdoll chain, a kinematic character controller, and a determinism hash).

```
cd spikes/physics
cargo run --release
cargo run --release --features enhanced   # rapier's cross-platform determinism mode
```

Every result is a line `key value`. It has its own `[workspace]` and `Cargo.lock`: it is **not** part of the main workspace, CI or `cargo deny`, and nothing in it ships. Delete it once the rigid-body
module exists. Numbers depend on the machine; the RFC lists the ones it quotes and where they came from.
