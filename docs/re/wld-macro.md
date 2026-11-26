# `.wld` and `.mac` scripting: the macro language as stored (T01-012g)

- **Roadmap task:** T01-012g (part g of the world spec T01-012); the `.mac` files of `misc-types.md` are the same format
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the storage (program, functions, statement and expression tree, code segment ids, the stored source text, macro entities, variable bindings), for the class list and counts, for how event expressions, handlers and run statements are stored (all checked on every world and `.mac` file with `tools/re/wld_macro_verify.py`; members come from the files' own type tables, see `wld-macro-schema.md`), and, as of T01-065 (`tools/re/macro_leftovers_verify.py`, same scope), for the bare `else {…}` clause of `CMacroStatementIf.4` (exact per-program match, 1,053 of 1,053) and for marker kind 1 being comments; **medium** for the other token marker kinds (read from the 8 `.mac` files) and for the source text grammar (read from samples, not from a parser); **low** for run-time semantics (when handlers fire, how events are matched, what each action does, and how the interpreter itself — `Engine.dll`'s `CMacroInterpreterImp`/`CMacroCodeSegment::DispatchVisitor` — actually executes each statement class, whose own per-class override was not found as a separate symbol in either DLL): those are T01-030 and need the vocabulary tables and measurement. Scope: all 59 worlds (1,045 embedded programs) and the 8 `.mac` files.

Prerequisites: `ctsemeta-objects.md`, `wld-root.md` (domains, entities), `wld-entities.md` (macro entities and records), `help-digest.md` (the editor documentation of the language).

## Summary

- A **macro program** is a visual script. It is stored **twice**: as a **syntax tree** (the part a runtime executes) and as the **source text** the editor shows, with **76,210 token markers** that describe the text for the editor's colouring. 1,053 programs: 1,045 embedded in 49 worlds and 8 in `.mac` files.
- A program is a list of **functions**; a function is a **statement block** plus its **`On` handlers** (conditions with their own blocks that run as new threads). 1,668 functions, 74,603 tree objects.
- A **macro entity** (`CMacroEntity`, 1,094 in 50 worlds) runs one function of one program (normally `Main`). The program names world objects by **variable name**; the entity binds each name to an **entity handle**, an array of handles, a resource or nothing. All 14,173 handles resolve to entities of the world.
- The vocabulary (131 + 90 actions, 55 calls, 457 event names, 383 sent events, 307 run targets) is generated in `wld-macro-vocab.md`; what each name does belongs to the level logic spec (T01-030).

## Evidence

`tools/re/wld_macro_verify.py` (read-only, `SS2_DATA_DIR`; about 15 minutes) over the retail install with all patches:

- **Every program decodes**; code segment ids are unique inside every program (1,053 of 1,053); `CMacroProgram.4` equals the highest id plus one in 1,045 of 1,053; all 24,648 statements of blocks name their block as parent (`CMacroStatement.2`).
- **Source text and tree agree** on the number of `Action`, `ActionAsync`, `Run`, `Break`, `ExitThread`, `ExitAllThreads`, `Print`, assignments and `Macro` headers in all 1,053 programs, and on `RunAsync`, `for` and `var` in 1,052, `foreach` in 1,051, `On` in 1,048, `Send event` in 1,047, `If` in 1,037 and `Wait` in 1,028. 49 programs differ in at least one count (for example `Wait` 9 in the text and 10 in the tree): the text is a view that can lag behind the tree.
- **Markers:** 76,207 of 76,210 are single-line and lie inside the text.
- **Macro entities:** 1,094 of 1,094 have a record with the same function name (`CMacroProperties.5` = `CMacroEntity.11`) and the same number of variables; 1,071 name a function that exists in their program (19 of the other 23 name `Main` in a program without one).
- **Handles:** 9,834 handle values and 4,339 array elements, all resolving to entities of the world.
- **Leftovers (T01-065).** `tools/re/macro_leftovers_verify.py` (read-only, same scope; ~15 minutes): resolves the 12 run-target and 23 macro-entity-function mismatches by name against every other program's functions in the install, samples marker kinds 1 and 15 with their exact source substrings, checks `CMacroSource.2`/`CMacroProgram.5`/`CMacroEntity.4` against other known fields, and cross-checks `CMacroStatementIf.4` against the stored text's bare `else {` count, per program.

## The program

