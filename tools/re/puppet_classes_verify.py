"""Evidence for docs/re/puppet-classes.md and its three companion pages (T01-103): what the data says about the puppet classes of Sam2Game.dll.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. the 101 exported classes of the family (names quoted from the export table of Sam2Game.dll): whether each is in the data (own name,
     the `S` twin of a `C` name, or the one misspelled alias), its base class, own member count, number of files and file extensions;
  2. the member-order check (the rule of docs/re/weapon-classes.md section 4, applied to the puppet classes): for every data class whose
     member table the binary also declares, the data's members in file order against the declared names: same count (after the stated
     omissions), every member the data names at the position of the same name, every number-only member given the declared name, and the
     data type against the Hungarian prefix of the name;
  3. the puppet parameter files: the root class of every `.ep` under Databases/Puppets and which placed Character entity class goes with it
     (counts of files only; the placed instances are counted by tools/re/vehicles_verify.py and tools/re/wld_entity_verify.py);
  4. what the install does NOT contain: any `DummyFoe` file, any entity of the eleven obsolete kinds, of the dummy foe or of the generic legged kind;
  5. the enumeration type names that the data uses for the puppet members.

Usage: python -I tools/re/puppet_classes_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; about 5 minutes)
"""
import collections, os, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import _natural, merged_files, parse, u32  # noqa: E402
import weapons_verify  # noqa: E402

# The 101 classes of the family, by role (exported names; 26 + 26 + 24 + 25).
ENTITY = [
    'CAirBikePuppetEntity', 'CAircraftCharacterEntity', 'CAircraftPuppetEntity', 'CAutoTurretPuppetEntity',
    'CBuggyPuppetEntity', 'CDummyFoePuppetEntity', 'CFlyingKleerPuppetEntity', 'CGenericLeggedPuppetEntity',
    'CHovercraftCharacterEntity', 'CHovercraftPuppetEntity', 'CKleerPuppetEntity', 'CLeggedCharacterEntity',
    'CLeggedPuppetEntity', 'CManifestedSpawnerPuppetEntity', 'CMilitaryCarPuppetEntity', 'CNPCPuppetEntity', 'CPuppetEntity',
    'CRollingBallCharacterEntity', 'CRollingBallPuppetEntity', 'CSciFiSoldierPuppetEntity', 'CTurretCharacterEntity',
    'CTurretPuppetEntity', 'CWereBullPuppetEntity', 'CWheeledVehicleCharacterEntity', 'CWheeledVehiclePuppetEntity',
    'CZombieSoldierPuppetEntity',
]
PROPERTIES = [
    'CAirBikePuppetProperties', 'CAircraftCharacterProperties', 'CAircraftPuppetProperties', 'CAutoTurretPuppetProperties',
    'CBuggyPuppetProperties', 'CDummyFoePuppetProperties', 'CFlyingKleerPuppetProperties', 'CGenericLeggedPuppetProperties',
    'CHovercraftCharacterProperties', 'CHovercraftPuppetProperties', 'CKleerPuppetProperties', 'CLeggedCharacterProperties',
    'CLeggedPuppetProperties', 'CManifestedSpawnerPuppetProperties', 'CMilitaryCarPuppetProperties', 'CNPCPuppetProperties',
    'CPuppetProperties', 'CRollingBallCharacterProperties', 'CRollingBallPuppetProperties', 'CSciFiSoldierPuppetProperties',
    'CTurretCharacterProperties', 'CTurretPuppetProperties', 'CWereBullPuppetProperties', 'CWheeledVehicleCharacterProperties',
    'CWheeledVehiclePuppetProperties', 'CZombieSoldierPuppetProperties',
]
PARAMS = [
    'CAirBikePuppetParams', 'CAircraftPuppetParams', 'CAutoTurretPuppetParams', 'CBuggyPuppetParams', 'CDummyFoePuppetParams',
    'CFlyingKleerPuppetParams', 'CGenericLeggedPuppetParams', 'CHovercraftPuppetParams', 'CKleerPuppetParams',
    'CLeggedPuppetParams', 'CManifestedSpawnerPuppetParams', 'CMilitaryCarPuppetParams', 'CNPCBomberPuppetParams',
    'CNPCCivilianPuppetParams', 'CNPCKamikazePuppetParams', 'CNPCPuppetParams', 'CNPCShooterPuppetParams', 'CPuppetParams',
    'CRollingBallPuppetParams', 'CSciFiSoldierPuppetParams', 'CTurretPuppetParams', 'CWereBullPuppetParams',
    'CWheeledVehiclePuppetParams', 'CZombieSoldierPuppetParams',
]
RECORD = [
    'CAllMovingAnimation', 'CBodyMovingAnimation', 'CCorpseSinkConfig', 'CDamageMultipliers', 'CKickingAbilities',
    'CLeggedPuppetBodyParams', 'CLerpToAnimAdjuster', 'CMiscPuppetParams', 'CMoveCharacterData', 'CMovingAbilities',
    'CMovingAnimData', 'CMovingAnimationScheme', 'CPoseTransitionAnimation', 'CPuppetBodyParams', 'CPuppetDeathParams',
    'CPuppetEffectParams', 'CPuppetHealthParams', 'CPuppetSeatData', 'CPuppetSoundRanges', 'CRideAndRiderAnim', 'CRideAndSeat',
    'CShieldDescription', 'CSpineAdjuster', 'CWayToSeat', 'CWheelData',
]

