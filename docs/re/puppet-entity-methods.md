# The 519 methods of `CPuppetEntity` (T01-103, part of T01-026)

- **Roadmap task:** T01-103 (overview [`puppet-classes.md`](puppet-classes.md); the class, its members and rules: [`puppet-entity-classes.md`](puppet-entity-classes.md))
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the method names (the 519 named exports of `CPuppetEntity`, one table row each, none missing: checked against the export table); for the sentences: **medium** for rows with Basis `R` (103 rows) and `P` (181), **low** for `N` (235, name only). Nothing was run.

Prerequisites: [`puppet-entity-classes.md`](puppet-entity-classes.md) (members, state machine, damage rules), `puppet-classes.md` (enumerations), `replication.md`.

## Summary

Every named method exported by `CPuppetEntity`, grouped by role, with one plain sentence for what it is for. **Basis**: `R` = the function was decompiled and its text read for this sentence (the large ones in outline); `P` = the strings, constants and callees of the function were listed and the sentence follows them; `N` = the sentence follows the method name only. Parameter lists and return types are not recorded. Strings in backticks and quoted error texts are the binary's own. Methods whose name starts with `mac` are **script functions** (callable from level scripts, `level-logic-api.md`); the remote calls (run on every machine) are listed in `puppet-entity-classes.md` section 6. The standard members (default and copy constructor, destructor, two operators, one special member and `mdGetDataType`) exist as for every class (`players-session-classes.md` section 1) and are not listed.

## Evidence

Export table (527 exported symbols of `CPuppetEntity`: 519 named functions, 7 standard functions and `md_pdtDataType`); size, strings, constants and callees of each function were listed with a read-only Ghidra script that is not committed (`CLEANROOM.md`); 103 functions were decompiled and read. No function was run. The grouping and sentences are ours.

## Layout / rules

The tables below; section titles are ours.

### Life cycle, network and debug

