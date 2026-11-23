# Visibility: sight and sound

`ss2_physics::visibility` (T07-024) answers two questions for AI and gameplay: **can A see B**, and **how far does a sound carry**. It is queries and arithmetic over data given to it; it does not know about enemies, doors or levels.

## Sight

- `line_of_sight(world, from, to, target, params)` casts the segment against any `SpatialQuery` (the static world and moving colliders; `docs/PHYSICS_QUERIES.md`) and returns `Clear`, `Blocked { distance, by }`
  (`by` is `None` for the static world), `OutOfRange` or `Invalid` (a point that is not finite).
  - **The target does not block itself.** A ray to the centre of a body first meets its surface; pass the target's `ColliderId` and hitting it counts as clear. Without it, the surface blocks.
  - **Which layers are opaque is the caller's mask** (`SightParams::filter`): glass, triggers and water volumes are just not in it. The looker's own body is excluded with `filter.exclude`.
  - A **margin** (default 0.01) stops the line short of the target, so the floor or wall a target stands against does not block it. A line no longer than the margin is clear, even from inside a collider.
  - Sight is symmetric for the static world (tested); for moving colliders it is whatever the filter and target say.
- `exposure(world, eye, points, target, params)` casts to several points of a target (head, chest, feet) and counts the clear ones (`Exposure { visible, total }`, `any()`, `full()`): partial cover.
- `in_cone(from, forward, min_cos, to)` is the field-of-view test. **The field of view is a cosine, not an angle**: the physics crate keeps trigonometry out so that its results are the same bits on every platform
  (`docs/GEOMETRY.md`); convert degrees to a cosine once, in configuration. The same point is inside; a non-finite input is never.
- `Perception { min_cos, sight }::look(...)` combines range, cone and lines and returns `Seen::Full`, `Seen::Partial(exposure)` or `Seen::No(reason)` with the reason (`OutOfRange`, `OutsideCone`, `Blocked(..)`, `Invalid`).
  The first point is taken as the target's centre for the range and the cone.

## Sound

A `SoundGraph` is **sectors joined by portals** (`Portal { a, b, length, closed_penalty, open }`): the distance a sound travels to cross it, and the **extra distance it costs when closed** (a door muffles). A sound's **loudness is
the distance it carries in open air**. `propagate(source, loudness)` returns, for every sector the sound reaches, the loudness left there (the source sector has all of it; a sector `d` away through portals has `loudness - d`, if not
negative); `heard_at(source, loudness, listener)` is one entry. Opening or closing a portal is `set_open`. It is Dijkstra over the distance, so **the shortest way wins**, cycles are harmless, the answer is a `BTreeMap` (sorted
by sector, the same on every machine) and it does not depend on the order portals were added in (a test builds a 2,000-sector chain both ways).
Portals need finite, non-negative lengths and penalties; a loudness that is negative or not finite reaches nowhere.

The loudness left in a sector is the loudness **at its centre**: the distance inside the source sector (from the noise to the portal) and inside the listener's sector is the caller's to subtract.

## How it was tested

Fifteen tests and a doctest, mutation-checked. Sight: a wall blocks with its distance and one in front of the target does not; symmetry; moving colliders block, the target does not block itself, the looker's own body is excluded;
glass in the mask or out of it; the range and margin boundaries; the same point, a very short line, a line inside a collider; non-finite input; partial cover; the cone at its edge, behind, and with bad input; a looker that
reports each reason. Sound: attenuation and the exact boundary; closed doors and opening them; the shortest way among several (including one found late); a sector alone; bad loudness; bad portals; a long chain built
in two orders. **Eighteen deliberate breakages**; fifteen failed a test at once, **three survived** (a zero-length line, the exact range of a looker, and keeping the first way found instead of the shortest), and I added tests for them; the
eighteenth (a self-portal listed twice) is equivalent. The first cone test was wrong (a direction 0.001 off the line has a cosine that rounds to exactly 1 in 32-bit floats) and was fixed.

## Not covered (follow-up T07-024b)

- **Where sectors and portals come from.** Level data (T07-015) does not exist, so the graph is built by hand in tests. Doors that open and close must call `set_open`; nothing does.
- **AI behaviour**: what a looker does with `Seen`, memory of a last known position, reaction time, noticing over several ticks. That is gameplay (T09).
- **Sound categories and occlusion by geometry**: every sound is one number; a thin wall between two sectors with no portal blocks it completely and a thick one is no different from a thin one, and the distance inside a sector is not modelled.
- **Cost**: `exposure` is one ray per point per looker; the speed of a level full of enemies looking every tick is not measured (a ray costs about 1 to 1.6 microseconds, `docs/rfcs/0008-physics-engine.md`), and no look-at-most-every-N-ticks scheduling exists.
- **Sight through moving platforms and rotated or scaled shapes** is whatever `QueryWorld` does; capsule and box colliders are only as exact as its ray tests.