BIN = {
    "SPuppetEntity": (
        'en_bTouchedFloor', 'en_bLastTouchedFloor', 'en_bTouchedFloorOnce', 'en_henTouchedWall', 'en_vWallContact',
        'en_iFeelerSteer', 'en_fFeelerH', 'en_fFeelerOrgH', 'en_vFeelerOrgPos', 'en_fFeelerY', 'en_fFeelerLen',
        'en_fFeelerTimer', 'en_vFeelerMovedPos', 'en_idPuppet', 'en_pmcInstanceModel', 'en_henBrain', 'en_henDefaultBrain',
        'en_henSpawner', 'en_henSpawnerWhoSpawnedMe', 'en_henSpawnerThatUsesUs', 'en_vDesiredTempoAbs',
        'en_vLastDesiredTempoAbs', 'en_vDesiredLookDirEul', 'en_fPitchAddedToDesiredLookDirEul', 'en_henCurrentPathMarker',
        'en_henLastReachedMarker', 'en_bRotateOnLastPatrolingMarker', 'en_psState', 'en_slHealthMax', 'en_slArmorMax',
        'en_bLookAhead', 'en_bLookAheadAllowed', 'en_bBrakingToStopOnGP', 'en_bRotatingOnGP', 'en_tmStartedBraking',
        'en_vGoalPointRotationEul', 'en_bUserForcedDesiredLookDir', 'en_bForceDesiredLookDir', 'en_vForcedDesiredLookDirEul',
        'en_bMultiSpawnerTemplate', 'en_maLastMoveAbilities', 'en_maMoveAbilitiesTemp', 'en_hmech', 'en_hbo', 'en_hjo',
        'en_hre', 'en_idLastSetMechanism', 'en_idLastSetCollisionCategory', 'en_ulLastSetMechanismFlags',
        'en_fLastJointVelocity', 'en_fPatrolThrustAddingAngle', 'en_maPatrolingMoveAbilities', 'en_ulAutoPatrolMarkerEntityID',
        'en_bAutoPatrolMarkers', 'en_tmLastNonIdle', 'en_tsDefaultThreatSensitivity', 'en_hpaehAnimEventHandler',
        'en_hssMouth', 'en_hssMoving', 'en_hssAlive', 'en_hfxAlive', 'en_hss0', 'en_hss1', 'en_hss2', 'en_hss3',
        'en_iLastSoundSrc', 'en_tmBorn', 'en_tmDeath', 'en_csoSoundAnimEventsToTerminate', 'en_csoEffectAnimEventsToTerminate',
        'en_hmhMomentHandler', 'en_idGenericAnimToPlay', 'en_idLastPlayedGenericAnimation', 'en_idCurrentAttackID',
        'en_idAttackToStart', 'en_idCurrentBlast', 'en_ctConsecutiveShootingsToFire', 'en_iCurrentConsecutiveShooting',
        'en_fRotateTowardsFoeTime', 'en_henShooter', 'en_bAttackDamageApplied', 'en_idCGSPlayedAnim', 'en_fTemp', 'en_vTemp',
        'en_pbaLocalMove', 'en_eaRiders', 'en_henCurrentClimbingRide', 'en_idCurrentSeat', 'en_iCurrentWayToSeat',
        'en_tmLastDamage', 'en_slLastDamage', 'en_slAccuDamage', 'en_vAccuForceImpulse', 'en_slAccuHeal', 'en_bHealing',
        'en_tmLastHeal', 'en_tmLastPuppetKicked', 'en_tmLastNonPuppetKicked', 'en_vLastDamageDirectionAbs',
        'en_fAllowedMovingRange', 'en_bAllowMovingForward', 'en_tmForcingBackward', 'en_tmForcingForward', 'en_tmSlowingDown',
        'en_fMinSteerR', 'en_tmMaxSteeringStarted', 'en_tmMinSteerRLastLerped', 'en_tmLastSteeringRadiusAdjust',
        'en_bSlowingDownToReachGP', 'en_bMovingForward', 'en_bMeleeHitFoe', 'en_fInstanceShortSightDistance',
        'en_fInstanceSenseRange', 'en_tsCurrentThreatSensitivity', 'en_hcpInstanceMounts', 'en_bMountee', 'en_eaDefaultRiders',
        'en_idDefaultState', 'en_bLastFrameMoving', 'en_ulPuppetExecutionAbilities', 'en_tmLastTimeFlockingUpdated',
        'en_eaNearPuppets', 'en_fFlockingCacheTime', 'en_fLastRotationAngle', 'en_fCustomFallHeightAllowed',
        'en_fFallHeightAllowed', 'en_fFlockingRange', 'en_fInstanceFlockingCacheTime', 'en_fInstanceFlockingRange',
        'en_fInstanceStepUp', 'en_fInstanceStepDown', 'en_fStepDownHeight', 'en_fStepUpHeight', 'en_hcc3rdPersonRideCamera',
        'en_hcc3rdPersonCamera', 'en_hcc3rdPersonDeathCamera', 'en_tmLastBlastAnim', 'en_iLastFiredBarrel',
        'en_henDefaultTacticEntity', 'en_henCurrentTacticEntity', 'en_henDesiredTacticEntity', 'en_ubLastSeenFireButtonStates',
        'en_tmAttackDesired', 'en_tmAttackStarted', 'en_bCanAttackOwnAligment', 'en_hjoCorrection', 'en_plfLocalFlags',
        'en_hboCarrying', 'en_hmechCarried', 'en_hjoCarrying', 'en_tmFire0Pressed', 'en_henLastDamager',
        'en_bDisableThreatSoundCheck', 'en_henDeathSpawner', 'en_henOperator', 'en_fLast3rdPersonCameraHitDistance',
        'en_tmLastCameraRayCast', 'en_f3rdCameraBehindRatio', 'en_vOperatorAimPoint', 'en_vClientPositionError',
        'en_fClientHeadingError', 'en_boxCachedMechBoundingBox', 'en_fDesiredIdlePeriod', 'en_henFixer',
        'en_ctSpawnSourceRequests', 'en_aactAttachedTools', 'en_adctDesiredCharacterTools', 'en_adctStoredToolsBeforeRiding',
        'en_atarToolAnimRelations', 'en_henLastThrownToolEntity', 'en_fAnimLerpTime', 'en_bTouchedAnything', 'en_vFloorNormal',
        'en_hboFloor', 'en_pmaMaterialTheirs', 'en_tmLastDamageAppliedByMaterial', 'en_tmLastFloorTouch', 'en_amtMountedTools',
        'en_fFOVInstanceOverride', 'en_bDisableNavigation', 'en_bDisableFeelers', 'en_bDisableDeblocking',
        'en_fMovingSoundVolume', 'en_pppPuppetParams', 'en_pcbCharacterBehavior', 'en_baAlignment', 'en_slHealth',
        'en_slArmor', 'en_bShouldRotateOnGP', 'en_bPassThroughGoalPoint', 'en_bGoalPointReached', 'en_vPosWhenGPIsSet',
        'en_henLookTarget', 'en_vGoalPoint', 'en_vLastReachedGoalPoint', 'en_mtMoveType', 'en_bAlive', 'en_maMoveAbilities',
        'en_qvOriginalPlacement', 'en_idCurrentMechanism', 'en_vNetPosition', 'en_vNetMechVelocity', 'en_fNetHeading',
        'en_fSpeedMultiplier', 'en_fMoverMultiplier', 'en_fDecelMultiplier', 'en_fAccelMultiplier',
        'en_fTurningVelocityMultiplier', 'en_fThrustAddingAngleOverride', 'en_fPatrolSpeedMultiplier',
        'en_fPatrolRotateSpeedMultiplier', 'en_fRnd', 'en_ava', 'en_henRide', 'en_rsRidingState', 'en_idSeat', 'en_iWayToSeat',
        'en_fStretch', 'en_bAvatarViewMode', 'en_bcBarrierCollision', 'en_pnfNetFlags', 'en_henCarrying',
        'en_iCarriedPartIndex', 'en_qvGrabRel', 'en_pctFirstDesiredTool', 'en_henPickedEntityOfFirstDesiredTool',
        'en_bSetFirstDesiredToolImmediatelly', 'en_pctSecondDesiredTool', 'en_henPickedEntityOfSecondDesiredTool',
        'en_bSetSecondDesiredToolImmediatelly', 'en_henToolEntityToPick', 'en_ctAmmoPrimary', 'en_ctAmmoSecondary',
        'en_fMaxAltitude', 'en_henPlacementEntity', 'en_dmtDeathDamageType', 'en_qvDeathPlacement',
        'en_fMaxDotTouchNormalAndRelVelocity', 'en_hhuStrongestTouch', 'en_vStrongestTouchNormal',
        'en_bDisableFallingDamageOnNextTouch', 'en_vAccuDamage', 'en_henFlame', 'en_tmSpawned', 'en_strName',),
    "SPuppetProperties": (
        'ep_idPuppetObsolete', 'ep_pppPuppetParams', 'ep_idBehaviorObsolete', 'ep_pcbCharacterBehavior', 'ep_idDefaultState',
        'ep_pmcInstanceModel', 'ep_hspSpawner', 'ep_bMultiSpawnerTemplate', 'ep_hepAutoPatrolMarker', 'ep_bAutoPatrolMarkers',
        'ep_ulAutoPatrolMarkerEntityID', 'ep_bIsSpawned', 'ep_tsDefaultThreatSensitivity', 'ep_hmhMomentHandler',
        'ep_fStretch', 'ep_fAllowedMovingRange', 'ep_fPatrolSpeedMultiplier', 'ep_fPatrolRotateSpeedMultiplier',
        'ep_fPatrolThrustAddingAngle', 'ep_maPatrolMovingAbilities', 'ep_fFOVInstanceOverride',
        'ep_fInstanceShortSightDistance', 'ep_fInstanceSenseRange', 'ep_fFallHeightAllowed', 'ep_fFlockingRange',
        'ep_fFlockingCacheTime', 'ep_fStepUpOverride', 'ep_fStepDownOverride', 'ep_hcpInstanceMounts', 'ep_bcBarrierCollision',
        'ep_hepTacticEntity', 'ep_henTacticEntity', 'ep_bCanAttackOwnAligment', 'ep_bDisableThreatSoundCheck',
        'ep_bDisableNavigation', 'ep_bDisableFeelers', 'ep_bDisableDeblocking', 'ep_hepDeathSpawner', 'ep_henDeathSpawner',
        'ep_amtMountedTools',),
    "SPuppetParams": (
        'pp_phpHealth', 'pp_pbpBody', 'pp_maMovingAbilities', 'pp_pfxEffectParams', 'pp_pppProjectileParams', 'pp_csoSeats',
        'pp_opOperatingParams', 'pp_omObjectManipulation', 'pp_kaKickingAbilities', 'pp_pdpDeath', 'pp_mppMiscPuppetParams',
        'pp_cCameras', 'pp_paPuppetAttack', 'pp_atdThrowDescs', 'pp_hcmGenericMounts', 'pp_faaFoeAssigning',
        'pp_amtMountedTools', 'pp_aiShownAttachments',),
    "SLeggedPuppetParams": (
        'lpp_pepLand', 'lpp_pepAir', 'lpp_pepWater', 'lpp_lpbBody', 'lpp_kopKnockOutParams',),
    "SLeggedPuppetEntity": (
        'en_ppPuppetPose', 'en_fBodyEulH', 'en_vFrontEul', 'en_vRelLookEul', 'en_bForcingDesiredFront',
        'en_vForcedDesiredFrontEul', 'en_bAddingThrust', 'en_mdMoveDirection', 'en_fOldDesTempoToDesLook',
        'en_iOldMoveDirection', 'en_hsaSpineAdjuster', 'en_hltaLerpToAnim', 'en_ppOldPuppetPose', 'en_bJumpButtonReleased',
        'en_bInTheAirAfterJump', 'en_fFluidRatio', 'en_fFluidDensity', 'en_tmDived', 'en_fLastFluidRatio', 'en_mapDesiredPose',
        'en_marDesiredReadiness', 'en_tmLastJump', 'en_bUsesAirControl', 'en_tmLastAirControlDesired', 'en_fAirControlTime',
        'en_fAirControlFactor', 'en_fAirControlVelocityMultiplier', 'en_bAutoAnimations', 'en_madLast', 'en_mad',
        'en_fLastTestedMoveSpeed', 'en_idPoseChangeAllAnim', 'en_idPoseChangeBodyAnim', 'en_idAllAnim', 'en_idBodyAnim',
        'en_bLastReverse', 'en_fLastSeenNominalVelocity', 'en_fLastSeenMoveSpeed', 'en_tmPoseChangeEnd',
        'en_rrsRideAndRiderSequence', 'en_tmStartEnteringIntoRide', 'en_tmStartExitingFromRide',
        'en_qvStartEnteringIntoRideLook', 'en_qvStartExitingFromRideLook', 'en_bLerpRiderPlacement', 'en_tmRiderAnimStart',
        'en_qvRiderStart', 'en_fRiderAnimLength', 'en_cssShieldStatus', 'en_ceCharacterEmotion', 'en_tmResetShieldState',
        'en_tmResetEmotionState', 'en_tmLastKO', 'en_bAddKOForceImpulse', 'en_bLastPlayedMoveAnim', 'en_lpsState',
        'en_iCurrRideAndRiderAnimPair', 'en_bGenLeapShouldRunAway', 'en_bGenLeapGroundTouched', 'en_tmLeapAttackStarted',
        'en_tmRotateTowardsFoeStart',),
    "SGenericLeggedPuppetProperties": (
        'ep_pmcModel', 'ep_vStretch', 'ep_glppPuppetParamsObsolete',),
    "SGenericLeggedPuppetEntity": (
        'en_pmcModel',),
    "SDummyFoePuppetProperties": (
        'ep_baAligment', 'ep_bFlying', 'ep_fVelocity', 'ep_slHealth',),
    "SDummyFoePuppetEntity": (
        'en_bImmortal', 'en_bFlying', 'en_fVelocity', 'en_slCustomHealth',),
    "STurretPuppetParams": (
        'tpp_fRotationSpeed', 'tpp_fGunAcceleration', 'tpp_fGunDecceleration',),
    "STurretPuppetEntity": (
        'tpe_fUpJointLimitEul', 'tpe_fDnJointLimitEul', 'tpe_henParent', 'tpe_qvRelToParent', 'en_bLastRotating',),
    "SAircraftPuppetParams": (
        'app_apcPlayerControls', 'app_bFrontDirInXZPlane', 'app_bSlowDownWhenTurning', 'app_bLeanPWhenMoving', 'app_fMaxLeanP',
        'app_bLeanBWhenMoving', 'app_fMaxLeanB', 'app_fMaxFrontVel', 'app_fFrontAcceleration', 'app_fFrontDeceleration',
        'app_fMaxUpVel', 'app_fUpAcceleration', 'app_fUpDeceleration', 'app_fMaxRightVel', 'app_fRightAcceleration',
        'app_fRightDeceleration', 'app_fAngAccelH', 'app_fAngAccelP', 'app_fAngAccelB', 'app_fAngVelH', 'app_fAngVelP',
        'app_fAngVelB', 'app_fAirResistanceForLeaningP', 'app_fAirResistanceForLeaningB',),
    "SAircraftPuppetEntity": (
        'en_vVelocityLast', 'en_vLastDesiredVelocity',),
    "SHovercraftPuppetParams": (
        'hpp_fAngAccelH', 'hpp_fAngAccelB', 'hpp_fMaxHVelocity', 'hpp_fMaxBVelocity', 'hpp_fMaxLeanBanking',
        'hpp_fFrontAcceleration', 'hpp_fFrontDecceleration', 'hpp_fMaxVelForward', 'hpp_fMaxVelBackward',
        'hpp_fSideAcceleration', 'hpp_fSideDecceleration', 'hpp_fMaxVelSide', 'hpp_fMinJumpingPeriod',
        'hpp_fMaxValidSlopeForJump', 'hpp_fMinJumpVelocity', 'hpp_fMaxJumpVelocity', 'hpp_fSpringCoef', 'hpp_fDampingCoef',),
    "SHovercraftPuppetEntity": (
        'en_vVelocityLast', 'en_tmLastJump', 'en_fLastError',),
    "SRollingBallPuppetParams": (
        'rbpp_fMaxLinearVelocity', 'rbpp_fLinearAccel', 'rbpp_fMinJumpVelocity', 'rbpp_fMaxJumpVelocity',
        'rbpp_fMaxValidSlopeForJump', 'rbpp_fSeatRotationVelocity', 'rbpp_slAccumulatedImpulseNonBraking',
        'rbpp_fNonBrakingPeriodAfterImpulse', 'rbpp_bDiscardVelocityUponImpulse',),
    "SRollingBallPuppetEntity": (
        'en_fRadius', 'en_tmLastJump', 'en_hboSeat', 'en_fDesiredSeatHeadingEul', 'en_tmLastForceImpulse',
        'en_fDesiredVelocity',),
    "SWheeledVehiclePuppetParams": (
        'wwpp_fMaxSpeed', 'wwpp_fVehicleAcceleration', 'wwpp_fVehicleDecceleration', 'wwpp_fWheelSteeringTorque',
        'wwpp_fWheelSteeringSpeed',),
    "SWheeledVehiclePuppetEntity": (
        'en_awdWheelData', 'en_ctWheels', 'en_ctDrivingWheels',),
    "SPuppetSeatData": (
        'psd_idSeatName', 'psd_idSeatAttachment', 'psd_idOperatedFromSeat', 'psd_sfSeatFlags', 'psd_idRiderMechanism',
        'psd_idRiderAnimRideNotMoving', 'psd_idRiderAnimRideMoving', 'psd_idApproachFromSeat', 'psd_idRidePartToAttachTo',
        'psd_bRiderCanTurnHead', 'psd_csoWaysToSeat',),
    "SRideAndRiderAnim": (
        'rra_idRiderAnim', 'rra_idRideAnim', 'rra_bLerpRiderPlacement', 'rra_bPlayReversed',),
    "SWayToSeat": (
        'wts_idEnterPointAttachment', 'wts_csoEnteringAnimSequence', 'wts_csoExitingAnimSequence',),
}