```
CMacroProgram  (derives CResource; the root of a .mac file, or an object of a world listed in rf_crsShared)
  .2  list of functions: CMacroFunction items, plus CMacroFunctionNewLine (1,844) and CMacroFunctionComment (15) that keep the editor's layout
  .4  next free code segment id (highest id used + 1)
  .5  BOOL 0 or 1
  .6  CMacroSource: the stored text
CMacroFunction
  .1  -> CMacroStatementBlock  the body
  .2  IDENT  name (Main in 735 of 1,668)
  .3  IDENT  the entity class the function applies to; empty = any (1,262 of 1,668); otherwise a class name such as CLeggedCharacterEntity (209)
CMacroCodeSegment (base of every statement, expression and handler)
  .1  INDEX  the code segment id: unique inside the program, 64 or more in 1,023 programs; -1 on objects that are not code (variants, variable expressions)
CMacroStatement (derives CMacroCodeSegment)
  .2  -> the block that holds the statement (null for a body block)
```

- **Source text.** `CMacroSource.1` is the program text (editor language, below). `.2` is a number close to the line count (equal in 739 of 1,053 programs: do not rely on it), `.3` = 1, `.4` = 0, `.6` = 0. `msrc_ahvHintedVars` lists **hinted variables** (`CMSHintedVar`: name, optional type such as `INDEX`, `BOOL` or an entity class): 6,394 entries, 5,687 of them named like a variable bound by the macro entity.
- **Markers** (`CMacroSourceMarker`, `.5` of the source): `.1` line, `.2` end line (always equal), `.3` first column, `.4` last column (inclusive), `.5` token kind, `.6` flag. They give the colour class of each token. Kinds (counts over all programs; meaning read from the `.mac` files): 2 with flag 1 = keywords, punctuation and indentation (23,545), 8 = object names (15,468), 10 = action and call names (9,743), 3 = expression words such as `event`, `every`, `==`, `or`, `False`, times (9,278), 12 = event names (6,228), 9 = variable names (3,562), 11 = function names (2,345), 14 = `This`, `NULL`, operators (2,168), 13 = the class word of a function header (1,660), 7 = literals and type names (1,182), 1 (1,027) and 15 (4): not determined. A runtime does not need markers.

## The syntax tree

**Statements** (all derive `CMacroStatement`; counts are objects reachable from a function):

| Class | Source form | Members | Count |
|---|---|---|---|
| `CMacroStatementBlock` | `{ … }` | `.1` list of statements, `.2` list of `COnHandler` (only on a function body), `.3` BOOL 1 = function body | 8,059 |
| `CMacroStatementNewLine` | blank line | none | 6,856 |
| `CObjectActionStatement` | `Action Obj.Name(args)` | `.1` → object expression, `.2` IDENT name, `.3` empty list, `.4` list of variants (arguments) | 5,341 |
| `CObjectActionAsyncStatement` (derived) | `ActionAsync Obj.Name(args)` | same | 3,111 |
| `CMacroStatementWait` | `Wait (cond)` / `wait 3 sec` | `.1` → boolean expression | 3,734 |
| `CVarAssignmentStatement` | `Name=expr` | `.1` IDENT variable, `.2` → object expression | 1,522 |
| `CMacroStatementComment` | `//text` | `.1` string | 1,034 |
| `CMacroStatementNotify` | `Send event Obj.Name` | `.1` → object expression, `.2` IDENT | 884 |
| `CMacroStatementIf` | `If (c) {…} else If (c) {…} else {…}` | `.1`/`.2` always null, `.3` list of 1 to 7 `CMacroStatementIfClause` (condition `.1`, block `.2`), `.4` the bare `else {…}` clause's block, set in 33 of 609 (**confirmed**, T01-065: exactly matches the bare-`else` count of the stored text in every one of the 1,053 programs) | 609 (clauses 667) |
| `CVarTypeHintStatement` | `var Type Name`, `global var Type Name` | `.1` IDENT name, `.2` IDENT type, `.3` BOOL: 1 for `global var` (99 of 99 in the sample checked against the text) | 528 |
| `CMacroStatementRunMacro`, `…RunAsyncMacro` | `Run Obj.Function`, `RunAsync Obj.Function` | `.2` → object expression, `.3` IDENT function name | 220 + 466 |
| `CMacroStatementForeach` | `foreach X in Group {…}` | `.1` IDENT variable, `.2` IDENT collection, `.3` block | 162 |
| `CMacroStatementBreak` | `Break` | none | 114 |
| `CMacroStatementFor` | `for (i=0; i<1; i=i) {…}` | `.1` init, `.2` condition, `.3` step, `.4` block | 35 |
| `CMacroStatementPrint`, `…Error`, `…ExitThread`, `…ExitAllThreads` | `Print`, error placeholder, `ExitThread`, `ExitAllThreads` | string members or none | few |