| Method | What it does | Basis |
|---|---|---|
| `OnCreate` | Builds the puppet from its properties: reads the parameter file (`GetPuppetParameters`), copies the placed `ep_` values into the `en_` members, makes the model, mechanism and sound sources, initialises sound sources for the moving, not-moving and diving sounds, and warns when a puppet uses a local resource (that cannot work in a network game) or is a multi-spawner template with a death spawner. | P |
| `OnBoot` | Starts a created puppet: marks the death spawner, shows the default attachments, plays the alive sound and effects, clears the rider list on the host and sets the first state. | R |
| `OnShutDown` | Stops a puppet: deletes child entities and tool model instances and clears the rider list. | P |
| `OnDelete` | Removes the puppet from the world: its brain, riders, tools, carried object and the world info bookkeeping. | P |
| `OnUpdate` | Applies a changed properties object: copies the death spawner (a multi-spawner template is refused with the warning `Multispawner template can't be used as death spawner!`), the tactic entity and the spawner link. | R |
| `OnStep` | The per-step update (section 6 of this page): runs the profiler scopes `Puppet::OnStep`, `debug_info`, `mech_evalanim`, `sound_handling`, `eval_animevents`, `lerp_vel&look`, handles idle notification, tools, carrying, feelers, flocking, ride-rider relations and the health bar, and reports `Character in infinity detected!` when the placement stops being finite. | R |
| `ResetDynamicMembers` | Resets the run-time members (touch data, desired movement, goal point, attack state, tools list) to their start values. | P |
| `ResetVelocityMultipliers` | Sets the speed, mover, acceleration, deceleration, turning and patrol multipliers back to 1. | N |
| `UpdateTemps` | Refreshes derived temporary values each step (name; small helper). | N |
| `ContributeToGameTotalStats` | Adds this puppet (or its death spawner) to the level's total-enemies count. | P |
| `CanSpawn` | Answers whether the entity may be spawned (always yes in the base class). | R |
| `GetNetBehavior` | `NEB_DYNAMIC` when the puppet has no spawner handle and is not a multi-spawner template, otherwise `NEB_STATIC` (`replication.md`). | R |
| `GetUpdatePriority` | 8.0 while `en_bAlive` is set, 2.0 otherwise. | R |
| `GetRelevantDistance` | Distance within which a client receives the puppet: the effective cull distance of its model; 0 for a puppet without a model and, on the host, for a model with one renderable flag set (not resolved). | R |
| `PreSendUpdate` | Before a net update is built: stores the values a client needs (velocity when `ShouldSyncVelocity` says so). | P |
| `PostReceiveUpdate` | After a net update was applied: re-attaches to the ride, restores the placement and syncs the velocity. | P |
| `ShouldSyncVelocity` | True when the puppet is a non-predicted mover that did not just arrive and whose mechanism should send its velocity. | R |
| `IsLocal` | True when the puppet's brain belongs to this machine's player (client index match, or host and the host's player slot). | R |
| `IsLocalAtClient` | True on a client when the puppet is the local player's. | N |
| `IsMultiSpawnerTemplate` | Returns the template flag (`en_bMultiSpawnerTemplate`). | R |
| `IsSpawner` | True when the puppet is itself a spawner (the base class answers no; the property class forwards the question). | N |
| `AddDebugEntites` | Adds the puppet's related entities (brain, ride, riders, spawner) to the debug list. | P |
| `AddDebugInfo` | Shows a long debug text (abs placement, body placement, moving, move type, goal point, look target, riders, current marker, mechanism bounding box). | P |

### Parameters, model, mechanism, collision and placement

| Method | What it does | Basis |
|---|---|---|
| `GetDefaultModel` | The built-in fallback model `Content/SeriousSam2/Models/Default/Box.mdl`. | P |
| `GetPuppetModel` | The model configuration of the character (instance override, else the parameter file's body model, else the default model). | N |
| `GetPuppetStretch` | The scale factor of the puppet (instance stretch, else the parameter file's, else 1). | N |
| `GetPuppetParameters` | Loads the puppet's parameter file (section 3.2 of `puppet-classes.md`). | R |
| `GetPuppetParamsClass` | The class the parameter file must be derived from; the base class answers "invalid" and each movement-model class overrides it. | R |
| `IsGenericPuppet` | False in the base; the six Character classes answer true (section 3.1 of `puppet-classes.md`). | R |
| `GetDefaultMechanism` | The mechanism id the model must have (`Default`). | P |
| `SetDefaultMechanism` | Remembers another default mechanism id. | N |
| `GetDefaultSittingMechanism` | The mechanism used while the puppet sits in a ride (`Default`). | P |
| `GetSittingMechanism` | The mechanism id a rider uses on a seat (from the seat data, else the default sitting mechanism). | N |
| `SetupAspects` | Creates the aspects (attachment points) of the model that the puppet needs: sound sources, effect points, tool limbs, shoot origins. | P |
| `SetCurrentMechanism` | Switches the puppet to a named mechanism of its model (physics body and joints), error text `Invalid mechanism provided to CPuppetEntity::SetCurrentMechanism` and `... doesn't have mechanism ...` when the model lacks it. | P |
| `DeleteCurrentMechanism` | Destroys the current mechanism body and its handles. | N |
| `RestoreLastMechanism` | Returns to the mechanism that was set before the last switch. | N |
| `DeleteMechanismAndRenderable` | Destroys mechanism and model renderable (corpse removal). | N |
| `GetMechanism` | Returns the current mechanism. | N |
| `GetModelRenderable` | Returns the model renderable. | N |
| `GetAllAspects` | Lists all aspects of the model. | N |
| `GetAspectForAttaching` | Returns the aspect that a tool or effect attaches to. | N |
| `SetCollisionCategory` | Sets the collision category of the body. | N |
| `GetCollisionCategory` | Chooses the collision category by barrier mode and state: `character`, `character_through_barrier` or `character_blocked_by_custom_barrier`. | P |
| `GetBoundingBox` | The bounding box of the model for culling and placement. | N |
| `GetBoundingBoxForAutoAiming` | The box used for the player's auto-aim (name). | N |
| `GetAbsPlacement` | The world placement (from the mechanism when it has one, else the stored placement). | N |
| `SetAbsPlacement` | Moves the puppet (mechanism and renderable). | N |
| `GetDefaultPlacement` | The placement the puppet was created with. | N |
| `GetVelocity` | The linear velocity of the mechanism. | N |
| `SetVelocity` | Sets the linear velocity. | N |
| `GetAngVelocity` | The angular velocity. | N |
| `SetAngVelocity` | Sets the angular velocity. | N |
| `GetDipContent` | The medium the puppet stands in (`DC_LAND` 0, `DC_AIR` 1, `DC_WATER` 2 in the base class: always the first answer; legged puppets override). | R |
| `Teleport` | Teleports to a placement, resetting velocity and the touch data. | N |
| `RepositionPuppet` | Moves the puppet to a new place keeping the mechanism consistent (name). | N |
| `LerpIntoPlacementEntity` | Moves the puppet smoothly onto the placement of another entity (`en_henPlacementEntity`). | N |
| `SetPlacementEntityRemote` | The net call that sets the placement entity on the clients. | N |
| `EnforcePuppetMoveLook` | Applies the desired movement and look to the body (overridden by every movement model). | N |
| `LerpToDesiredVelocityAndLook` | The movement step: turns the desired velocity and look direction into the real motion; **the central override of each movement model**. | N |

### Movement, goal point, feelers, idle and local moves

| Method | What it does | Basis |
|---|---|---|
| `SetDesiredTempo` | Stores the desired velocity (move type `MT_DESIRED_TEMPO_LOOK`). | N |
| `SetDesiredLookDir` | Stores the desired look direction. | N |
| `SetDesiredTempoAndLookDir` | Stores both at once. | N |
| `SetDriveSteerRatioAndLookDir` | Stores a drive and steer ratio (move type `MT_DRIVE_STEER_LOOK`, used by rides). | N |
| `CalculateDesiredTempoAndLookDir` | Computes the desired velocity and look direction for the current move type from the goal point, patrol or operator input. | N |
| `CalculateMovingAndRotatingRatio` | Computes how fast the puppet moves and turns relative to its limits (profile scope names `MovingRatio`, `RotatingRatio`). | P |
| `AdjustDesiredTempoWhenCantStrafeHorizontally` | Corrects the desired velocity for a puppet that cannot strafe (name). | P |
| `AdjustMinSteeringRadius` | Raises the minimum steering radius when the puppet cannot turn tightly enough. | N |
| `CanAddHorizontalThrustInDirection` | Whether a horizontal push in a direction is allowed by the movement abilities. | N |
| `CanRotateInPlace` | Whether the puppet may turn on the spot (base: yes). | R |
| `MovingIn3DArea` | Whether the puppet moves freely in three dimensions (base: no; flyers answer yes). | R |
| `GetDesiredMovingSpeed` | The desired speed (name). | N |
| `GetMaxAxesVelocities` | The maximum velocity per axis (flyers). | N |
| `GetCurrentAcceleration` | The current acceleration limit (name). | R |
| `GetCurrentDeceleration` | The current deceleration limit (name). | R |
| `GetDeclaredNominalVelocity` | The velocity written in the parameters for the current situation. | R |
| `GetCurrentNominalVelocity` | The nominal velocity times the current velocity multiplier. | P |
| `GetCurrentVelocityMultiplier` | The product of the speed, mover and difficulty-dependent multipliers (reads the game info entity). | P |
| `GetSpeedMultiplier` | The puppet's speed multiplier (`en_fSpeedMultiplier`). | N |
| `GetMovementSpeedMultiplierForMaterial` | The slow-down of the floor material under the puppet. | N |
| `GetMaxJumpSlopeAngle` | The steepest slope a jump may start from (base value; legged puppets use their parameters). | R |
| `GetLookRotationVelocity` | How fast the puppet turns its look direction. | R |
| `GetLookDirEul` | The look direction as Euler angles. | N |
| `GetCurrentLookDirEul` | The look direction now. | N |
| `GetFrontDirEul` | The front direction as Euler angles. | N |
| `GetFrontDirection` | The front direction as a vector. | N |
| `GetLookTargetAngle` | The angle between the front and the look target. | N |
| `GetMatchDesiredLookTime` | The time needed to match the desired look direction (used by attacks that must face the foe). | N |
| `SetLookTarget` | Sets the entity to look at. | N |
| `SetAddedPitch` | Sets the pitch that is added to the desired look direction. | N |
| `OnLookDirMatched` | Called when the look direction reached the desired one (continues a spawn or an attack that waited for it). | P |
| `OnPuppetRotationFinished` | Called when a rotation that an attack waited for is complete. | R |
| `StopMoving` | Cancels the desired movement. | N |
| `ForceRunTroughPoint` | Orders a run through a point without stopping. | P |
| `PushAbs` | Pushes the puppet by an absolute impulse. | N |
| `PushRel` | Pushes the puppet by an impulse relative to its heading. | N |
| `AddForceImpulseManual` | Adds a force impulse (debug-drawn when requested; strings `Manual vForceImpulse`). | P |
| `ReceiveForceImpulse` | Receives an impulse from an explosion or hit (base: ignored; rolling balls and vehicles override). | R |
| `ApplyDamageImpact` | Turns a damage hit into a velocity change (debug view `Damage impact`). | P |
| `DoFlocking` | Keeps a distance from neighbouring puppets (`fDesiredDistance`, `fAttractionFactor`). | P |
| `GetAttractionFactor` | The flocking attraction value. | N |
| `ProcessFeelers` | Casts the feeler rays and steers around obstacles (profile scope `local_nav_feelers`). | P |
| `CheckEngageFeelers` | Decides whether the feelers must be switched on. | N |
| `AssureThatOnStepIsEnabled` | Makes sure the step hook runs (name). | N |
| `PrepareToDisableStep` | Called before the step hook is switched off. | R |
| `MarkNonIdle` | Records that the puppet did something this step so that it is not idle. | N |
| `RequestIdle` | Asks the brain to hold the puppet idle for a period. | N |
| `IsIdle` | True when no state, move or attack is active. | N |
| `DoIdle` | Starts the idle state. | N |
| `EndIdle` | Ends the idle state. | R |
| `DoLocalMove` | Plays a local move (a scripted short walk with its own animation). | N |
| `BeforeLocalMove` | The think routine before the local move starts. | P |
| `DuringLocalMove` | The think routine while it runs. | P |
| `AfterLocalMove` | The think routine after it. | P |
| `AfterLocalMoveFinished` | Finishes the local move. | N |
| `OnLocalMoveStarted` | Hook: a local move started. | R |
| `OnLocalMoveFinished` | Hook: a local move finished. | N |

### Goal points, markers and patrol

| Method | What it does | Basis |
|---|---|---|
| `SetGoalPoint` | Sets the goal point. | N |
| `SetRotatedGoalPoint` | Sets a goal point and the rotation to take there. | N |
| `SetRunTroughPoint` | Sets a pass-through goal point. | N |
| `SetupGoalPoint` | Common setup of a goal point. | N |
| `UpdateGoalPoint` | Moves the goal point. | N |
| `GoalPointCommonReset` | Clears the goal-point flags. | P |
| `GetGoalMarker` | The marker the goal point belongs to. | N |
| `OnGoalPointReached` | Hook: the goal point was reached (debug value `fReachedDistance`). | P |
| `GetClosestMarker` | The closest path marker (class `CPathMarkerEntity`) to the puppet. | P |
| `GetLastReachedMarker` | The last marker the puppet reached. | N |
| `ObtainFirstPathMarker` | Finds the first marker of an automatic patrol. | N |
| `SetCurrentPath` | Sets the current path. | N |
| `SetTacticEntity` | Sets the tactic entity. | N |
| `PatrolMarkers` | Starts a patrol over path markers. | N |
| `StopPatrolingMarkers` | Stops it. | N |
| `ContinuePatrolingMarkers` | Resumes a patrol. | N |
| `PrepareForAutoPatroling` | The think routine that starts the automatic patrol at the first marker. | P |
| `StartAutoPatroling` | Starts the automatic patrol. | N |
| `DoStartPatroling` | The remote call that starts a patrol on every machine. | N |
| `DoStopPatroling` | The remote call that stops it. | N |
| `SetPatrolingSpeedMultiplier` | Sets the patrol speed multiplier. | N |
| `SetPatrolingRotateSpeedMultiplier` | Sets the patrol turn-speed multiplier. | N |
| `ObtainControlledPuppets` | Lists the puppets this one controls (riders and carried). | P |

### Senses, foes and brain links

| Method | What it does | Basis |
|---|---|---|
| `CanSeeFoe` | A line-of-sight test to the foe (script event `character_sees_character`). | P |
| `ShouldExamineThreatSound` | Whether a heard sound is examined (base: yes; turrets say no). | N |
| `GetThreatSoundOrigin` | The point from which the puppet hears sounds. | N |
| `SetAlignment` | Sets the alignment. | N |
| `ForceFoe` | Gives the brain a foe by force (error text `Foe to force is not valid`, `Puppet doesn't have brain`). | P |
| `HuntFoe` | Gives the brain a foe and orders it to hunt it (same errors). | P |
| `LoseFoe` | Makes the brain forget its foe (error `Puppet doesn't have brain`). | P |
| `ExecuteAIMoment` | Runs an AI moment through the moment handler (`brain-classes.md`). | N |
| `IsAimable` | Whether the puppet can be aimed at. | R |
| `IsAutoAimable` | Whether the player's auto-aim may lock on it. | N |
| `GetHealthColor` | The colour of the health bar. | N |
| `AddHealthBar` | Creates the health bar of a boss or mini-boss. | N |
| `HandleMiniBossChange` | Adds the bar when the mini-boss flag turns on. | P |

### Brain

| Method | What it does | Basis |
|---|---|---|
| `CreateBrain` | Creates the brain (section 2.3 of `brain-classes.md`). | P |
| `DeleteBrain` | Deletes the brain. | N |
| `ChangeToBrain` | Hands control to another brain (a player entering a ride). | N |
| `ChangeToDefaultBrain` | Returns control to the default brain. | N |
| `ConnectToBrain` | Stores the brain and tells it about the puppet. | N |

### Health, armour, damage and invulnerability

| Method | What it does | Basis |
|---|---|---|
| `GetHealth` | Health. | R |
| `SetHealth` | Sets the health. | N |
| `GetMaxHealth` | Maximum health. | R |
| `SetHealthMax` | Sets the maximum health. | N |
| `GetInitialMaxHealth` | The maximum health at creation (name). | R |
| `GetNominalHealth` | The health value used for scoring and bars: the parameter file's health (100 when there is no file); for an evil puppet in co-op a scaled value from the game info (the formula was not resolved). | R |
| `macGetNominalHealth` | Script: returns `GetNominalHealth`. | R |
| `GetArmor` | Armour. | R |
| `SetArmor` | Sets the armour. | N |
| `GetMaxArmor` | Maximum armour. | R |
| `SetArmorMax` | Sets it. | N |
| `IsAlive` | Whether the alive flag is set. | R |
| `CanReceiveHealth` | In deathmatch only below half of the maximum (unless forced), always elsewhere. | R |
| `CanReceiveArmor` | The same for armour (deathmatch rule). | P |
| `ReceiveHealth` | Adds health on the host, never below 0 or above the maximum; in deathmatch (unless forced) not above half of the maximum. | R |
| `ReceiveArmor` | Adds armour (deathmatch rule as for health). | P |
| `ReceiveWeapon` | Weapon pickup (base: ignored). | N |
| `ReceiveAmmo` | Ammunition pickup (base: ignored). | N |
| `ReceivePowerUp` | Power-up pickup (base: ignored). | R |
| `GetHealingInfo` | The healing start, end and per-second values of the parameter file. | N |
| `ApplyHealing` | Heals over time (effects `HealingStarted`, `HealingStopped`, animation `auto_healing`). | P |
| `IsInvulnerable` | True when the invulnerability flag `PNF_INVULNERABILITY` is set, or when a player brain controls the puppet and its boot-invulnerability is active at this moment (not resolved). | R |
| `IsInvulnerableAsRider` | True for a rider whose seat makes non-players invulnerable (`SF_NON_PLAYER_IS_INVOULNERABLE`). | N |
| `BeInvulnerable` | Sets the invulnerability flag. | N |
| `ReceivesDamageFrom` | Whether damage from a given dealer counts (section 7 of `puppet-entity-classes.md`): a player-controlled puppet hit by another player-controlled puppet takes it always in deathmatch and otherwise only when friendly fire is on (`sgp_bFriendlyFire`); damage from a dealer with the same value of a base-entity member (not resolved) is refused unless it is an explosion; a puppet that is neither evil nor neutral (that is good, semi-neutral, or a player) takes damage only from evil dealers. | R |
| `GetReceiveDamageMultiplier` | In co-op, for an evil puppet: 1 divided by (1 + 0.01 x (number of players - 1) x `sgp_iEnemyStrengthPerPlayer`), times 1 divided by (1 + 0.01 x `sgp_iExtraEnemyStrength`); 1 in every other case (section 7 of `puppet-entity-classes.md`). | R |
| `GetApplyDamageMultiplier` | The multiplier for damage this puppet deals (name). | R |
| `OnReceiveDamage` | The damage entry point, host only (section 7 of `puppet-entity-classes.md`): ignored without parameters; the god cheat of a controlling player brain cancels it; invulnerability cancels it unless the damage is the maximum value (2,147,483,647) of another type than the kill-all cheat and the Serious Bomb; the level's boss puppet ignores maximum damage; a hit on a part that carries a shield tool reduces the tool's health instead and throws the tool away at 0 (`CShieldDescription`); the rest (health, armour, death, kick and score) was not read. | R |
| `OnDamageTaken` | Empty hook after damage. | R |
| `ReportDamage` | Hook for reporting damage to the brain. | R |
| `CanReceiveDamageFromMaterial` | Whether floor material damage applies. | N |
| `ApplyMaterialDamage` | Applies damage of the material under the puppet. | N |
| `CanReceiveFallingDamage` | Whether fall damage applies. | R |
| `HandleFallingDamage` | Applies fall damage (empty in the base class). | R |
| `CanReceiveRangeDamage` | Whether area damage applies. | R |
| `ApplyLethalDamage` | Kills the puppet outright. | P |
| `GetLastDamage` | The last damage amount. | R |
| `GetLastDamager` | The last damage dealer. | N |
| `GetKillScore` | The score for killing the puppet (`mpp_slScore` of the parameter file; 0 without one). | R |
| `OnHullTouched` | The collision handler: damage by touch, kicks (debug text `Kicker launch impulse`), floor and wall bookkeeping, `solid` hull test. | P |
| `OnHullTouchedInternal` | The internal part of it (empty in the base). | R |
| `OnKicked` | Hook: the puppet was kicked. | R |
| `ResetTouchingData` | Clears the touch members. | N |
| `ExecuteDestruction` | Destroys the puppet as a destructible (name). | N |
| `IsDestroyed` | True when the puppet is destroyed. | N |
| `IsDestructible` | True when it can be destroyed (base: no). | R |
| `ForcesPiercingProjectileToExplode` | From the parameter file `pbp_bForcesPiercingProjectileToExplode`. | N |
| `GetPiercingProjectileResilience` | The resilience against piercing projectiles (parameter value times its multiplier). | N |
| `BeDestroyedByPeircingProjectile` | Hook (name; the spelling is the binary's). | N |

### Death, explosion and corpse

| Method | What it does | Basis |
|---|---|---|
| `DoDying` | The death routine on the host: tells the world info that the puppet died, calls the death spawner (`CSpawnerEntity::DeathSpawn`), makes riders leave (`DoFallOutFromRide`/exit, by their state), increments a game statistic when the puppet is evil (the kill counter of the level; which statistic is not resolved), sends a script event (name not resolved), clears the alive flag, records the time of death and stops sounds and effects. | R |
| `DoDeathSequence` | Starts the death animation (base: an empty hook; legged puppets override). | N |
| `DoExplode` | Starts the explosion state of a puppet that explodes. | N |
| `ExplodeState` | The think routine of the explosion: applies `DMT_EXPLOSION` damage with the parameter file's exploding damage and radius, then ends. | R |
| `DropDead` | Script: kills the puppet with a given direction (default direction when no argument). | R |
| `PlayDeathSound` | Plays the scheme sound `Death`. | P |
| `FadeOut` | The state that fades the corpse out (starts the corpse-sink effect). | P |
| `GetCorpseStadyPeriod` | How long the corpse stays: the death parameters' value unless it is -1, else the global value (default 1.0 s); the spelling is the binary's. | R |
| `StartCorpseSinking` | Starts sinking the corpse (`ignoreall` collision, then deletion). | R |
| `StartCorpseSinkingEffect` | Plays the disappear effect. | P |
| `SpawnAlliveEffects` | Creates the alive effects (`Alive`, `Alive%1` per variant) from the character scheme. | P |
| `StopSoundsAndEffects` | Stops all sounds and effects of the puppet. | N |
| `HandleToolsActions` | Runs the pending tool actions (draw, pick, store, throw) and sends the script events `ToolDrawn`, `ToolPicked`, `ToolStored`, `ToolThrown`. | P |

### Rides, seats and riders

| Method | What it does | Basis |
|---|---|---|
| `IsRidable` | True when the parameter file has seats. | N |
| `GetCountOfSeats` | The number of seats. | P |
| `FindSeatDataByIndex` | The seat record by index. | N |
| `FindSeatIndex` | The index of a seat name. | N |
| `FindSeatDataByName` | The seat record by name (error `Can't find data for seat '%1' in ride '%2'`). | P |
| `FindBestSeat` | The best free seat for a rider. | P |
| `GetSeatAttachment` | The attachment point of a seat. | N |
| `AdjustSeatForRiderEntering` | Moves the seat for a rider that enters (rolling ball). | R |
| `GetEnterPointAttachment` | The attachment point where a rider enters. | N |
| `GetEnterPointAbsPlacement` | Its world placement. | N |
| `GetSeatAbsPlacement` | The world placement of a seat. | N |
| `MountRiders` | Creates the default riders of a ride from the instance mounts (`Can't find seat`, `Generic character can't be created as mountee`; uses `samCreateGenericCharacter`). | P |
| `SetupDefaultRiders` | Prepares the default rider list. | N |
| `DeleteDefaultRiders` | Deletes it. | P |
| `MountRide` | Mounts this puppet on a ride. | P |
| `AttachToRide` | Attaches the rider to the ride's seat. | P |
| `GetRide` | The ride the puppet sits in. | N |
| `IsConnectedToRide` | True when it is connected to a ride. | N |
| `IsMyRider` | True when a given puppet is one of its riders. | P |
| `GetCountOfRidersInRide` | The number of riders. | P |
| `GetRiderOnSeat` | The rider on a seat. | P |
| `RegisterForRide` | A rider registers (error `Rider tried to register twice onto same ride`). | P |
| `UnregisterFromRide` | A rider unregisters. | P |
| `ProcessRideRiderRelations` | Updates the rider list and sends `AllRidersSittingInRide` and `RiderLost`. | P |
| `OnStartRiding` | A rider starts riding (script event `NewRider`, `StartedRiding`). | P |
| `OnStopRiding` | It stops (`StoppedRiding`). | P |
| `DoStartRiding` | The remote call that sets the riding state to `RS_RIDING` (3) on every machine (read: one store). | R |
| `DoStopRiding` | The remote call for stopping. | N |
| `OnRiderStartedEntering` | Hook: a rider starts to enter. | R |
| `OnRiderStartedExiting` | Hook: a rider starts to leave. | R |
| `OnMyRideStartedMoving` | Hook: the ride started to move. | R |
| `OnMyRideStopped` | Hook: the ride stopped (plays the not-moving rider animation). | P |
| `OnTurnedOn` | The ride was switched on (calls the brain's `OnTurnedOn`). | P |
| `OnTurnedOff` | The ride was switched off. | P |
| `PlayEnteringIntoRideSequence` | Plays the enter sequence of a way to a seat (remote call). | N |
| `PlayExitingFromRideSequence` | Plays the leave sequence (remote call). | N |
| `PlayRiderAnim_RideMoving` | The rider animation while the ride moves. | P |
| `PlayRiderAnim_RideNotMoving` | The rider animation while it stands. | P |
| `ExitAllRidersFromRide` | Makes every rider leave. | R |
| `ExitAllRidersFromRideInternal` | The internal version. | P |
| `FallOutAllRidersFromRide` | Throws every rider out (crash). | R |
| `FallOutAllRidersFromRideInternal` | The internal version. | P |
| `DoFallOutFromRide` | One rider falls out. | N |
| `StoreToolsBeforeRide` | Stores the rider's tools before it sits. | P |
| `RestoreToolsAfterRide` | Restores them afterwards. | N |
| `IsRideFixed` | Whether a broken ride was fixed (name). | N |
| `AutoFixRide` | Starts the automatic fix. | N |
| `AutoFixingRideState` | The think routine of the fix (sends `FinishedRideFixing`). | P |
| `FinishedRideFixing` | Ends the fix. | N |
| `GetRideEnteringCameraPlacement` | The camera while entering. | N |
| `GetRideExitingCameraPlacement` | The camera while leaving. | N |
| `CantBeUsed` | Hook: the puppet cannot be used. | R |
| `EnableUsage` | Marks the puppet usable (`PNF_CAN_BE_USED`). | R |
| `DisableUsage` | Clears it. | R |
| `GetObjectManipulationFlags` | The use flags (`MF_CAN_BE_USED` and others). | N |
| `GetObjectManipulationMessage` | The use prompt text (`ETRSObjectActionMessage.UseButtonToEnter`, `...UseButtonToFix`). | P |

### Character tools and attachments

| Method | What it does | Basis |
|---|---|---|
| `AddDesiredTool` | Asks for a tool on a limb. | N |
| `RemoveDesiredTool` | Cancels the request. | N |
| `IsToolDesired` | True when a tool is requested. | N |
| `GetDesiredToolOnLimb` | The requested tool of a limb. | N |
| `AttachTool` | Attaches the tool model to the limb. | P |
| `DetachTool` | Detaches it. | P |
| `UpdateTools` | Compares desired and attached tools and attaches or detaches. | P |
| `IsToolAttached` | True when a tool is attached. | N |
| `GetAttachedTool` | The attached tool record. | N |
| `GetAttachedToolByID` | The record by tool id. | N |
| `GetAttachedToolOnLimb` | The attached tool of a limb. | N |
| `IsToolFromEntityAttached` | True when a tool made from an entity is attached. | N |
| `GetToolAttachedFromEntity` | The tool record made from a given entity. | N |
| `GetToolRelatedWithAnim` | The tool tied to the playing animation. | N |
| `ConnectLastAnimToTool` | Ties the last animation to a tool. | N |
| `ClearToolAnimRelation` | Removes that tie. | N |
| `MountDefaultTools` | Mounts the parameter file's default tools. | P |
| `DeleteAttachedTools` | Deletes all attached tools. | P |
| `DrawTool` | Script: draws a tool. | P |
| `StartDrawingTool` | Starts the draw animation. | P |
| `PickTool` | Script: picks a tool up (error `Invalid picking animation specified for tool`). | P |
| `StartPickingTool` | Starts the pick animation. | P |
| `StoreTool` | Stores a tool. | N |
| `StartStoringTool` | Starts the store animation. | P |
| `ThrowTool` | Throws a tool. | N |
| `StartThrowingTool` | Starts the throw animation. | P |
| `LaunchTool` | Launches a tool as a projectile along a throw description (`CThrowDescription`; error `Can't find throw description`). | P |
| `PrepareForLaunch` | Hook before a launch. | N |
| `GetThrowDescription` | The throw description by name. | N |
| `ThrowDeathTools` | Throws the tools away when the puppet dies. | P |
| `ThrowPickedTool` | Throws the picked tool. | P |
| `ThrowPickedToolAtTargetEntity` | Throws it at an entity. | P |
| `GetToolRelPlacementInPuppet` | A tool's placement in the puppet. | N |
| `DynamicallyShowAttachment` | Shows an attachment model. | P |
| `ShowDefaultAttachments` | Shows the parameter file's default attachments (`pp_aiShownAttachments`). | P |
| `HideAttachment` | Hides one. | P |
| `OnAnimEventAttachToolFromEntity` | Animation event: attaches a tool made from an entity. | P |
| `OnAnimEventDetachTool` | Animation event: detaches. | P |
| `OnAnimEventDrawTool` | Animation event: draw. | P |
| `OnAnimEventStoreTool` | Animation event: store. | P |
| `OnAnimEventDeathThrowTools` | Animation event: throw the death tools. | P |
| `OnAnimEventHideAttachment` | Animation event: hide an attachment. | P |
| `OnAnimEventShowAttachment` | Animation event: show an attachment. | P |

### Carrying and throwing objects

| Method | What it does | Basis |
|---|---|---|
| `TakeObject` | Starts carrying an object (`ObjectTaken`). | P |
| `ConnectToCarriedObject` | Joins the puppet to the carried object. | P |
| `HandleObjectCarrying` | The per-step carry logic. | P |
| `DeleteCarryingJoint` | Removes the joint (`ObjectThrown`). | P |
| `CollideWithCarriedObject` | Hook: a collision with the carried object. | R |
| `DropObject` | Drops it. | R |
| `ThrowObject` | Throws it. | P |
| `CalculateObjectThrowingRatio` | The throw strength from the aim. | N |
| `OnObjectGrab` | Plays the grab sound and animation (`ObjectGrab`). | P |
| `OnObjectThrowPrepare` | Plays the wind-up (`ObjectThrowPrepare`). | P |
| `OnObjectThrow` | Plays the throw (`ObjectThrow`). | P |
| `OnObjectDropped` | Hook: dropped. | R |
| `OnObjectTaken` | Hook: taken. | R |
| `ExecuteOperatorFiring` | Fires or throws on the operator's button. | P |

### Attacks, shooting and projectiles

| Method | What it does | Basis |
|---|---|---|
| `DoAttack` | Starts an attack by the player fire type. | N |
| `StopAttack` | Stops it. | N |
| `AttackMelee` | Starts a melee attack. | N |
| `AttackShoot` | Starts a shooting attack. | N |
| `GetPlayerFireType` | The fire type of the operator. | N |
| `CanFireFromOwnWeapons` | True when the (rider's) own weapons may fire. | N |
| `IsAllowedToFire` | True when firing is allowed. | P |
| `CanPerformGenericShooting` | True when a scripted shot may start (error `Shooting is rejected for rider`). | P |
| `DoGenericShooting` | The remote call that starts a generic shooting attack. | N |
| `PrepareForCGS` | Starts a consecutive generic shooting: the `CGS` chain below. | P |
| `WaitStartAttackDistance` | Waits until the foe is close enough. | P |
| `AdjustPoseForCGS` | Adjusts the pose for the attack. | P |
| `WaitBeforeCGS` | Waits before the first shot. | P |
| `PlayPreparingAnimForCGS` | Plays the preparing animation. | P |
| `OnEndPreparingAnimForCGS` | Hook: it ended. | N |
| `DoConsecutiveGenericAiming` | Aims. | P |
| `OnAimingFinished` | Hook: aiming ended. | P |
| `DoConsecutiveGenericShooting` | Fires one shot of the series. | P |
| `PlayCGSAnim` | Plays the shoot animation (error `Shoot animation is not defined in shooting process`). | P |
| `OnCGSAnimFinished` | Hook: the animation ended. | P |
| `OnSingleShootFinished` | Hook: one shot ended. | P |
| `OnReloadingAnimFinished` | Hook: reloading ended. | N |
| `UnprepareCGS` | Plays the leaving animation. | P |
| `OnUnpreparingAnimFinished` | Hook: it ended. | N |
| `OnCGSFinished` | The series ended. | P |
| `OnAfterShootingDelayFinished` | The after-shooting delay ended. | P |
| `OnBeginCGS` | Hook: a series began. | N |
| `OnEndCGS` | Hook: it ended. | N |
| `OnContinueCGS` | Hook: it continues. | P |
| `OnAnimEventShootingBlast` | Animation event: fires the blast named in the shooting process. | P |
| `OnAnimEventMeleeDamage` | Animation event: applies the melee or run-over blast of the current attack (`Attack can't be found in melee or run over foe attacks`). | P |
| `CanPerformMeleeAttack` | True when a melee attack may start. | N |
| `DoGenericMeleeAttack` | The remote call that starts a generic melee attack. | N |
| `OnBeginGenericMeleeAttack` | Hook: it began. | N |
| `GenericMeleeAttackState` | The think routine of a melee attack (`Melee attack with ID doesn't exist in character`). | P |
| `OnMeleeAnimFinished` | Hook: the animation ended. | N |
| `OnEndGenericMeleeAttack` | Hook: it ended. | N |
| `CanPerformLeapOntoFoe` | True when a leap may start (base: no). | R |
| `DoGenericLeapOntoFoe` | The remote call that starts a leap. | N |
| `DoGenericAnimation` | The remote call that plays a scripted animation. | N |
| `GenericActionPlayAnim` | The think routine of a scripted animation. | P |
| `OnGenericAnimationFinished` | Hook: it ended. | N |
| `OnEndGenericActionPlayAnim` | Hook. | R |
| `GenericActionStarted` | Hook: a scripted action started. | R |
| `DoEndGenericBehavior` | Ends the generic behavior and returns to the default state. | P |
| `GetExactShootTarget` | The exact point to shoot at. | N |
| `GetShootDirection` | The direction of a shot. | N |
| `GetShootingOriginAttachment` | The attachment a shot starts from (`Barrel01` and its numbered siblings). | P |
| `GetSightsPlacement` | The sights placement of the weapon. | N |
| `FireProjectileFrom` | Launches a projectile from a placement (checks `samIsShootingFromWall`). | P |
| `ThrowProjectileFrom` | Throws a projectile. | P |
| `SpawnProjectile` | Creates the projectile on the host (error `SpawnProjectile executed on client!`). | P |
| `GetProjectileProperties` | Creates the properties object of a generic or flamer projectile. | P |
| `OnFireButtonDown` | Hook: fire button is down. | N |
| `OnFireButtonPressed` | Hook: pressed. | N |
| `OnFireButtonReleassed` | Hook: released (the spelling is the binary's). | R |
| `IsFireButtonPressed` | True when the operator's fire button is down. | N |
| `IsFlippingFireButtons` | True when the buttons are swapped. | R |
| `OnWeaponPutDown` | Hook (empty). | R |
| `OnWeaponRecoiled` | Hook (empty). | R |
| `ShouldShowCrosshair` | Whether a crosshair is drawn for the operator. | R |

### Spawning, appearing and the state machine

| Method | What it does | Basis |
|---|---|---|
| `SetSpawner` | Records the spawner and plays its spawn-in animation (`SpaweeAnimationFinished`). | P |
| `YouWillBeMySpawnSource` | The spawner tells the puppet it will be its spawn source. | N |
| `DoSpawnSource` | Starts the spawn-source state. | P |
| `SpawnSourceStopped` | The spawn source stopped. | P |
| `OnSpawnSourcePrepareAnimationFinished` | Hook: the prepare animation ended. | P |
| `OnSpawnSourceAnimationFinished` | Hook: the spawn animation ended. | P |
| `OnAnimEventReleaseSpawnee` | Animation event: releases the spawned puppet. | N |
| `ManuallyReleaseSpawnee` | Releases it by script. | N |
| `SpaweeAnimationFinished` | Hook: the spawnee's appearing animation ended (spelling as in the binary). | N |
| `Appear` | Script: makes the puppet appear (warnings when called on a spawner or template). | P |
| `DoAppear` | Executes it (warnings `called on a puppet that has mechanism`, `already visible`, `marked as alive`). | P |
| `OnAnimEventAppear` | Animation event: appear: a 5-byte forwarder to `DoAppear` (not empty; corrected in `puppet-spawning.md`). | R |
| `Disappear` | Script: makes it disappear. | P |
| `DoDisappear` | Executes it. | P |
| `OnAnimEventDisappear` | Animation event: disappear: forwards to `DoDisappear` (corrected in `puppet-spawning.md`). | R |
| `SetDefaultState` | Returns the state machine to `PS_DEFAULT`. | R |
| `SetState` | Changes the puppet state (section 4). | R |
| `NewClearState` | Creates a state-clearing helper (name). | N |
| `NewClonedState` | Creates a copy of the state (name). | N |
| `ChangeState` | Changes the state by the brain's order. | N |

### Animation and sound

| Method | What it does | Basis |
|---|---|---|
| `AddAnim` | Adds an animation layer. | N |
| `RemoveAnim` | Removes it. | N |
| `RemoveLayer` | Removes a layer. | N |
| `PlayAnim` | Plays an animation once. | N |
| `PlayAnimLoop` | Plays one in a loop. | N |
| `PlayAnimNet` | Plays one on every machine. | P |
| `PlayAnimNetRemote` | The remote part. | N |
| `PlayAnimWait` | Plays one and waits for it. | N |
| `StopAnim` | Stops it. | N |
| `StopAnimNet` | Stops it on every machine. | P |
| `StopAnimNetRemote` | The remote part. | N |
| `PlayDefaultAnimation` | Plays the default (idle) animation (empty in the base). | R |
| `PlayLifeAnimation` | Plays the life animation `Alive`. | P |
| `AutoAnimationOn` | Turns the automatic move animations on (empty in the base). | R |
| `AutoAnimationOff` | Turns them off (empty in the base). | R |
| `ShouldEvaluateAnimation` | True when the animation must be evaluated this step. | N |
| `RandomizeAnimations` | Randomises the start of the looping animations (name). | N |
| `GetMacroAnimLayer` | The layer of a macro animation (error `Invalid macro animation layer name provided`). | P |
| `CanPlayMacroAnim` | True when a macro animation may play. | N |
| `PlayMacroAnimations` | Plays the macro animations. | N |
| `PlaySound` | Plays a sound on a source. | N |
| `PlaySchemeSound` | Plays a sound of the character scheme. | P |
| `SaySound` | Plays speech. | N |
| `SaySchemeSound` | Plays a scheme speech. | P |
| `GetFreeSoundSource` | Finds a free sound source (`PSST_AVAILABLE`). | N |
| `StartMovingSound` | Starts the moving sound (scheme `Moving`). | P |
| `StartNotMovingSound` | Starts the not-moving sound (`NotMoving`). | P |
| `GetPuppetMovingSoundRatio` | How strongly the moving sound plays (base: 0). | N |
| `OnAnimEventProduceEffect` | Animation event: starts an effect. | P |
| `OnAnimEventProduceShakeEffect` | Animation event: shakes the camera. | P |
| `OnAnimEventProduceSound` | Animation event: plays a sound. | P |
| `OnAnimEventTerminateEffect` | Animation event: stops an effect. | P |
| `OnAnimEventTerminateSound` | Animation event: stops a sound. | P |

### Views, cameras, rendering and operator input

| Method | What it does | Basis |
|---|---|---|
| `GetViewOrigin` | The eye position of the puppet (for the operator). | N |
| `GetViewTarget` | The point the operator looks at. | N |
| `GetRelativeViewOrigin` | The eye position relative to the body (name). | N |
| `GetCameraPlacement` | The camera placement of the puppet. | N |
| `Get3rdPersonCameraPlacement` | The third-person camera placement. | N |
| `Provide3rdPersonCameraPos` | Computes the third-person camera with the wall probe. | N |
| `Provide3rdPersonCameraPosAsRide` | The same for a ride. | N |
| `OnToggle3rdPersonView` | Toggles the view (cheat). | N |
| `RendersIn3rdPerson` | True when the third-person view is on. | N |
| `GetProjectionMatrix` | The projection matrix of the view. | N |
| `GetFOVMultiplier` | The field-of-view multiplier (1). | R |
| `Render3D` | Renders the world from this puppet (error text `Infinite view bug detected`). | P |
| `RenderView` | Renders the whole view including the overlays. | P |
| `RenderOverlay` | Empty overlay hook. | R |
| `RenderMissionObjectives` | Draws the mission objectives. | P |
| `RenderNavigationalBeacons` | Draws the navigation beacons. | P |
| `GetBeaconsAndObjectivesFadeRatio` | The fade of those (name). | R |
| `InjectRenderingCommands` | Empty hook. | R |
| `ProcessOperatorInput` | Applies the operator's move, look and fire input (profile scope `camera_aim_ray`). | P |
| `GetOperatorMoveDir` | The operator's move direction. | N |
| `ClampLookDirEul` | Clamps the look direction. | P |
| `ClampLookDirEulAsRide` | The same as a ride. | P |
| `ClampLookDirEulForCamera` | The same for the camera. | N |
| `AdjustClientOrientation` | Hook that corrects the orientation on a client (empty in the base). | R |
| `ApplyClientPositionCorrection` | Smooths away the client position error. | N |

### Script (macro) functions

| Method | What it does | Basis |
|---|---|---|
| `macAllowRecharging` | Script: clears the no-recharging flag. | N |
| `macDisableAI` | Script: disables the brain (`CBrainEntity::DisableAI`). | P |
| `macEnableAI` | Script: enables it. | P |
| `macDisableMovingAbility` | Script: removes a movement ability by name (error `Invalid ability name`). | P |
| `macEnableMovingAbility` | Script: adds one. | P |
| `macDisableNonPlayerFoes` | Script: ignore non-player foes. | N |
| `macDisablePlayerFoes` | Script: ignore player foes. | N |
| `macDisableStopBeforePatrol` | Script: sets `PNF_DISABLE_STOP_BEFORE_PATROL`. | R |
| `macDrawTool` | Script: `DrawTool`. | P |
| `macFireButtonPressed` | Script: presses the fire button. | N |
| `macFireButtonReleased` | Script: releases it. | N |
| `macGetCharacterClass` | Script: the character class name. | N |
| `macGetFoe` | Script: the current foe. | N |
| `macGetHealth` | Script: the health. | R |
| `macGetLastThrownTool` | Script: the last thrown tool. | N |
| `macGetPrimaryAmmo` | Script: primary ammunition count. | R |
| `macGetSecondaryAmmo` | Script: secondary ammunition count. | R |
| `macGetThrownObject` | Script: the thrown object. | N |
| `macGetTimeFromLastDamage` | Script: time since the last damage. | N |
| `macHideAttachment` | Script: hides an attachment. | P |
| `macIsAlive` | Script: whether alive. | R |
| `macMarkAsMiniBoss` | Script: sets `PNF_MINI_BOSS`. | R |
| `macMountRide` | Script: mounts a ride. | P |
| `macNotifyAIEnabled` | Script: sends a script event when the AI is enabled. | R |
| `macNotifyIdle` | Script: sends a script event when idle. | R |
| `macPickTool` | Script: `PickTool`. | P |
| `macSetAlignment` | Script: sets the alignment by its `EBrainAligment` name. | P |
| `macSetHealth` | Script: sets the health. | N |
| `macSetLookTarget` | Script: sets the look target. | N |
| `macSetMaxAltitude` | Script: sets the flyers' maximum altitude. | N |
| `macSetPlacementEntity` | Script: sets the placement entity. | P |
| `macSetPrimaryAmmo` | Script: sets primary ammunition. | N |
| `macSetSecondaryAmmo` | Script: sets secondary ammunition. | N |
| `macSetThreatSensitivity` | Script: sets the threat sensitivity by name (`Dead horse`, `Damage only`, `Damage and sense`, `Short sighted`, `Standard`, `Long sighted`, `Hear through walls`, `See through walls`; error `Unknown threat sensitivity`). | P |
| `macShowAttachment` | Script: shows an attachment. | P |
| `macStopMoving` | Script: stops moving. | N |
| `macStoreTool` | Script: `StoreTool`. | P |
| `macTeleport` | Script: teleports to an entity (error `given entity ptr argument is NULL`). | P |
| `macThrowObject` | Script: `ThrowObject`. | R |
| `macThrowPickedTool` | Script: `ThrowPickedTool`. | R |
| `macThrowPickedToolAtTargetEntity` | Script: the same at an entity. | P |
| `macThrowTool` | Script: `ThrowTool`. | P |
| `macThrowToolAtTargetEntity` | Script: the same at an entity. | P |

### Senses, foes and brain links

| Method | What it does | Basis |
|---|---|---|
| `AddFlameEntity` | Attaches the flame entity of a burning puppet (name). | N |
| `GetFlameEntity` | Returns the flame entity (name). | N |
| `RemoveFlameEntity` | Removes the flame entity (name). | N |

### Health, armour, damage and invulnerability

| Method | What it does | Basis |
|---|---|---|
| `IsOnProteins` | Whether the puppet is under a protein power-up (base: no; name). | R |

## Invariants

1. The tables list 519 methods once each (519 of 519 named exports of `CPuppetEntity`).
2. The 44 methods whose name starts with `mac` are the script functions.

## Open questions

The methods with Basis `N` and `P` are described only as far as the basis allows; the behaviors that matter are parked as T01-139 to T01-150 in the overview (`puppet-classes.md`, Open questions): damage and hits (T01-140), movement (T01-141, T01-142), attacks (T01-143), tools, carrying and rides (T01-144), spawning (T01-145), run-time checks (T01-146).

## Test vectors

- Counts: 519 rows; by Basis 103 `R`, 181 `P`, 235 `N`.
- The 44 `mac` methods are in the last table.
- Example row: `GetNetBehavior` -> dynamic or static by the spawner handle (`R`).