FAMILY = ENTITY + PROPERTIES + PARAMS + RECORD
ALIASES = {"CMiscPuppetParams": "CMiscPupeptParams"}  # the data spells this class with a typo
ELEVEN = ["CAirBikePuppetEntity", "CAutoTurretPuppetEntity", "CBuggyPuppetEntity", "CFlyingKleerPuppetEntity", "CKleerPuppetEntity",
          "CManifestedSpawnerPuppetEntity", "CMilitaryCarPuppetEntity", "CNPCPuppetEntity", "CSciFiSoldierPuppetEntity",
          "CWereBullPuppetEntity", "CZombieSoldierPuppetEntity"]
CHARACTERS = {"CLeggedCharacterEntity": "SLeggedPuppetParams", "CAircraftCharacterEntity": "SAircraftPuppetParams",
              "CTurretCharacterEntity": "STurretPuppetParams", "CHovercraftCharacterEntity": "SHovercraftPuppetParams",
              "CRollingBallCharacterEntity": "SRollingBallPuppetParams", "CWheeledVehicleCharacterEntity": "SWheeledVehiclePuppetParams"}

# Members of the declared tables that the data does not store (the other differences are printed as they are found).
OMIT = {"SPuppetEntity": {"en_hcc3rdPersonRideCamera", "en_hcc3rdPersonCamera", "en_hcc3rdPersonDeathCamera"}}