**Handlers.** `COnHandler`: `.1` → boolean expression (the condition), `.2` → block (the body). Handlers are listed in `CMacroStatementBlock.2` of the **function body**: 1,220 body blocks hold 3,666 handlers, and no other block holds any. Source form: `} On (cond) do { … }` after the function's closing brace; in some older programs `on every 20 sec do {`.

**Boolean expressions** (derive `CBoolMacroExpression`):

| Class | Source form | Members | Count |
|---|---|---|---|
| `CEventBoolExpression` | `event(Obj.Event)`, `event any(A.E, B.F)`, `event all(…)` | `.4` `EEventExpressionType` **0 = single (4,656), 1 = any (189), 2 = all (300)**, `.5` list of `CSingleEvent` (`.1` → object expression, `.2` IDENT event name; 1 to 5 per expression; 4,974 have one) | 5,145 |
| `CTimeMacroExpression` | `3 sec`, `(Time sec)` | `.2` → object expression (a float constant or a variable); **`.1` is stale memory (values such as 1e-38 or 3e38): ignore it** | 1,915 |
| `CEveryMacroExpression` | `every (cond)` | `.1` → boolean expression | 826 |
| `CCounterMacroExpression` | `2 times (cond)` | `.1` INDEX count (1 to 90), `.2` → boolean expression | 337 |
| `CFalseMacroExpression`, `CTrueMacroExpression` | `False`, `True` | none | 413, 36 |
| `CNotEqualBoolExpression`, `CIsEqualBoolExpression`, `CIsLessThanBoolExpression` and the other comparisons | `!=`, `==`, `<`, `<=`, `>`, `>=` | two object expressions | 292, 191, 57, rest few |
| `COrBoolMacroExpression`, `CAndBoolMacroExpression`, `CAndThenBoolMacroExpression`, `CNotBoolMacroExpression` | `or`, `and`, `and then`, `not` | two or one boolean expressions | 72, 21, 13, 10 |
| `CObjectAdapterBoolExpression` | an object expression used as a condition | `.1` → object expression | 117 |

**Object expressions** (derive `CObjectExpression`): `CVarObjectExpression` (a name: `.1` IDENT; 15,478), `CThisObjectExpression` (`This`, 1,825), `CNullObjectExpression` (`NULL`, 300), `CFunctionObjectExpression` (`Obj.Function(args)`: `.1` object, `.2` IDENT name, `.3` list of variants; 1,324), constants (`CFloatConstObjectExpression` 1,914, `CIntegerConstObjectExpression` 605, `CStringConstObjectExpression` 26; each holds a `CVariant`), `CAssignmentObjectExpression` (`i=expr` inside `for`: 71), `CAddObjectExpression`, `CSubstractObjectExpression`, `CMultiplyObjectExpression`, `CNegObjectExpression` (arithmetic, few). **Variants** (arguments of actions and calls): `CMacroVariantString` 1,923, `…Var` 1,224 (a name as string), `…Integer` 311, `…Float` 240, `…BOOL` 156, `…This` 39, `…Null` 37.

## Macro entities and variables

```
CMacroEntity (1,094; world records CMacroProperties)
  .3   -> CMacroProgram: embedded (1,049 entities, 1,045 programs; 3 programs are shared by 2 or 3 entities) or an external .mac file (45 entities, 8 files)
  .11  IDENT  the function to run: Main in 795 entities, otherwise a function of the program (ChapterControl, Chapter001, …)
  .2   list of CEntityVariable (name IDENT, CVariant): 15,692 bindings, 0 to 173 per entity
  .1   null (the interpreter handle, empty in the stored state); .4 BOOL; .5 0; .10 -> its editor renderable
CMacroProperties: .1 program, .2 list of CEntityPropertyVariable (name IDENT, .2 null, CVariant), .3 BOOL, .5 function name
```

