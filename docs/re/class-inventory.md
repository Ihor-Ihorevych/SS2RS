# Class inventory of `Sam2Game.dll`: all 932 exported classes against the twelve parts (T01-113)

- **Roadmap task:** T01-113 (the last step of the class inventory T01-026: one table of all exported classes with the part that covers each, and the check that every exported class and every member table appears exactly once). Tables: [`class-inventory-1.md`](class-inventory-1.md) (`CAIBrainEntity` to `CMCPatternMoving`) and [`class-inventory-2.md`](class-inventory-2.md) (`CMCStayInArea` to `SMoveCharacter`). Parts: T01-102 to T01-112 and [`helper-classes.md`](helper-classes.md) (T01-113).
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the counts in sections 1 to 5 (the export table, the type tables and the member tables of the DLL against the text of the pages; `tools/re/class_inventory_check.py` re-runs the checks that need only the repository); **medium** for the family rules of section 3 (a page states the rule and names the partner class, but does not list every member of the family); **high** that the gaps in section 8 are the gaps this check can see, **low** that nothing else is missing (a class can be named on a page and still be described thinly).

Prerequisites: the twelve parts named in section 1, `classes.md` and `classes-catalog.md` (the data's type tables), `replication.md` (the member tables, section 3.4 there).

## Summary

`Sam2Game.dll` exports **932 classes** with **10,502 exported member functions** (11,610 exports in all). The T01-026 split put every class into one of twelve families, and each family was specified by one part. This page is the **reconciliation**: the table of all 932 classes (two pages) says, for each class, its part, the page that covers it, how the page names it, whether and under which name it is in the data, its exported function count and its member table.

| Check | Result |
|---|---|
| 1. Every exported class appears exactly once in the table | **932 rows, 932 distinct names**; assigned to one part each (section 4) |
| 2. Every member table of the DLL is named on a page of some part | **227 of 227**: 194 by the table's own name, 31 by the class they belong to, 2 by a family rule (section 6); **36 tables have member names that no page spells out** (listed) |
| 3. Per part, the class count agrees with the part's own page | **12 of 12** (section 1) |
| 4. Classes mentioned on a part's pages but not exported | listed in section 7 (saved-only names, stems, names the pages say do not exist) |
| 5. 505 binary-only + 427 in the data = 932 | **505 + 427 = 932**: 178 under the same name and 249 as the `S` twin (the survey's rule); **11 more are saved under another name**, so **438 are in the data and 494 exist only in the binary** (section 5) |
| Classes no page names literally | **146**, all covered by a stated family rule whose partner class is named (section 3); **0 classes are covered by no page** |
| Classes defined in two parts | **0** (each has one part); **13** have a table row or heading on another part's page too, and **102** are named on another part's pages at all (cross references, section 4) |

**Headline findings.**

1. **The inventory is complete at the level of names**: every one of the 932 classes is named, literally or by a stated family rule, on a page of its own part, and every one of the 227 member tables is named on some page. **High**.
2. **146 classes are covered only by a rule** (`C<Name>Properties` empty shells and similar families) and not by a line of their own; the rule and its partner class are on the page for all of them (section 3). **Medium**.
3. **The survey's 427 / 505 split undercounts the data by 11**: eleven classes are saved under a name that is neither the exported name nor its `S` twin (a misspelling, a renaming, an editor name); earlier parts found nine of them and this part two (section 5). **High** for the nine named by earlier parts and the carry/throw record, **medium** for the environment record.
4. **The 227 member tables are 199 tables of exported classes and 28 tables of menu gadget classes that are not exported** (`menu-gadget-members.md`); 12 of the 199 are registered under a name other than the class's (section 6). **High**.
5. **36 tables name more members in the DLL than the pages spell out** (section 6): the gap is member-level naming, not classes; it is parked as `T01-216`. **Medium** (the measure matches member names as words on the pages and can over-count when a page names a member by its data number).

## Evidence

- **Export table.** The 11,610 exports of `Sam2Game.dll` were read once (private notes outside the repository); the 932 classes with their exported function lists, the 1,043 classes of the data's type tables (`classes-catalog.md`) and the DLL's 227 member tables (name, parent, members) were tabulated. Class names are quoted as exported. The twelve-way family split is the T01-026 survey's (a name-pattern split), and the check below confirms it against the pages: no class sits in a family whose pages do not name it.
- **Pages.** The 12 parts have 43 pages (section 1). A class counts as **named** on a page when its exported name or its `S` twin occurs there as a word; as **rule-covered** when it does not and the page states the family rule and names the partner class (the entity, or the stem for the 23 inventory items); the covering page of a class is the page of its own part where it appears in a table row or heading, else the page with most mentions, else (rule) the page with most mentions of its stem.
- **Generation and re-check.** The two tables and the numbers on this page were generated by a private script from the export tables and the pages (it reads the binary's export table, so it is not in the repository). `tools/re/class_inventory_check.py` (standard library, reads only `docs/re`) re-checks everything that needs only the committed pages: 932 rows, one per class; every row's covering page exists and names the class or its family; the per-part counts and the function total; and the 59 classes of T01-113. It does not re-derive the export table.
- **Not checked:** that a page's *description* of a class is correct or deep (every part page carries its own confidence words); the classes of the other modules (`Engine.dll`, `Core.dll`, the GUI modules: 1,043 - 427 - 11 = 605 data classes have no export in this DLL, engine classes and `S` value classes among them, `classes.md`); the DLL's exports that are not member functions of an exported class (the per-class data symbols and any free functions); anything that needed running the game.

## Layout / rules

### 1. The twelve parts

The family column counts come from the T01-026 split; "stated" is the class count the part's own pages state (found as `N classes` in the overview, 12 of 12 agree). "Named" = literally (exported name or `S` twin); "rule" = by a family rule (section 3). "Other name" = classes saved under a name that is neither the exported name nor the `S` twin. "Tables" = classes with a member table in the DLL.

| Part | Topic | Overview page | Classes | Stated | Named | Rule | In data (name / twin) | Other name | Functions | Tables |
|---|---|---|---|---|---|---|---|---|---|---|
| T01-102 | players, game info, inventory | [`players-session-classes`](players-session-classes.md) (1 page) | 52 | 52 | 29 | 23 | 10 | 0 | 740 | 4 |
| T01-103 | puppets | [`puppet-classes`](puppet-classes.md) (5 pages) | 101 | 101 | 84 | 17 | 49 | 1 | 1,464 | 22 |
| T01-104 | brains | [`brain-classes`](brain-classes.md) (1 page) | 61 | 61 | 44 | 17 | 0 | 0 | 572 | 1 |
| T01-105 | behaviours | [`behavior-classes`](behavior-classes.md) (4 pages) | 95 | 95 | 95 | 0 | 45 | 1 | 955 | 38 |
| T01-106 | weapons | [`weapon-classes`](weapon-classes.md) (1 page) | 83 | 83 | 46 | 37 | 44 | 0 | 780 | 27 |
| T01-107 | projectiles | [`projectile-classes`](projectile-classes.md) (1 page) | 32 | 32 | 32 | 0 | 14 | 1 | 332 | 17 |
| T01-108 | items | [`item-classes`](item-classes.md) (3 pages) | 113 | 113 | 66 | 47 | 97 | 0 | 919 | 8 |
| T01-109 | mechanisms, spawners, world info | [`mechanism-classes`](mechanism-classes.md) (6 pages) | 82 | 82 | 82 | 0 | 70 | 2 | 1,206 | 29 |
| T01-110 | world content | [`worldcontent-classes`](worldcontent-classes.md) (5 pages) | 55 | 55 | 50 | 5 | 23 | 2 | 854 | 20 |
| T01-111 | HUD | [`hud-classes`](hud-classes.md) (4 pages) | 95 | 95 | 95 | 0 | 45 | 2 | 797 | 18 |
| T01-112 | menus | [`menu-classes`](menu-classes.md) (8 pages) | 104 | 104 | 104 | 0 | 0 | 0 | 1,412 | 1 |
| T01-113 | helpers, events, global parameters | [`helper-classes`](helper-classes.md) (4 pages) | 59 | 59 | 59 | 0 | 30 | 2 | 471 | 14 |
| **all** | | 43 pages | **932** | **932** | **786** | **146** | **427** | **11** | **10,502** | **199** |

Reading: the counts add up to 932 classes and 10,502 functions, the same as the survey; 427 classes are in the data by the name rule.

### 2. The columns of the two tables

| Column | Meaning |
|---|---|
| Class | the exported class name, as exported |
| Part | the roadmap task of the part that specified the class (T01-102 to T01-113) |
| Covering page | the page of that part where the class is described (section 1) |
| Named | `name` (the exported name is on the page), `S twin` (only its saved name is), `rule` (a family rule covers it, section 3) |
| In the data | `same` (a type table has this name), `S twin` (a type table has the `S` name), `other name` (saved under a different name, section 5), `-` (binary only) |
| Functions | exported member functions of the class (the sum is 10,502) |
| Member table | the name of the DLL's member table of the class (the `S` twin's name for most; an alias where the registered name differs, section 6) or `-` |
| Note | `also in T01-1xx`: the other part's page also has a table row or heading for the class (section 4) |

### 3. Family rules (the 146 classes not named literally)

Each of these families is stated on the part's page and its partner class (the entity, or the weapon or ammunition stem) is named there; the check found the partner on a page of the part for **146 of 146** classes.

| Part | Family | Classes | Page | The rule on the page |
|---|---|---|---|---|
| T01-104 | `...Properties` | 17 | [`brain-classes`](brain-classes.md) | 20 `C<Kind>BrainProperties`, empty shells (table of the overview) |
| T01-103 | `...Properties` | 17 | [`puppet-classes`](puppet-classes.md) | one properties class `C<Kind>PuppetProperties` or `C<Kind>CharacterProperties` per entity class; section 1.2 |
| T01-108 | `...Properties` | 22 | [`item-classes-vitals`](item-classes-vitals.md) | every `C<Name>ItemEntity` has a `C<Name>ItemProperties` (section 1) |
| T01-102 | `...InventoryItem` | 23 | [`players-session-classes`](players-session-classes.md) | one `C<Kind>InventoryItem` per weapon and ammunition kind (13 + 10), all obsolete; section 6.3 lists the 23 stems |
| T01-108 | `...Properties` | 22 | [`item-classes-ammo-weapons`](item-classes-ammo-weapons.md) | every `C<Name>ItemEntity` has a `C<Name>ItemProperties` (section 1) |
| T01-106 | `...Properties` | 17 | [`weapon-classes`](weapon-classes.md) | 17 `C<Weapon>WeaponProperties`, empty (overview table) |
| T01-106 | `...RenderingParams` | 15 | [`weapon-classes`](weapon-classes.md) | 16 rendering parameter classes `C<Weapon>WeaponRenderingParams` (the blades have none) |
| T01-108 | `...Properties` | 3 | [`item-classes`](item-classes.md) | every `C<Name>ItemEntity` has a `C<Name>ItemProperties` (section 1) |
| T01-106 | `...Params` | 5 | [`weapon-classes`](weapon-classes.md) | the five weapons whose parameter class is not spelled out, `C<Weapon>WeaponParams` |
| T01-110 | `...Properties` | 2 | [`worldcontent-effects`](worldcontent-effects.md) | the 18 properties classes pair with the 18 entity classes (overview) |
| T01-110 | `...Properties` | 1 | [`worldcontent-classes`](worldcontent-classes.md) | the 18 properties classes pair with the 18 entity classes (overview) |
| T01-110 | `...Properties` | 2 | [`worldcontent-sound-media`](worldcontent-sound-media.md) | the 18 properties classes pair with the 18 entity classes (overview) |

Most of these classes are **empty shells** (no member, no method: the entity framework needs a properties class for every entity class). A reader who needs one finds it by replacing `Entity` with `Properties` (and so on) in the partner's name.

### 4. One part per class; cross references

Each class is in the family of exactly one part (the split is a function of the name), so no class is in two parts and none is in none. **102 classes** are named on the pages of another part as well (cross references: for example the HUD pages name the player puppet), and for **13** another part's page has a table row or heading for the class:

| Class | Assigned to | Also has a row on |
|---|---|---|
| `CBulletParameters` | T01-113 | T01-107 |
| `CBulletsItemEntity` | T01-107 | T01-108 |
| `CCannonBallsItemEntity` | T01-106 | T01-108 |
| `CCannonItemEntity` | T01-106 | T01-108 |
| `CDamageMultipliers` | T01-103 | T01-113 |
| `CGlobalPlayerParams` | T01-102 | T01-113 |
| `CLauncher` | T01-113 | T01-107 |
| `CPlayerDeathHudElement` | T01-102 | T01-111 |
| `CPlayerDeathHudElementParams` | T01-102 | T01-111 |
| `CPuppetSoundRanges` | T01-103 | T01-113 |
| `CVotingInfoHudElement` | T01-102 | T01-111 |
| `CVotingInfoHudElementParams` | T01-102 | T01-111 |
| `CWeaponItemEntity` | T01-106 | T01-108 |

Resolution: each class stays with the part in the first column's **Assigned to** (the T01-026 split). Eight are the boundary cases between neighbouring families: the weapon part defines `CWeaponItemEntity`, `CCannonItemEntity` and `CCannonBallsItemEntity` and the item part describes their pickup rules (`item-classes-ammo-weapons.md`); the projectile part defines `CBulletsItemEntity`; the player part defines the two HUD elements `CPlayerDeathHudElement` and `CVotingInfoHudElement` and their parameter classes, which the HUD part tables again for the 44-element database. Five involve this part's pages: `CBulletParameters` and `CLauncher` are defined here (their layout and defaults, `helper-classes-params.md`) and used by the projectile part, and `CDamageMultipliers`, `CGlobalPlayerParams` and `CPuppetSoundRanges` are records of the global parameters that belong to the puppet and player parts and are tabled on `helper-classes-params.md` only as members of `CGlobalGameParams`. No two pages disagree on any of the 13 (each part's class count, 113 items, 83 weapons, 32 projectiles and the others, includes the classes assigned to it and excludes the rest, as each page states).

### 5. Classes saved under another name, and the 427 / 505 split

By the survey's rule (the exported name or its `S` twin is a type table of the data) **427** classes are in the data (178 same name, 249 `S` twin) and **505** only in the binary. **11** of the 505 are in fact saved under another name, so the true counts are **438 in the data and 494 binary only**:

| Exported class | Saved as | Part |
|---|---|---|
| `CBAFoeAssigning` | `CBAPlayerAssigning` | T01-105 |
| `CExternalJointEntity` | `CTestJointEntity` | T01-109 |
| `CExternalJointProperties` | `CTestJointProperties` | T01-109 |
| `CLPEnvironmentParams` | `SPuppetEnvironmentParams` | T01-113 |
| `CMiscPuppetParams` | `CMiscPupeptParams` | T01-103 |
| `CObjectManipulationParams` | `CThrowingParams` | T01-113 |
| `CParticleEffectEntity` | `CEffectEntity` | T01-110 |
| `CParticleEffectProperties` | `CEffectProperties` | T01-110 |
| `CSeriousJumpHudElementParams` | `CJumpSimpleHudElementParams` | T01-111 |
| `CSeriousStrengthHudElementParams` | `CStrengthSimpleHudElementParams` | T01-111 |
| `CShooterEntity` | `SBulletShooterEntity` | T01-107 |

The mappings come from the earlier parts (world content, projectiles, mechanisms, behaviours, HUD, puppets) and, for `CObjectManipulationParams` and `CLPEnvironmentParams`, from this part (defaults against the data, `helper-classes-params.md`). Per part this agrees with the parts' own statements where they give numbers: puppets 50 = 49 + 1 (the misspelled `CMiscPupeptParams`), world content 25 = 23 + 2, HUD 47 = 45 + 2; the mechanism, projectile and behaviour parts name their mappings in text (the external joint pair, the shooter, `CBAFoeAssigning`); helpers 32 = 30 + 2.

### 6. Member tables

The DLL declares **227** class member tables (name, parent, members). **199** belong to exported classes and **28** to menu gadget and layout classes that the DLL does not export (`menu-gadget-members.md`: all 28 are named there).

| How a table is named on the pages | Tables |
|---|---|
| by the table's own name | 194 |
| by the class it belongs to (the `S` twin's table names the `C` class) | 31 |
| by a family rule (their classes are rule-covered properties classes: `SShakerEffectProperties`, `SStaticSoundProperties`) | 2 |

**Tables registered under a name that is neither the class's nor its `S` twin** (12): the table's name, the exported class it belongs to, and the part:

| Table | Exported class | Part |
|---|---|---|
| `CSamTestRunParams` | `CSamProjectParams` | T01-113 |
| `GoToPointProperties` | `CGoToPointProperties` | T01-105 |
| `GoToPointEntity` | `CGoToPointEntity` | T01-105 |
| `GoToPointByPathProperties` | `CGoToPointByPathProperties` | T01-105 |
| `GoToPointByPathEntity` | `CGoToPointByPathEntity` | T01-105 |
| `CBAPlayerAssigning` | `CBAFoeAssigning` | T01-105 |
| `CEffectProperties` | `CParticleEffectProperties` | T01-110 |
| `CEffectEntity` | `CParticleEffectEntity` | T01-110 |
| `CTestJointProperties` | `CExternalJointProperties` | T01-109 |
| `CTestJointEntity` | `CExternalJointEntity` | T01-109 |
| `SMoviePlayerentity` | `CMoviePlayerEntity` | T01-110 |
| `SBulletShooterEntity` | `CShooterEntity` | T01-107 |

(`GoToPoint...` have no first letter, `CSamTestRunParams` is the session options class, `SMoviePlayerentity` is a misspelling with a lower case `e`, `CTestJoint...` are the external joints and `CEffect...` the particle effect entities.)

**Tables whose members are not all spelled on the pages** (36; "k of n" = member names of the table that occur as words on some page of the twelve parts): `GoToPointEntity` (2 of 4), `GoToPointByPathEntity` (6 of 10), `SPuppetProperties` (22 of 40), `SGBActionListBehaviorProperties` (0 of 1), `SGBARunOverFoeBehaviorProperties` (5 of 7), `SGBARunOverFoeBehaviorEntity` (6 of 13), `SBAShoot` (4 of 7), `SSimpleEffectProperties` (3 of 16), `SSoundEffectProperties` (3 of 10), `SMissionObjectiveEntity` (7 of 26), `SPlayerPuppetParams` (11 of 24), `SBaseProjectileProperties` (1 of 11), `CTestJointEntity` (2 of 13), `SPropProperties` (16 of 25), `SStaticSoundProperties` (4 of 13), `SCameraEnvelopeProvider` (0 of 1), `SKillerEntity` (6 of 9), `SMoviePlayerProperties` (0 of 4), `STacticProperties` (1 of 7), `SAnimatedMoverEnvelopeProvider` (0 of 1), `SAircraftPuppetParams` (11 of 24), `SHovercraftPuppetParams` (4 of 18), `SCircularSawWeaponParams` (3 of 9), `SComboWeaponsItemProperties` (0 of 5), `SComboWeaponsItemEntity` (1 of 5), `SMiniGunWeaponParams` (2 of 6), `SSniperWeaponEntity` (11 of 15), `SDamageHudElement` (4 of 7), `SHorizontalWeaponsListHudElement` (4 of 18), `SScannerHudElementParams` (16 of 20), `SWeaponsListHudElementParams` (5 of 12), `SHorizontalWeaponsListHudElementParams` (12 of 22), `SSimpleHudElement` (12 of 21), `SScannerHudElement` (2 of 6), `SWeaponsListHudElement` (5 of 11), `CMenuGadgetImageSelectBoxLayout` (6 of 24). The measure counts a member as spelled when its declared name occurs on a page; pages that name a member by its data number or a variant spelling count as missing, so this is an **upper bound** of the real gap. The parked task is `T01-216` (list and name the missing members of these tables; the data-side ones are named by the data's own type tables).

### 7. Classes mentioned on a part's pages but not exported (check 4)

Method: every word on the pages of a part that matches the part's family pattern (the T01-026 split's pattern) and is not an exported class; split into the `S` twins of exported classes (the saved names: **265** words, not listed) and the rest:

| Part | Kind | Count | Names |
|---|---|---|---|
| T01-103 | neither exported nor in the data | 1 | `CCharacterProperties` |
| T01-104 | neither exported nor in the data | 4 | `CCharacterBrainParams`, `CDummyFoeBrainParams`, `CLeggedBrainProperties`, `CZombieSoldierBrainParams` |
| T01-105 | in the data, no export of that name or twin | 8 | `CBADefaultAction`, `CBAList`, `CBANoAction`, `CBAPlayerAssigning`, `CBAVTTermCombiner`, `CBAValidityTerm`, `CBehaviorAction`, `CBehaviorActionList` |
| T01-105 | neither exported nor in the data | 28 | `CBA`, `CBAVT`, `CBaseGenericBehavior`, `CBehavior`, `CExitFromRide`, `CGBA`, `CGBAAnimation`, `CGBAAnimationBehavior`, `CGBALeapOntoFoe`, `CGBALeapOntoFoeBehavior`, `CGBALocalMove`, `CGBALocalMoveBehavior`, `CGBAMelee`, `CGBAMeleeBehavior`, `CGBARunOverFoe`, `CGBARunOverFoeBehavior`, `CGBAShoot`, `CGBAShootBehavior`, `CGBAWait`, `CGBAWaitBehavior`, `CGBActionListBehavior`, `CGoToPoint`, `CMC`, `CPossessRide`, `CTactic`, `CThinkPointOscilator`, `CTurretBehavior`, `SFoeManager` |
| T01-107 | in the data, no export of that name or twin | 5 | `CBulletFireing`, `CCrumbsLayout`, `CCrumbsRenderable`, `CDebrisToolEntityCreator`, `SBulletShooterEntity` |
| T01-107 | neither exported nor in the data | 2 | `CDebrisLaunchParams`, `SBulletShooterProperties` |
| T01-109 | in the data, no export of that name or twin | 7 | `CGenericMotorJoint`, `CGenericMotorJointTemplate`, `CGenericMotorJointValues`, `CSpawnEffectConfiguration`, `CSpawnTemplateProvider`, `CTestJointEntity`, `CTestJointProperties` |
| T01-109 | neither exported nor in the data | 3 | `CExternalJoint`, `CTestJoint`, `SWorldInfo` |
| T01-110 | in the data, no export of that name or twin | 10 | `CEffectEntity`, `CEffectProperties`, `CEntityProperties`, `CFlareConfiguration`, `CFlareRenderable`, `CSpawnEffectConfiguration`, `CTextEffect`, `CTextEffectEntity`, `CTextEffectProperties`, `CVisBoxRenderable` |
| T01-110 | neither exported nor in the data | 2 | `CDebug`, `SMoviePlayerentity` |
| T01-111 | in the data, no export of that name or twin | 2 | `CJumpSimpleHudElementParams`, `CStrengthSimpleHudElementParams` |
| T01-111 | neither exported nor in the data | 1 | `CPowerUpHudElementParams` |
| T01-112 | neither exported nor in the data | 9 | `CMSOptimatchDetails`, `CMenuScreen`, `CPopupMenu`, `SLoadGameMenuScreen`, `SMatchListMenuScreen`, `SMultiplayCharacterMenuScreen`, `SOptionsMenuScreen`, `SPopupMenuUnderlay`, `SSingleplayerMenuScreen` |
| T01-113 | in the data, no export of that name or twin | 10 | `CAnimEvent`, `CAnimEventHandler`, `CAnimEventHolder`, `CPlayerWeaponEffectForOneHandAnimEvent`, `CRenderMuzzleFlashAnimEvent`, `CSamAnimEventHandler`, `CSamEffectAnimEvent`, `CSamShakeEffectAnimEvent`, `CSamSoundAnimEvent`, `CStandAloneSimpleEffectAnimEvent` |
| T01-113 | neither exported nor in the data | 2 | `CBaseProjectParams`, `CEntityIterator` |

The first kind ("in the data, no export") are classes the data has and the DLL does not export (base classes of the engine, saved-only names and the 11 renamed classes' data names); the second ("neither") are **stems and shorthand forms** the pages use in running text (`CBA`, `CGBA`, `CMC`, `CGoToPoint` and similar are prefixes of families) and names the pages say **do not exist** (`CCharacterBrainParams`, `CDummyFoeBrainParams`, `CZombieSoldierBrainParams`, `CLeggedBrainProperties`, `CCharacterProperties`, `CPowerUpHudElementParams`: `brain-classes.md`, `puppet-classes.md`, `hud-classes.md`). None is an exported class missing from the table.

### 8. Gaps: what this inventory does not cover

| Gap | Size | Follow-up |
|---|---|---|
| Classes named on a page but described only by a family rule | 146 classes (mostly empty shells) | none needed unless a family turns out not to be empty; the parts' own open questions apply |
| Member names of the DLL's tables that no page spells out | up to 36 tables | **T01-216** |
| The 605 data classes with no export in this DLL (engine classes, `S` value classes, editor classes) | 605 | `classes.md` and `classes-catalog.md` list them; no spec reads them from the engine modules (**T01-218**) |
| The engine and tool modules' own classes (`Engine.dll`, `Core.dll`, the GUI and graphics modules) | not surveyed | **T01-218** |
| The depth of each part (each page lists its own open questions with follow-up tasks) | all | the parts' open questions (T01-114 to T01-206 and T01-207 to T01-219) |
| The umbrella T01-026 line | to be ticked by its owner after review | not ticked by this part |

## Invariants

1. The two tables hold 932 rows, 932 distinct class names, 12 parts; the per-part counts are 52, 101, 61, 95, 83, 32, 113, 82, 55, 95, 104, 59.
2. The sum of the rows' function counts is 10,502.
3. The rows with `-` in the data column number 494, with `other name` 11, with `same` 178 and with `S twin` 249.
4. No row has the covering page `none` (0 of 932).

## Open questions

1. Whether the engine and GUI modules' classes (which the data also uses) need their own inventory: **T01-218**.
2. The member names missing from the pages (section 6): **T01-216**.
3. Whether the T01-026 line can be ticked: the owner decides after reading this page; this part does not tick it and does not touch the umbrella issue.

## Test vectors

1. Look up `CPuppetEntity`: part T01-103, page `puppet-entity-classes`, named, in the data as `S` twin `SPuppetEntity`, member table `SPuppetEntity`.
2. Look up `CAutoShotgunInventoryItem`: part T01-102, covered by the rule of section 3 (the page names the stem `CAutoShotgun` in section 6.3), not in the data, no member table.
3. Look up `CParticleEffectEntity`: part T01-110, data under another name `CEffectEntity`, member table `CEffectEntity`.
