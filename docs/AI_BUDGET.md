# AI performance: thinking less often, and a budget

`ss2_game::ai_budget` (T09-017) is what keeps a horde of hundreds cheap to simulate **without any enemy knowing about it**. It is pure logic (time is the tick number, no randomness, no `HashMap`), written before there is any AI to use it, so it has only been tried against toy agents (see the end). Three pieces:

## How often an agent thinks: `RatePolicy`

`RatePolicy::interval(distance, visible)` is the number of ticks between thoughts for an agent that far from the nearest player. The default has **bands**: every tick within 15 units, every 2nd within 40, every 4th within 80, every 8th within 150, every 16th beyond. An agent **nobody can see** thinks half as often (`hidden_factor` 2), and nothing waits longer than `max_interval` (30) ticks. A NaN distance counts as far, a negative one as at the player; the result is always at least 1, even for a policy with zeros in it (`validate()` refuses those). `validate()` also refuses bands that do not go outward (distances increasing, intervals not decreasing) and a far interval shorter than the last band's.

## Who thinks this tick: `AiScheduler`

`plan(tick, &[(key, wanted_interval)])` returns the agents that think on this tick, as `Think { key, dt_ticks }`:

- **Staggered.** A new agent's first thought falls within its interval, at a phase taken from a hash of its key (`mix`, which is splitmix64: tested against that function's published first output, and for an even spread of keys that step by 1, 2, 8, 64 or 2^20, because entity ids often carry stride or generation bits). 800 agents wanting every 8th tick therefore think about 100 a tick, not 800 every eighth tick.
- **Capped.** At most `max_per_tick` think. When the cap bites the **most overdue go first** (then the lowest key), and whoever was cut is first in line next tick. A test with 1,000 agents wanting every tick and a cap of 100 shows every agent thinking every 10 ticks: the longest wait is 10 (+ 1), nobody is neglected. A new agent that was cut keeps its place (it is not given a fresh phase, which would let a busy tick starve it).
- **`dt_ticks` is the real time since the last thought**, so an agent advances its timers by what actually passed, not by what it wanted (a first thought reports its interval). It is exactly right when the cap or a changed rate shifted the schedule.
- **A changed rate** takes effect from the last thought (an agent that comes close thinks again as soon as the new, shorter interval has passed), and agents no longer listed are forgotten.

## Path requests: `PathBudget`

A queue of `PathRequest { key, priority, cost }` with `max_per_tick` (8) and `max_cost_per_tick` (4,000), both applying to each `take(tick)`:

- **Ageing.** Every 15 ticks of waiting count as one more level of priority, so a patient low-priority request overtakes a fresh urgent one (tested at 30 ticks: not yet; at 40: tie, and the older wins; at 95: well ahead).
- **One request per agent.** Asking again **replaces** the request (new destination, priority and cost) but **keeps its place in line**, so an agent that asks every tick cannot keep itself at the back.
- **Cost.** A request that does not fit in what is left of the tick's cost is **skipped** and a cheaper one behind it may still be taken, but **the first request of a tick is always answered, whatever it costs**, so an expensive one cannot block the queue forever.
- Bounded (1,024 waiting), `cancel(key)` for an agent that died.

## The horde test

One test runs **800 toy agents** for 600 ticks: distances from 5 to 400 units that change every 50 ticks as the "player" moves, a third of them hidden, a cap of 150 thoughts per tick, and a path request from a quarter of the thoughts, against the default path budget. It checks that the cap holds, that the mean is far below 800 and above 40, that **every agent thought**, that no agent waited much longer than the slowest rate allows, and that no path request waited unreasonably long.

## How it was tested

Twenty-one unit tests and two doctests. **Twenty-four deliberate breakages**: eighteen failed a test at once; **six survived my first tests**. Four were real and are now covered: two weakened versions of the hash (one without its first mixing step, one without its last fold), which my even-spread test on sequential keys could not tell from splitmix64 (it needed keys that step by a power of two, and the published reference value), a `clamp` replaced by a `min` (visible only for a policy with zeros in it), and `dt` always being the wanted interval (which only differs when the cap or a rate change delays a thought). Two were equivalent and I **removed the code**: the explicit NaN branch (NaN fails every comparison, so no band matches) and the key tie-break in the path queue (it iterates in key order and the sort is stable). The first run of the tests also found a real bug in my first scheduler: a `saturating_sub` that put a new agent's first thought one interval late near tick 0, which I replaced with signed arithmetic.

## Not covered (follow-up T09-017b)

- **No AI uses it.** There are no enemies or AI states yet (T09-010), so nothing asks for a distance, calls `plan`, or submits a path request; and there is no path finder to answer the requests (T07-013). **The horde test is of the scheduler with toy agents, not a horde of enemies**: what it costs to think, and whether 150 thoughts a tick is the right cap, is not measured.
- **Visibility and distance are inputs.** Nothing computes "nearest player" or "can anyone see this agent" (a cheap test against the camera frustum, or the real line of sight, is the AI's choice).
- **Perception and animation also need rates**: an agent that thinks every 16th tick but animates every frame, and a far agent that still has to be able to *hear* a shot, are for the AI framework.
- **The numbers are mine**: the bands, the factors, the caps and the path budget are placeholders for a measurement on real hordes, on a phone too (Phase 2).