- **Binding.** The program uses names (`PlatformMover`, `Chapter001`, `Spawnee`, …); the entity's variable list says what they are. Values (`CVariantImp…`): **handle** `CVariantImpHANDLE` (9,834; the `CMetaHandle` is an object id of an **entity of the world**, all resolve), **handle array** (1,534 variables, 4,339 handles in all, for groups used by `foreach` and `all`/`any` events), **resource pointer** (346: an external `.ogg` 311, `.ctl` 17, `.wav` 17, `.mdl` 1) and **void** (3,978: a declared name without a value). The record repeats the entity's variables with the same names in the same order; a void variable has an **empty name in the entity** even when the record names it.
- **Hinted variables** of the source name entity classes (`CLeggedCharacterEntity`, `CPlayerPuppetEntity`, `CPropEntity`, …) or `INDEX` / `BOOL` for the editor's type checking.
- **Function classes.** `Macro Name Class` restricts a function to one entity class (`?` = any); a `Run`/`RunAsync` statement runs a function **on an object** (`RunAsync Rocket.Rocket`). 674 of 686 run targets name a function of the same program; the rest run on other objects' programs (not analysed).
- **Domain.** `CMacroDomain` (one per world) holds `md_ateEvents` (empty in all 59 worlds) and `md_ulUniqueEventID` (0): the run-time thread and event state is not stored.

## The source text (editor language)

Observed grammar (not a parser; keywords are written with different capitalisation in old programs):

```
Macro <Name> <Class|?>
{
  [global] var <Type> <Name>        // declaration
  <Name>=<expr>                     // assignment
  Action <Obj>.<Name>(<args>)       ActionAsync …       // latent action: Action waits, ActionAsync does not
  Run <Obj>.<Function>              RunAsync …
  Send event <Obj>.<Event>
  Wait (<cond>)                     wait 3 sec
  If (<cond>) { … } else If (<cond>) { … } else { … }
  for (<i>=<a>; <cond>; <i>=<expr>) { … }     foreach <X> in <Group> { … }
  Break   ExitThread   ExitAllThreads   //comment
} On (<cond>) do { … } On (<cond>) do { … }
cond: event(<Obj>.<Event>) | event any(<list>) | event all(<list>) | every (<cond>) | (<n> times (<cond>))
      | <n> sec | (<cond>) and then (<cond>) | and | or | not | <expr> == != < <= > >= <expr> | False | True
```

The help documentation (`help-digest.md`) describes the meaning: a handler runs as a **new thread** whenever its condition fires; a macro entity starts `Main` with itself as `This`; latent actions take time and can be waited on. Only the structure above is verified by the data.

## Reader invariants

1. Execute the tree, never the text; the text may lag behind it (49 programs).
2. Read a program as: functions (skip `CMacroFunctionNewLine`/`Comment` items), each with a body block whose `.2` list holds its handlers.
3. Resolve names through the macro entity's variable list; a void variable is unbound; a handle is an entity object id of the same world.
4. Event expressions: `.4` selects single, any or all; ignore `CTimeMacroExpression.1`.
5. Code segment ids identify code inside one program only; do not compare across programs.
6. Unknown statement, expression or variant classes must fail loudly: the classes in `wld-macro-schema.md` are the complete list of the retail data.

## Test vector

`Content/SeriousSam2/Levels/Multiplayer/DM_Catacombs/PlatformControl.mac` (4,817 bytes): one `CMacroProgram` with one function `Main` (class `?`), body block (id 64) with a newline, `Wait (False)` (73, 74) and a newline, and one handler (65): `On (every (event(PlatformDetector.Activated)))` (66, 67) whose block (68) holds `Action PlatformMover.PlayAnimWait("Default")` (69) and `Action PlatformDetector.Recharge()` (70) and a newline (71); `CMacroProgram.4` = 76, highest id 75. Stored text (180 characters): `Macro Main ?\n{\n  \n  Wait (False)\n  \n} On (every (event(PlatformDetector.Activated))) do {\n  Action PlatformMover.PlayAnimWait("Default")\n  Action PlatformDetector.Recharge()\n  \n}\n ` with 17 markers (the first: line 0, columns 6 to 9 = `Main`, kind 11; the second: line 0, column 11 = `?`, kind 13) and hinted variables `PlatformDetector` and `PlatformMover` (no type). `IDNT`: `['', 'Main', 'Activated', 'PlayAnimWait', 'Recharge', 'PlatformDetector', 'PlatformMover']`.

## Open questions

