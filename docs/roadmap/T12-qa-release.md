# T12 — QA, Performance, Packaging & Release

PC first. Mobile budgets/devices/packaging are Phase 2.

- [ ] **T12-001** · Test strategy doc: unit / synthetic-fixture / data-gated / golden-image / golden-audio / sim-scenario / netsim / soak / fuzz layers; what runs on PR vs nightly · `S` · `P0`
- [ ] **T12-002** · Data-gated CI design: self-hosted runner(s) operated by contributors with legal copies; data never leaves the machine; results only (pass/fail/coverage); trust model for fork PRs · `M` · `P0`
- [ ] **T12-003** · Parity checklist template per level/weapon/enemy; signed off in the PR that completes the task; evidence stored as text/CSV, not game assets · `S` · `P1` · 🟢
- [ ] **T12-004** · PC performance budgets (frame ms, draw calls, memory, load time) on 2–3 reference PCs (low/mid/high); automated perf runs on fixed scenes, regression alerts · `M` · `P1`
- [ ] **T12-007** · Nightly fuzz + sanitizer jobs (ASan/UBSan for any `unsafe`, Miri on selected crates) · `M` · `P1`
- [ ] **T12-008** · Nightly/release artifacts for Windows/Linux/macOS; reproducible builds; checksums; changelog · `M` · `P1`
- [ ] **T12-009** · PC packaging & distribution: GitHub Releases, Windows zip/installer, AppImage/Flatpak, macOS `.app`; no game assets in any package · `M` · `P2`
- [ ] **T12-010** · Compliance: license audit/SBOM per release, clean-room audit log (RE spec ↔ implementer), DCO check · `M` · `P1`
- [ ] **T12-011** · Playtest program: build channels, structured feedback form (no telemetry by default), crash-report triage · `M` · `P2`
- [ ] **T12-012** · Accessibility review (input remap, subtitles, contrast, motion) · `S` · `P2`
- [ ] **T12-013** · Localization QA pass (text overflow, fonts) · `S` · `P3`
- [ ] **T12-014** · User docs: "Install & point to your data", FAQ, troubleshooting, supported game versions (Steam/GOG/retail, patch level) · `M` · `P1` · 🟢
- [~] **T12-015** [INPRG @Ihor-Ihorevych/agent · branch `T12-015-bug-triage` · 2026-11-22] · Bug triage process, severity definitions, issue templates (crash, desync, visual parity, gameplay parity) · `S` · `P1` · 🟢
- [ ] **T12-016** · Soak test farm: nightly long co-op bot sessions on a dedicated server; leak/desync detectors · `L` · `P2`
- [ ] **T12-017** · PC 1.0 release checklist & go/no-go criteria (perf, parity, crash-free %, legal) · `S` · `P2`

## PHASE 2 — Android & other platforms
- [ ] **T12-005** · Mobile memory budgets (4 GB-class devices): per-subsystem caps, texture streaming limits, OOM recovery tests · `M` · `PHASE 2`
- [ ] **T12-006** · Android device matrix: tiers (low/mid/high; Adreno/Mali/PowerVR), Vulkan vs GLES, OS versions; community device-lab process · `M` · `PHASE 2`
- [ ] **T12-018** · Android distribution: signed APK/AAB, F-Droid/Play policy review (user-supplied data), update channel · `M` · `PHASE 2`
- [ ] **T12-019** · Mobile perf budgets & thermal test protocol (sustained 20-minute runs) · `M` · `PHASE 2`
- [ ] **T12-020** · Cross-play test matrix PC ↔ Android (protocol versions, latency profiles) · `M` · `PHASE 2`