# Hungarian prefix of a declared member name -> acceptable data type names (a prefix with no entry is not checked).
PRIMITIVE = {"BOOL", "FLOAT", "SLONG", "INDEX", "IDENT", "ULONG", "UBYTE", "SBYTE", "Time", "Vector3f", "Vector2f", "QuatVect", "CString"}
FITS = {"b": ("BOOL",), "f": ("FLOAT",), "sl": ("SLONG",), "tm": ("Time",), "id": ("IDENT",), "ct": ("INDEX", "SLONG"), "i": ("INDEX", "SLONG"),
        "ul": ("ULONG",), "ub": ("UBYTE",), "sb": ("SBYTE",), "v": ("Vector3f", "Vector2f"), "qv": ("QuatVect",), "str": ("CString",)}


def prefix(name):
    """The Hungarian prefix of `<class>_<prefix><Name>` (the lower-case letters after the underscore)."""
    tail = name.split("_", 1)[1]
    i = 0
    while i < len(tail) and tail[i].islower(): i += 1
    return tail[:i]


def twin(name):
    return [name] + ([ALIASES[name]] if name in ALIASES else []) + (["S" + name[1:]] if name[0] == "C" else [])


def collect(install, names):
    """One pass over every format-8 file: for each wanted class name the variants (base, members with type names), files, extensions."""
    merged = {}
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if i.is_dir() or i.file_size < 64: continue
                with z.open(i) as fh: head = fh.read(16)
                if head[:8] == b"CTSEMETA" and u32(head, 12) == 8: merged[i.filename.lower()] = (f, i.filename)
    by_arch = collections.defaultdict(list)
    for f, name in merged.values(): by_arch[f].append(name)
    out = {}; nfiles = 0
    for f, ns in by_arch.items():
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for name in ns:
                r = parse(z.read(name)); nfiles += 1; ext = os.path.splitext(name)[1].lower()
                for t in r["types"].values():
                    if t.members is None or t.name not in names: continue
                    parent = r["types"][t.parent].name if t.parent in r["types"] else None
                    mem = tuple((m, r["types"][mt].name if mt in r["types"] else "?") for m, mt in t.members)
                    rec = out.setdefault(t.name, {"variants": collections.Counter(), "files": 0, "exts": collections.Counter()})
                    rec["variants"][(parent, mem)] += 1; rec["files"] += 1; rec["exts"][ext] += 1
    return out, nfiles


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    wanted = {n for c in FAMILY for n in twin(c)} | set(BIN) | {"SPuppetEntity", "SPuppetProperties", "SPuppetParams", "SPlayerPuppetParams"}
    cat, nfiles = collect(install, wanted)
    print(f"type tables of {nfiles} format-8 files")

    print(f"\n1. the {len(FAMILY)} exported classes of the family ({len(ENTITY)} entity, {len(PROPERTIES)} properties, {len(PARAMS)} parameter, "
          f"{len(RECORD)} record and helper classes) and the data")
    indata = collections.Counter(); total = collections.Counter(); alias_used = []
    for role, lst in (("entity", ENTITY), ("properties", PROPERTIES), ("params", PARAMS), ("record", RECORD)):
        for c in lst:
            total[role] += 1
            found = [n for n in twin(c) if n in cat]
            if not found:
                print(f"   {role:10s} {c:36s} not in the data"); continue
            indata[role] += 1; n = found[0]; rec = cat[n]
            if n in ALIASES.values(): alias_used.append((c, n))
            (parent, mem), _ = rec["variants"].most_common(1)[0]
            exts = ",".join(f"{e.lstrip('.')}:{k}" for e, k in rec["exts"].most_common(3))
            print(f"   {role:10s} {c:36s} saved as {n:34s} base {str(parent):26s} {len(mem):3d} members, {rec['files']:4d} files ({exts}), {len(rec['variants'])} variant(s)")
    by_twin = sum(1 for c in FAMILY if any(n in cat for n in [c] + (["S" + c[1:]] if c[0] == "C" else [])))
    print(f"   in the data: {sum(indata.values())} of {len(FAMILY)} ({by_twin} by the own-name / `S`-twin rule + {len(alias_used)} saved under a misspelled name {alias_used}); "
          f"by role {dict(indata)} of {dict(total)}")
    print(f"   binary only: {len(FAMILY) - sum(indata.values())}; entity classes of the 11 obsolete kinds in the data: "
          f"{[c for c in ELEVEN if any(n in cat for n in twin(c))] or 'none'}; of the dummy foe and the generic legged kind: "
          f"{[c for c in ('CDummyFoePuppetEntity', 'CGenericLeggedPuppetEntity') if any(n in cat for n in twin(c))] or 'none'}")

    print("\n2. member order: the data's members against the declared names")
    ok_all = 0; checked = 0
    for cls, names in BIN.items():
        names = [n for n in names if n not in OMIT.get(cls, ())]
        if cls not in cat: print(f"   {cls}: declared by the binary ({len(names)} members), not in the data"); continue
        checked += 1; first = True
        for (parent, mem), count in cat[cls]["variants"].most_common():
            dn = [m for m, _ in mem]
            nnum = sum(1 for m in dn if m.isdigit())
            same_count = len(dn) == len(names)
            diff = [(i, m, names[i]) for i, m in enumerate(dn) if not m.isdigit() and (i >= len(names) or m != names[i])]
            named_ok = not diff
            bad = []
            for i, (m, t) in enumerate(mem):
                if i >= len(names): break
                fit = FITS.get(prefix(names[i]))
                if fit and t in PRIMITIVE and t not in fit: bad.append((names[i], t))
            print(f"   {cls:28s} data {len(dn):3d} members ({nnum:2d} number-only) in {count:3d} file(s), declared {len(names):3d}; same count: {same_count}; "
                  f"named members that differ from the declared name at the same position: {len(diff)} {diff if diff else ''}; "
                  f"declared prefix vs data type mismatches: {len(bad)} {bad[:4] if bad else ''}")
            if first and len(diff) <= 3 and same_count: ok_all += 1
            first = False
        (parent, mem), _ = cat[cls]["variants"].most_common(1)[0]
        mapping = {m: names[i] for i, (m, _) in enumerate(mem) if m.isdigit() and i < len(names)}
        if mapping: print(f"      number -> name: {mapping}")
    print(f"   classes checked: {checked}; with the same count and at most three differently spelled named members (most common variant): {ok_all}")

    print("\n3. the puppet parameter files (Databases/Puppets, all `.ep`)")
    eps = merged_files(install, ".ep"); roots = collections.Counter(); nfile = 0; odd = []
    for name, d in eps.items():
        if "databases/puppets/" not in name.lower().replace("\\", "/"): continue
        nfile += 1
        r, objs, ex, rt = weapons_verify.load(d); roots[rt.type] += 1
        if rt.type in ("SGenericLeggedPuppetParams", "SPlayerPuppetParams"): odd.append((rt.type, name.rsplit("/", 1)[-1]))
    print(f"   {nfile} files; root classes: {dict(roots.most_common())}")
    for ch, par in CHARACTERS.items():
        e = [n for n in twin(ch) if n in cat]
        print(f"   {ch:34s} in the data as {e[0] if e else '-':32s} {cat[e[0]]['files'] if e else 0:3d} file(s); parameter class {par}: {roots.get(par, 0)} file(s)")
    print(f"   files with a generic legged or player root class: {sorted(odd)} (the entity class CGenericLeggedPuppetEntity is binary-only)")

    print("\n4. files named for the dummy foe, and every entry under Puppets/Objects")
    n = 0; hits = []
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                n += 1; low = i.filename.lower()
                if "dummyfoe" in low or "/puppets/objects/" in low: hits.append(i.filename)
    print(f"   {n} archive entries; matches: {hits or 'none'}")

    print("\n5. enumeration type names used by the members of the puppet data classes")
    tn = collections.Counter()
    for cls in ("SPuppetEntity", "SLeggedPuppetEntity", "SPuppetProperties", "SPuppetParams", "SLeggedPuppetParams", "SPuppetSeatData"):
        if cls in cat:
            (parent, mem), _ = cat[cls]["variants"].most_common(1)[0]
            for m, t in mem:
                if (t.startswith("E") and t[1:2].isupper()) or t == "EnemySize": tn[t] += 1
    print(f"   {dict(tn.most_common())}")


if __name__ == "__main__":
    main()