- **Semantics:** when a handler's condition is evaluated, how `every`, `times`, `and then` and `any`/`all` events are matched, what each action, call and event name does (the tables in `wld-macro-vocab.md` list them with their use counts): T01-030, with the entity specs.
- **Answered by T01-065** (`tools/re/macro_leftovers_verify.py`, data only — the macro interpreter itself, `CMacroInterpreterImp` and `CMacroCodeSegment::DispatchVisitor`, lives in `Engine.dll`, but the per-statement-class `Execute`/`Visit` override that would show *how* `If`, `Run` and the rest actually run (as opposed to how they are merely stored) was not found as a separate symbol in either DLL — likely folded into something else or never kept as its own export — so none of this is confirmed against the interpreter, only against the stored data):
  - **The 12 run targets naming no function of their own program** (674 of 686 do; this answers the other 12): `LookTarget` (3 uses, one program, `1_1_Jungle`), `Behavior` (3 uses, `2_1_Swamp`), `BridgeControl` (1, `5_1_Village`), `Particle_Control` (1, `5_3_Town_City`), `ZombieSoldiers` (1, `6_2_Military_Base`), `Helicopter_01_Behavior` and `Helicopter_02_Behavior` (1 each, `7_3_Arenas_DesertStadium`), `StartTanks` (1, `7_6_City_Part_III`). **The "runs on another object's program" idea is mostly refuted**: `LookTarget`, `ZombieSoldiers`, `Helicopter_01_Behavior` and `StartTanks` name no function anywhere in the whole install — plain broken references. `Behavior` and `Particle_Control` are common generic names that happen to exist in several unrelated worlds' programs (coincidence, not a real target: nothing ties the `Run` statement's object to those other worlds). Only `BridgeControl` (a function of a *different* program in the *same* world, `5_1_Village` program 26) and `Helicopter_02_Behavior` (a function of `6_4_Alliance_Headquarters`'s own program) are even plausible cross-program runs, and even those were not confirmed against the interpreter.
  - **The 23 macro entities whose function (`.11`) is not a function of their own program**: 19 of them simply store the editor's default name, `Main`, while their own program defines no `Main` at all (it has only custom-named functions) — these entities are inert in the shipped game, not redirected anywhere; "elsewhere" always finds *some* program's `Main` (735 of 1,668 functions are named `Main`), which is not evidence of anything. The other 4 are genuine authoring mistakes, not redirections: `CamearMacro` for the stored `CameraMacro` (a typo, `1_6_Inside_Temple`), `Fight` for `FightMacro` (likely a truncation, same world), `SecurityChapter004` for the program's own `SecurityChapter003` (an off-by-one rename that one entity missed, `3_3_Chaolin_Chambers`), `SpawnMachineSpawn` matching none of the program's `SpawnSoldiers01`-`04` (`6_2_Military_Base`). None of the 23 resolves anywhere that looks like a real, designed redirection.
  - **Marker kind 1** is **comments**: every sampled instance is a `//...` line (`1_1_Jungle`'s 15 largest). **Kind 15** stays open: only 4 exist in the whole install and the one this script's single-line bounds check accepted resolved to an **empty span** (`c0 == c1`, zero characters) — rare enough, and degenerate enough, that it is likely a cursor/placeholder marker rather than a highlighted token, but that is not confirmed.
  - **`CMacroSource.2`**: confirms the existing "739 of 1,053 equal the line-break count" exactly, and adds that 28 more equal `line-break count + 1` (a plain "total line count including the last one" convention) while the remaining 286 (27%) match neither — consistent with a **cached value the editor does not always keep in sync with later edits**, not a computed field a reader can rely on (as the existing text already warns).
  - **`CMacroProgram.5`**: 0 in 1,049 of 1,053 programs; the 4 programs with it set to 1 average 2.5 functions each against 1.58 for the rest — too few (4) to call that a rule rather than noise. Still open.
  - **`CMacroEntity.4`**: every one of the 1,094 macro entities has a world record regardless of its value (801 are 1, 293 are 0); record presence does not distinguish the two, so this one check does not reveal the flag's purpose. Still open.
  - **The "else" clause of `CMacroStatementIf.4`** (the existing doc's "set in 33 of 609, probably `else {…}`, not verified"): **confirmed, exactly, per program** — for every one of the 1,053 programs, the number of bare `else {` occurrences in its stored text (an `else` not immediately followed by `If`) equals the number of its own `CMacroStatementIf` objects with `.4` set to a non-null reference: 33 and 33, 0 programs differ. `.4` **is** the bare `else {…}` clause.
  - `CMacroSource.2`/`CMacroProgram.5`/`CMacroEntity.4`'s remaining opens, `CTimeMacroExpression.1`'s writer, and the interpreter's own `DispatchVisitor` per statement class are not re-ticketed: each is a one-line open note on this page, not a separate task yet.
- `CSamMacroEvent` (an animation event class whose `sme_idEvent` is a macro event name such as `Lift_Down`): how animations raise macro events (`mdl.md`, T01-057).
