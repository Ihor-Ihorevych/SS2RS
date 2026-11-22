# Summary

[Introduction](introduction.md)

- [Architecture overview](architecture.md)
- [Design](superpowers/specs/2026-11-21-ss2rs-design.md)

# User guide

- [Install and point SS2RS at your game data](user/install.md)
- [FAQ](user/faq.md)
- [Troubleshooting](user/troubleshooting.md)

# Contributing

- [Coding standards](CODING_STANDARDS.md)
- [Developer tools](DEV_TOOLS.md)
- [Cheat commands](CHEATS.md)
- [Fuzzing and Miri](FUZZING.md)
- [Time synchronization](TIMESYNC.md)
- [Input command stream](INPUTSTREAM.md)
- [Snapshot buffer](SNAPSHOTS.md)
- [Reliable events](EVENTS.md)
- [Netcode architecture guide](NETCODE.md)
- [Net statistics](NETSTATS.md)
- [Culling and TAA jitter](CULLING.md)
- [Network threat model](THREAT_MODEL_NET.md)
- [Network protocol versioning](PROTOCOL.md)
- [Testing strategy](TESTING.md)
- [Data-gated CI design](DATA_GATED_CI.md)
- [Parity with the original game](PARITY.md)
- [Performance budgets and benchmarks](PERF_BUDGETS.md)
- [Profiling](PROFILING.md)
- [Releasing](RELEASING.md)
- [PC packaging and distribution](PACKAGING.md)
- [Publishing crates](PUBLISHING.md)
- [PC 1.0 release checklist](PC_1_0_CHECKLIST.md)
- [Bug triage](TRIAGE.md)
- [Playtest program](PLAYTEST.md)
- [Accessibility review and checklist](ACCESSIBILITY.md)
- [Localization QA pass](LOCALIZATION_QA.md)
- [Modding policy](MODDING_POLICY.md)
- [Dependencies and advisories](DEPENDENCIES.md)
- [Compliance procedure](COMPLIANCE.md)
- [Clean-room audit log](compliance/clean-room-log.md)

# Roadmap

- [Index and rules of engagement](roadmap/README.md)
  - [T00 Repo, infrastructure, contracts](roadmap/T00-repo-infra.md)
  - [T01 Reverse engineering and specs](roadmap/T01-reverse-engineering.md)
  - [T02 Data formats](roadmap/T02-data-formats.md)
  - [T03 Platform](roadmap/T03-platform.md)
  - [T04 Renderer](roadmap/T04-renderer.md)
  - [T05 Shaders](roadmap/T05-shaders.md)
  - [T06 Audio and video](roadmap/T06-audio-video.md)
  - [T07 World and physics](roadmap/T07-world-physics.md)
  - [T08 Simulation and netcode](roadmap/T08-sim-netcode.md)
  - [T09 Gameplay](roadmap/T09-gameplay.md)
  - [T10 UI](roadmap/T10-ui.md)
  - [T11 Tools](roadmap/T11-tools.md)
  - [T12 QA and release](roadmap/T12-qa-release.md)
  - [T13 Mods](roadmap/T13-mods.md)

# RFCs

- [RFC template](rfcs/0000-template.md)
- [0001 Vfs and asset API](rfcs/0001-vfs-and-asset-api.md)
- [0002 Renderer front-end API](rfcs/0002-render-frontend-api.md)
- [0003 Platform API](rfcs/0003-platform-api.md)
- [0004 Audio API](rfcs/0004-audio-api.md)
- [0005 ECS choice](rfcs/0005-ecs-choice.md)
- [0006 Network transport](rfcs/0006-network-transport.md)

# Reverse-engineering specs

- [Overview and template](re/README.md)
- [Spec conventions](re/conventions.md)
- [Help file digest](re/help-digest.md)
- [Install inventory](re/inventory.md)
- [CTSEMETA container](re/ctsemeta-container.md)
- [CTSEMETA type tables](re/ctsemeta-types.md)
