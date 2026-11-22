# ECS spike (T08-001)

Throwaway benchmark behind `docs/rfcs/0005-ecs-choice.md`: `hecs`, `bevy_ecs` and a minimal sparse-set store on the same workload
(50,000 entities; spawn, movement pass, snapshot by a typed query and through a registry, churn with stale-handle checks, and an
iteration-order fingerprint).

```
cd spikes/ecs
cargo run --release
```

It has its own `[workspace]` and `Cargo.lock`: it is **not** part of the main workspace, CI or `cargo deny`, and nothing in it
ships. Delete it once the `ss2-sim` facade exists. Numbers depend on the machine; the RFC lists the ones it quotes and where they
came from.
