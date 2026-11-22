# Frequently asked questions

Answers come from the project's own documents. Where a feature is not built yet, the answer says so
and names the roadmap task.

## Legal and clean room

**Does SS2RS include Serious Sam 2?**
No. The repository and the release packages contain no game content: no `.gro` archives, `.dll` or
`.exe` files from the game, and no textures, models, sounds, movies or levels. See the
[NOTICE](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/NOTICE) file and the
[README](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/README.md).

**Do I need the game?**
Yes. SS2RS is designed to read the data of your own, legally obtained copy of Serious Sam 2. See
[Install and point SS2RS at your game data](install.md).

**Is SS2RS made by Croteam?**
No. It is an unofficial, independent project. It is not affiliated with, endorsed by or sponsored by
Croteam or Devolver Digital.

**How was the engine written?**
Under the [clean-room policy](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CLEANROOM.md). Specs
are written from the game's files and from observation, and implementers write Rust from those specs.
Decompiler output is not used for code, and no code from Serious Sam 1 is copied.

**Can I put game files in a mod or a bug report?**
No. The [modding policy](../MODDING_POLICY.md) forbids mods that contain game files. The
[bug triage rules](../TRIAGE.md#8-game-content-in-reports) forbid attaching them to reports.

## What works today

**Can I play Serious Sam 2 with SS2RS now?**
No. The project is pre-alpha. The window shows a placeholder menu that uses no game data. The
engine does not load levels, render the game world or run the campaign yet.

**What is done and what is next?**
The [roadmap](../roadmap/README.md) lists every task with its state (`[x]` done, `[~]` claimed,
`[ ]` open). The milestones are in [section 5](../roadmap/README.md#5-milestones-and-exit-criteria):
M1 reads the data, M2 shows a textured model in a window, M3 walks through a level, and so on up to
M8, PC 1.0. The project does not publish dates for these milestones.

**Can I read the game's files with SS2RS?**
Yes, partly. `gro-tool` lists, searches and prints the files in the archives (`stats`, `find`,
`cat`, `list`). It is a developer tool that runs from a source checkout.

## Multiplayer and co-op

**Will there be co-op?**
Co-op is a first-class goal. The design uses a server-authoritative simulation with client prediction
and join-in-progress, written fresh rather than copied from the original netcode. See the
[design document](../superpowers/specs/2026-11-21-ss2rs-design.md). None of it is playable yet; the
first networked walk is milestone M4.

**Will it be cross-platform?**
Android is planned for a later phase, after PC 1.0 (milestone M9). Cross-play between PC and Android
is a planned test matrix (T12-020).

## Mods

**Can I use mods?**
Not yet. The mod format is documented in [docs/re/mods.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/mods.md):
a mod has a `.mod` descriptor and `.nfo` file, and overlay `.gro` archives that replace or add files.
The engine does not read mod folders today. Mod loading is planned in T13-001, and the data-only mod
Renovation is planned in T13-003.

**Can InSamnity! 2 work?**
Not yet, and it is not covered by the data-only plan (T13-003). That mod ships its own replacement
game module (a `.dll`), and the [modding policy](../MODDING_POLICY.md) notes that a Rust engine cannot
load it. How such mods are handled is a planned decision (the spike T13-002).

**What may a mod contain?**
Read the [modding policy](../MODDING_POLICY.md), especially sections 2 to 4. In short, a mod may
contain its author's own content and may refer to the player's files by path, but it may not contain
game files or copies of them.

## Data and privacy

**Does SS2RS collect data about me?**
The repository contains no telemetry code. The planned playtest program is described as having
**no telemetry by default** (T12-011), and the [PC 1.0 checklist](../PC_1_0_CHECKLIST.md) says the same.
Those are plans, not guarantees for a build that does not exist yet.

**Does SS2RS need an internet connection?**
The repository does not document a requirement. Online play is part of the co-op plans and is not
built yet.

## Bugs and support

**How do I report a bug?**
Open an issue on
[GitHub](https://github.com/Ihor-Ihorevych/SS2RS/issues) with the template that matches the problem:
crash, desync (for co-op peers), visual or gameplay parity, or a general bug. The steps and the
fields are in [Troubleshooting: what to include in a report](troubleshooting.md#what-to-include-in-a-report).
Security problems must not be reported in public; see
[SECURITY.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/SECURITY.md).

**Will someone answer my report?**
The triage targets are volunteer goals: a first response within 7 days, and an acknowledgement of
critical (S1) reports within 3 days. They are not guarantees. See [TRIAGE.md](../TRIAGE.md#6-response-goals).

## Contributing

**How can I help?**
Read [CONTRIBUTING.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CONTRIBUTING.md).
Work is taken from the roadmap by claiming a task first. Testing and bug reports help too; see the
[testing strategy](../TESTING.md) for what the automated tests cover.

**Can I write a mod?**
Not yet. Mods wait for the mod loader (T13-001), and the modder documentation is planned (T13-010).
The [developer tools list](../DEV_TOOLS.md) names what a contributor needs today.
