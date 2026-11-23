# Console variables seen in the shipped modules (generated)

Names only, from three sources: the decorated names that the modules export, the plain name strings inside `Engine.dll`, `Sam2Game.dll`, `ProcRender.dll` and `Shaders.dll`, and the start-up scripts of `Content/SeriousSam2/Config/`. The prefix is the family (`gfx_`, `sam_`, …); the next letters are the Hungarian type tag the authors use (`b` boolean, `i` integer, `f` float, `str` string, `sl` signed long, `ul` unsigned long, `pix` pixel count, `ct` count, `tm` time). **Default** is the value written in `SafeMode.cfg` (a safe-mode profile, not necessarily the shipped default) when that file sets it. A name listed here is *known to exist*; that it is a user-settable console variable (and not an internal global) is certain only for names set by a start-up script or the help, and likely for the rest (same naming, same registration). The list is not claimed to be complete.


## `bmk_` (3)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `bmk_bAutoQuit` | bool | Engine, Sam2Game |  |
| `bmk_bBenchmarkDemos` | bool | Engine, Sam2Game |  |
| `bmk_bOnScreenReport` | bool | Engine |  |

## `ccn_` (5)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `ccn_fChatFontScale` | float | Sam2Game |  |
| `ccn_fHeightRatio` | float | Sam2Game |  |
| `ccn_fLineFadeTime` | float | Sam2Game |  |
| `ccn_fLineLifeTime` | float | Sam2Game |  |
| `ccn_iLastLines` | int | Sam2Game |  |

## `cdc_` (1)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `cdc_slCurrentAddress` | long | Engine, Sam2Game |  |

## `cli_` (5)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `cli_iBPSQualityTreshold` | int | Engine |  |
| `cli_iHighQualityBPS` | int | Engine |  |
| `cli_iLowQualityBPS` | int | Engine |  |
| `cli_iMaxBPS` | int | Engine, Sam2Game |  |
| `cli_iMaxOutBPS` | int | Engine, Sam2Game |  |

## `con_` (4)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `con_fHeightRatio` | float | (cfg), Sam2Game | `0.25` [Sam2Game] |
| `con_fLineFadeTime` | float | (cfg), Sam2Game | `2` [Sam2Game] |
| `con_fLineLifeTime` | float | (cfg), Sam2Game | `5` [Sam2Game] |
| `con_iLastLines` | int | (cfg), Sam2Game | `10` [Sam2Game] |

## `cor_` (1)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `cor_iEmulatePlatform` | int | Core, Engine, Sam2Game |  |

## `crm_` (10)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `crm_bAllowRendering` | bool | (cfg), ProcRender, Sam2Game | `0` [ProcRender] |
| `crm_bFrozenViewer` | bool | ProcRender |  |
| `crm_bPreviewTileBoxes` | bool | ProcRender |  |
| `crm_bTrashCaches` | bool | ProcRender |  |
| `crm_fDensityBias` | float | (cfg), ProcRender, Sam2Game | `1` [ProcRender] |
| `crm_fDistanceBias` | float | (cfg), ProcRender, Sam2Game | `1` [ProcRender] |
| `crm_fMaxCachingTime` | float | ProcRender |  |
| `crm_fStretch` | float | ProcRender |  |
| `crm_iCachingMethod` | int | ProcRender |  |
| `crm_iVertexCachingThreshold` | int | (cfg), ProcRender | `64` [ProcRender] |

## `d3d_` (11)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `d3d_bForcePixelProgramPartialPrecision` | bool | (cfg), Engine, GfxD3D, Shaders | `0` [Engine] |
| `d3d_bForcePixelProgramVersion2` | bool | (cfg), Engine, Sam2Game | `0` [Engine] |
| `d3d_bNoHardwareVertexProcessing` | bool | Engine, GfxD3D |  |
| `d3d_bReferenceRasterizer` | bool | Engine, GfxD3D |  |
| `d3d_ctCommonVertexBuffers` | count | (cfg), Engine, GfxD3D | `4` [Engine] |
| `d3d_iCommonVertexBuffersSize` | int | (cfg), Engine, GfxD3D | `1024` [Engine] |
| `d3d_iMaxAllowedPixelProgramVersion` | int | (cfg), Engine, Sam2Game | `11` [Engine] |
| `d3d_iRenderingDevice` | int | (cfg), Engine, GfxD3D | `0` [Engine] |
| `d3d_iShaderFlowControl` | int | Engine, GfxD3D, Shaders |  |
| `d3d_strProfilePS` | string | Engine, GfxD3D |  |
| `d3d_strProfileVS` | string | Engine, GfxD3D |  |

## `dbg_` (10)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `dbg_bAtHome` | bool | Engine |  |
| `dbg_bCallDebugFuncs` | bool | Engine |  |
| `dbg_bDumpStackOnError` | bool | Engine |  |
| `dbg_bLockStealthMode` | bool | Core, Engine |  |
| `dbg_bLogTransitionStack` | bool | Engine |  |
| `dbg_bShowFreeMemory` | bool | Engine |  |
| `dbg_bShowMemConsumption` | bool | Engine |  |
| `dbg_bShowReferences` | bool | Engine |  |
| `dbg_bStealthMode` | bool | Core, Engine |  |
| `dbg_strMarkedDataTypes` | string | Engine |  |

## `drn_` (5)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `drn_ctDepthBufferFaces` | count | Engine |  |
| `drn_mDepthProjection` | matrix | Engine |  |
| `drn_mDepthView` | matrix | Engine |  |
| `drn_pixDepthBufferHeight` | pixels | Engine |  |
| `drn_pixDepthBufferWidth` | pixels | Engine |  |

## `dsd_` (3)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `dsd_bAllowEmulation` | bool | (cfg), Engine, SfxDSD | `1` [Engine] |
| `dsd_iDefaultAdapter` | int | (cfg), Engine, SfxDSD | `0` [Engine] |
| `dsd_iSoftwareProcessing` | int | Engine, Sam2Game, SfxDSD |  |

## `efx_` (7)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `efx_bBloomPreview` | bool | ProcRender |  |
| `efx_bFog` | bool | Engine |  |
| `efx_bParticles` | bool | ProcRender |  |
| `efx_fBloomBias` | float | (cfg), ProcRender, Sam2Game | `0` [ProcRender] |
| `efx_fParticleBias` | float | (cfg), ProcRender, Sam2Game | `1` [ProcRender] |
| `efx_iBloomBlurDirection` | int | ProcRender |  |
| `efx_iLensFlareQuality` | int | (cfg), Engine, Sam2Game | `3` [Engine] |

## `fil_` (5)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `fil_bPreferZips` | bool | Core, Engine |  |
| `fil_bReportFileOpen` | bool | Core, Engine |  |
| `fil_iReadFailRange` | int | Core, Engine |  |
| `fil_iReportStats` | int | Core, Engine |  |
| `fil_iWriteCacheSize` | int | Core, Engine |  |

## `gfx_` (63)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `gfx_bDisableMultiMonSupport` | bool | Engine, GfxOGL |  |
| `gfx_bFullScreen` | bool | (cfg), Engine, Sam2Game | `0` [Engine] |
| `gfx_bHDRRendering` | bool | (cfg), Engine, GfxD3D, Sam2Game | `0` [Engine] |
| `gfx_bMaximized` | bool | (cfg), Engine, Sam2Game | `0` [Engine] |
| `gfx_bMultiMonDisabled` | bool | Engine, GfxOGL |  |
| `gfx_bNoAlphaBlending` | bool | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_bNoAlphaTesting` | bool | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_bNoStaticGeometry` | bool | Engine, GfxD3D, GfxOGL |  |
| `gfx_bOutputGPUPrograms` | bool | Engine, GfxD3D, GfxOGL |  |
| `gfx_bRelaxedOcclusionTest` | bool | (cfg), Engine | `0` [Engine] |
| `gfx_bRenderParticles` | bool | Engine |  |
| `gfx_bSimulatedFullScreen` | bool | Engine, Sam2Game |  |
| `gfx_ctAntiAliasingSamples` | count | Engine, GfxD3D, GfxOGL, Sam2Game |  |
| `gfx_ctMaxOcclusionQueries` | count | (cfg), Engine | `1024` [Engine] |
| `gfx_ctMaxPixelProgramBooleanConstants` | count | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_ctMaxPixelProgramFloatConstants` | count | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_ctMaxPixelProgramIntegerConstants` | count | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_ctMaxVertexProgramBooleanConstants` | count | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_ctMaxVertexProgramFloatConstants` | count | Engine, GfxD3D, GfxOGL, GfxSWR, Shaders |  |
| `gfx_ctMaxVertexProgramIntegerConstants` | count | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_ctMonitors` | count | Engine, GfxOGL |  |
| `gfx_ctTextureUnits` | count | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_ctTriangles` | count | Engine, GfxD3D, GfxOGL |  |
| `gfx_ctVertices` | count | Engine, GfxD3D, GfxOGL |  |
| `gfx_eCurrentAPI` | enum | Engine, GfxD3D, GfxOGL, GfxSWR, Sam2Game, Shaders |  |
| `gfx_fAspectRatio` | float | (cfg), Sam2Game | `1.3333` [Sam2Game] |
| `gfx_fBlueBias` | float | (cfg), Engine | `1` [Engine] |
| `gfx_fBrightness` | float | (cfg), Engine, Sam2Game | `0` [Engine] |
| `gfx_fContrast` | float | (cfg), Engine, Sam2Game | `1` [Engine] |
| `gfx_fGamma` | float | (cfg), Engine, Sam2Game | `1` [Engine] |
| `gfx_fGreenBias` | float | (cfg), Engine | `1` [Engine] |
| `gfx_fMaxLineWidth` | float | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_fMaxPointSize` | float | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_fMaxTextureLODBias` | float | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_fRedBias` | float | (cfg), Engine | `1` [Engine] |
| `gfx_fSubPixelPrecision` | float | Engine, GfxD3D |  |
| `gfx_iAPI` | int | (cfg), Engine | `2` [Engine] |
| `gfx_iAntiAliasing` | int | (cfg), Engine, GfxD3D, GfxOGL, Sam2Game | `0` [Engine] |
| `gfx_iCurrentAdapter` | int | Engine, GfxD3D, GfxOGL, GfxSWR, Sam2Game |  |
| `gfx_iFinish` | int | (cfg), Engine, Sam2Game | `0` [Engine] |
| `gfx_iFrameNumber` | int | Engine, GfxD3D, GfxOGL, ProcRender |  |
| `gfx_iLevels` | int | (cfg), Engine | `256` [Engine] |
| `gfx_iMaxAllowedPixProgVer` | int | Engine |  |
| `gfx_iMaxFPSActive` | int | (cfg), Engine, Sam2Game | `10000` [Engine] |
| `gfx_iMaxFPSInactive` | int | (cfg), Engine | `20` [Engine] |
| `gfx_iMaxInputStreams` | int | Engine, GfxD3D, GfxOGL, Shaders |  |
| `gfx_iMaxTextureAnisotropy` | int | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_iPixelProgramVersion` | int | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_iRefreshRate` | int | (cfg), Engine | `0` [Engine] |
| `gfx_iVertexProgramVersion` | int | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_iWaitVSyncs` | int | (cfg), Engine, GfxD3D, GfxOGL, Sam2Game | `0` [Engine] |
| `gfx_pixCurrentResHeight` | pixels | Engine |  |
| `gfx_pixCurrentResWidth` | pixels | Engine |  |
| `gfx_pixMaxRenderTargetSize` | pixels | Engine, GfxD3D, GfxOGL |  |
| `gfx_pixMaxTextureSize` | pixels | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_pixMaxVolumeTextureSize` | pixels | Engine, GfxD3D, GfxOGL, GfxSWR |  |
| `gfx_pixResHeight` | pixels | (cfg), Engine, Sam2Game | `768` [Engine] |
| `gfx_pixResWidth` | pixels | (cfg), Engine, Sam2Game | `1024` [Engine] |
| `gfx_pixWinHeight` | pixels | (cfg), Engine, Sam2Game | `480` [Engine] |
| `gfx_pixWinPosX` | pixels | (cfg), Engine | `64` [Engine] |
| `gfx_pixWinPosY` | pixels | (cfg), Engine | `32` [Engine] |
| `gfx_pixWinWidth` | pixels | (cfg), Engine, Sam2Game | `640` [Engine] |
| `gfx_ulFlags` | unsigned long | Engine, GfxD3D, GfxOGL, GfxSWR, ProcRender, Sam2Game, Shaders |  |

## `hud_` (16)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `hud_bNetricsaActive` | bool | Sam2Game |  |
| `hud_bShowCheatsInfo` | bool | Sam2Game |  |
| `hud_bShowCoords` | bool | (cfg), Sam2Game | `0` [Sam2Game] |
| `hud_bShowCopyright` | bool | (cfg), Sam2Game | `0` [Sam2Game] |
| `hud_bShowCrosshair` | bool | (cfg), Sam2Game | `1` [Sam2Game] |
| `hud_bShowInfo` | bool | (cfg), Sam2Game | `1` [Sam2Game] |
| `hud_bShowLevelName` | bool | (cfg), Sam2Game | `0` [Sam2Game] |
| `hud_bShowLogo` | bool | (cfg), Sam2Game | `0` [Sam2Game] |
| `hud_bShowPausedMessage` | bool | Sam2Game |  |
| `hud_bShowPing` | bool | (cfg), Sam2Game | `0` [Sam2Game] |
| `hud_bShowTime` | bool | (cfg), Sam2Game | `0` [Sam2Game] |
| `hud_bShowWeapon` | bool | (cfg), Sam2Game | `1` [Sam2Game] |
| `hud_bWeaponsMenuActive` | bool | Sam2Game |  |
| `hud_fVerticalOffset` | float | Sam2Game |  |
| `hud_iShowBandwidth` | int | (cfg), Sam2Game | `0` [Sam2Game] |
| `hud_vFirstRelElmPos` | vector | Sam2Game |  |

## `inp_` (8)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `inp_bGamepadChecks` | bool | Core, Engine |  |
| `inp_bInvertMouseY` | bool | Engine |  |
| `inp_bMouseSmoothing` | bool | Engine, Sam2Game |  |
| `inp_fMouseSensitivity` | float | Engine |  |
| `inp_iControllerDeadzone` | int | Engine, Sam2Game |  |
| `inp_tmGamepadsChanged` | time | Core, Engine |  |
| `inp_tmLastMouse` | time | Engine |  |
| `inp_tmLastXinput` | time | Engine |  |

## `lmp_` (5)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `lmp_bAllowAnisotropy` | bool | (cfg), Engine, Sam2Game | `0` [Engine] |
| `lmp_iBaseMip` | int | (cfg), Engine, Sam2Game | `0` [Engine] |
| `lmp_iMaxSize` | int | (cfg), Engine, Sam2Game | `8` [Engine] |
| `lmp_iQuality` | int | (cfg), Engine, Sam2Game | `0` [Engine] |
| `lmp_iRenderingMode` | int | Engine |  |

## `md_` (16)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `md_bAllowRawLoading` | bool | Core, Engine |  |
| `md_bCollectingForClear` | bool | Core |  |
| `md_bCopyClipboard` | bool | Core |  |
| `md_bCopyEditData` | bool | Core, Engine |  |
| `md_bCopyExternalHandles` | bool | Core, Engine, Sam2Game |  |
| `md_bCopyRetainsAll` | bool | Core, Sam2Game |  |
| `md_bInEditor` | bool | Core, Engine, OnlineGS, Sam2Game |  |
| `md_bLoadMiscasedTypes` | bool | Core, Engine |  |
| `md_bPrecaching` | bool | Core, Engine |  |
| `md_bReportReadStats` | bool | Core, Engine |  |
| `md_bSaveAliases` | bool | Core |  |
| `md_bSaveDiscardsEditData` | bool | Core |  |
| `md_bSaveTextFiles` | bool | Core |  |
| `md_bShareExposedResources` | bool | Core, Engine |  |
| `md_iReportUnusedObjects` | int | Core, Engine |  |
| `md_ulUniqueEventID` | unsigned long | Engine |  |

## `mdl_` (5)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `mdl_bAllowAnimations` | bool | Engine |  |
| `mdl_bCacheAnimations` | bool | (cfg), Engine, Sam2Game | `1` [Engine] |
| `mdl_bHardwareSkinning` | bool | (cfg), Engine, Sam2Game | `1` [Engine] |
| `mdl_ctMaxCachedModels` | count | (cfg), Engine | `64` [Engine] |
| `mdl_iMaxWeightsPerVertex` | int | (cfg), Engine, Shaders | `2` [Engine] |

## `men_` (16)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `men_bAttractMode` | bool | Sam2Game |  |
| `men_bDetectVideoChange` | bool | Sam2Game |  |
| `men_bDisableMenuSystem` | bool | Sam2Game |  |
| `men_bEditing` | bool | Sam2Game |  |
| `men_bRenderGadgetGrid` | bool | Sam2Game |  |
| `men_bRenderSafeArea` | bool | Sam2Game |  |
| `men_bRenderSafeZone` | bool | (cfg), Sam2Game | `0` [Sam2Game] |
| `men_bShowActiveProfile` | bool | Sam2Game |  |
| `men_bShowHiddenKeys` | bool | Sam2Game |  |
| `men_fAttractDelay` | float | Sam2Game |  |
| `men_fGadgetGridSpacingX` | float | Sam2Game |  |
| `men_fMusicVolume` | float | Sam2Game |  |
| `men_pixSafeZoneHeight` | pixels | Sam2Game |  |
| `men_pixSafeZoneWidth` | pixels | Sam2Game |  |
| `men_slLabelsFontHeight` | long | Sam2Game |  |
| `men_slOptionsFontHeight` | long | Sam2Game |  |

## `mmp_` (5)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `mmp_bIgnoreBlocksWithDesc` | bool | Core, Engine |  |
| `mmp_slMinBlockSize` | long | Core, Engine |  |
| `mmp_strAcceptedDescs` | string | Core, Engine |  |
| `mmp_strAcceptedTypes` | string | Core, Engine |  |
| `mmp_strIgnoredDescs` | string | Core, Engine |  |

## `mmt_` (4)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `mmt_bSortOnlyBySize` | bool | Core, Engine |  |
| `mmt_bTrackStrings` | bool | Core, Engine |  |
| `mmt_slMinBlockSize` | long | Core, Engine |  |
| `mmt_strTrackString` | string | Core, Engine |  |

## `net_` (46)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `net_bBlockStockCleanup` | bool | Engine, Sam2Game |  |
| `net_bCablePluggedIn` | bool | Engine |  |
| `net_bClientsMayPause` | bool | Engine, Sam2Game |  |
| `net_bDebuggingEnabled` | bool | Engine |  |
| `net_bLookupHostNames` | bool | Engine |  |
| `net_bReadUpdateCollectStrings` | bool | Engine |  |
| `net_bReportCommErrors` | bool | Engine |  |
| `net_bReportICMPErrors` | bool | Engine |  |
| `net_bReportMiscErrors` | bool | Engine |  |
| `net_bReportPackets` | bool | Engine |  |
| `net_bReportSocketErrors` | bool | Engine |  |
| `net_bReportUnknownErrors` | bool | Engine |  |
| `net_bSteamGameserver` | bool | Engine, Sam2Game |  |
| `net_bUseLatencyEmulation` | bool | Engine |  |
| `net_bUseVAC` | bool | Engine |  |
| `net_ctUpdateTimeCheckStep` | count | Engine |  |
| `net_fBandwidthInterval` | float | Engine |  |
| `net_fConnectionTimeout` | float | Engine |  |
| `net_fDisconnectPeriod` | float | Engine |  |
| `net_fDropPackets` | float | Engine |  |
| `net_fEnumerationInterval` | float | Engine |  |
| `net_fMaxEnumerationAge` | float | Engine |  |
| `net_fMessageDelayLimit` | float | Engine |  |
| `net_fProblemsTimeout` | float | Engine |  |
| `net_fRelevantTestsInterval` | float | Engine |  |
| `net_fUpdateTimeLimit` | float | Engine |  |
| `net_iCompression` | int | Engine |  |
| `net_iEnumerationSpread` | int | Engine |  |
| `net_iMaxClients` | int | Engine |  |
| `net_iMaxObservers` | int | Engine |  |
| `net_iPort` | int | Engine, OnlineGS |  |
| `net_iServerPort` | int | Engine, Sam2Game |  |
| `net_iSteamConnectionTimeout` | int | Engine |  |
| `net_iVIPReserve` | int | Engine |  |
| `net_idSteamServerLobby` | id | Engine, Sam2Game |  |
| `net_nsNetStats` | net stats | Engine |  |
| `net_slMaxPacketSize` | long | Engine |  |
| `net_slUDPBlockSize` | long | Engine |  |
| `net_strAdminPassword` | string | Engine |  |
| `net_strConnectPassword` | string | Engine |  |
| `net_strLocalHost` | string | Engine |  |
| `net_strObserverPassword` | string | Engine |  |
| `net_strVIPPassword` | string | Engine |  |
| `net_ulMaxUpdateDuration` | unsigned long | Engine |  |
| `net_ulServerAddress` | unsigned long | Engine, Sam2Game |  |
| `net_uwServerPort` | unsigned short | Engine |  |

## `ogl_` (1)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `ogl_bForcePixelProgramPartialPrecision` | bool | (cfg), Engine | `0` [Engine] |

## `onl_` (3)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `onl_bAutoLogin` | bool | Engine |  |
| `onl_bCheckForUpdates` | bool | Engine |  |
| `onl_fVoiceThroughSpeakersVolume` | float | Engine, Sam2Game |  |

## `phy_` (32)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `phy_bAdaptiveIterations` | bool | Engine |  |
| `phy_bBouncing` | bool | Engine |  |
| `phy_bContactReduction` | bool | Engine |  |
| `phy_bDepthCorrection` | bool | Engine |  |
| `phy_bDisableSphereInertiaForcing` | bool | Engine |  |
| `phy_bFrictionIndex` | bool | Engine |  |
| `phy_bFrictionRandomized` | bool | Engine |  |
| `phy_bFrictionlessContact` | bool | Engine |  |
| `phy_bSleeping` | bool | Engine |  |
| `phy_bTransferAngularMomentum` | bool | Engine |  |
| `phy_bTunnelingProtection` | bool | Engine |  |
| `phy_bWarmStarting` | bool | Engine |  |
| `phy_fAngularVelocityThreshold` | float | Engine |  |
| `phy_fBallCsp` | float | Engine |  |
| `phy_fGlobalCsp` | float | Engine |  |
| `phy_fGlobalSp` | float | Engine |  |
| `phy_fJointFriction` | float | Engine |  |
| `phy_fLimitSp` | float | Engine |  |
| `phy_fLinearVelocityThreshold` | float | Engine |  |
| `phy_fQuickSleepAngularVelocityThreshold` | float | Engine |  |
| `phy_fQuickSleepLinearVelocityThreshold` | float | Engine |  |
| `phy_fSORFactor` | float | Engine |  |
| `phy_fSleepDelta` | float | Engine |  |
| `phy_fSleepThresholdBaseFPS` | float | Engine |  |
| `phy_fSolutionThreshold` | float | Engine |  |
| `phy_fSubstepSize` | float | Engine |  |
| `phy_fWarmStartFactor` | float | Engine |  |
| `phy_iAccIterations` | int | Engine |  |
| `phy_iContactClusters` | int | Engine |  |
| `phy_iEquationOrder` | int | Engine |  |
| `phy_iMaxThreads` | int | Engine |  |
| `phy_iSolverSubsteps` | int | Engine |  |

## `plp_` (3)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `plp_bClearAchievementProgress` | bool | Sam2Game |  |
| `plp_iMaxPlayerProfiles` | int | Sam2Game |  |
| `plp_strActiveProfile` | string | Sam2Game |  |

## `prf_` (10)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `prf_bAutoFreezeProfiler` | bool | Core, Engine |  |
| `prf_bFrozenProfiler` | bool | Core, Engine |  |
| `prf_bShowFPS` | bool | Engine |  |
| `prf_bShowFPSGraph` | bool | Engine |  |
| `prf_bShowGeomteryInfo` | bool | Engine |  |
| `prf_bShowProfiler` | bool | Core, Engine, ProcRender, Sam2Game, SfxDSD, SfxOAL, Shaders |  |
| `prf_iLineOffset` | int | Engine, Sam2Game |  |
| `prf_iMaxNodeDepth` | int | Core, Engine |  |
| `prf_strOutputFilter` | string | Core, Engine |  |
| `prf_tmRefreshInterval` | time | Core, Engine |  |

## `rcon_` (1)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `rcon_slMaxBufferSize` | long | Engine |  |

## `rcts_` (4)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `rcts_bDebugNegotiations` | bool | Engine |  |
| `rcts_bLocalMode` | bool | Engine |  |
| `rcts_strAdminPassword` | string | Engine |  |
| `rcts_strWelcomeNote` | string | Engine |  |

## `ren_` (21)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `ren_bEffectOcclusion` | bool | (cfg), Engine, ProcRender, Sam2Game | `0` [Engine] |
| `ren_bFreezeVisibility` | bool | Engine |  |
| `ren_bFrustumCulling` | bool | Engine |  |
| `ren_bInverseDepthSort` | bool | Engine |  |
| `ren_bNoPuppets` | bool | Engine, Sam2Game |  |
| `ren_bOcclusionCulling` | bool | (cfg), Engine, ProcRender, Sam2Game | `1` [Engine] |
| `ren_bOpaqueOnly` | bool | Engine |  |
| `ren_bShowOccludees` | bool | Engine |  |
| `ren_bVisibilityGroups` | bool | (cfg), Engine, Sam2Game | `1` [Engine] |
| `ren_ctOcclusionPassThreshold` | count | (cfg), Engine | `16` [Engine] |
| `ren_fCullBiasSizeTreshold` | float | Engine |  |
| `ren_fDynamicShadowCullSize` | float | (cfg), Engine, Sam2Game | `0.1` [Engine] |
| `ren_fDynamicShadowFadeSize` | float | (cfg), Engine, Sam2Game | `0.2` [Engine] |
| `ren_fDynamicShadowSlanting` | float | (cfg), Engine, Sam2Game | `0.5` [Engine] |
| `ren_fLODBias` | float | (cfg), Engine, Sam2Game | `1` [Engine] |
| `ren_fNearClipPlaneDistance` | float | (cfg), Engine, Sam2Game | `0.1` [Engine] |
| `ren_fOcclusionSizeTreshold` | float | (cfg), Engine, Sam2Game | `0.5` [Engine] |
| `ren_iDynamicShadowQuality` | int | (cfg), Engine, Sam2Game | `1` [Engine] |
| `ren_mView` | matrix | Engine |  |
| `ren_pixDynamicShadowSize` | pixels | (cfg), Engine, Sam2Game | `128` [Engine] |
| `ren_vViewer` | vector | Engine, ProcRender |  |

## `sam_` (51)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `sam_bAllowAchievementsCheating` | bool | Sam2Game |  |
| `sam_bBootSequence` | bool | (cfg), Sam2Game | `1` [Sam2Game] |
| `sam_bDebugNotifications` | bool | Sam2Game |  |
| `sam_bDetectInfinityCharacters` | bool | Sam2Game |  |
| `sam_bEnableDebris` | bool | (cfg), Sam2Game | `1` [Sam2Game] |
| `sam_bEnableRagdollDeaths` | bool | (cfg), Sam2Game | `0` [Sam2Game] |
| `sam_bEnableUziReloading` | bool | Sam2Game |  |
| `sam_bEnableUziScreens` | bool | Sam2Game |  |
| `sam_bFixInfiniteViewBug` | bool | Sam2Game |  |
| `sam_bFreezeInfiniteViewBug` | bool | Sam2Game |  |
| `sam_bJoinAsObserver` | bool | Sam2Game |  |
| `sam_bJoinAsPrivate` | bool | Sam2Game |  |
| `sam_bLANServer` | bool | Sam2Game |  |
| `sam_bLoopDemo` | bool | Sam2Game |  |
| `sam_bNetricsaSubtitles` | bool | (cfg), Sam2Game | `1` [Sam2Game] |
| `sam_bNetricsaVoice` | bool | (cfg), Sam2Game | `1` [Sam2Game] |
| `sam_bNpcSubtitles` | bool | Sam2Game |  |
| `sam_bNpcVoice` | bool | Sam2Game |  |
| `sam_bSamSubtitles` | bool | (cfg), Sam2Game | `1` [Sam2Game] |
| `sam_bSamVoice` | bool | (cfg), Sam2Game | `1` [Sam2Game] |
| `sam_bShowMacroWarningsForStaticModels` | bool | Sam2Game |  |
| `sam_bShowSafeArea` | bool | (cfg), Sam2Game | `0` [Sam2Game] |
| `sam_bSkipMovies` | bool | Sam2Game |  |
| `sam_bStartAsServer` | bool | Sam2Game |  |
| `sam_bStartDemoPlayback` | bool | Sam2Game |  |
| `sam_bStartMainMenu` | bool | Sam2Game |  |
| `sam_ctMaxDebris` | count | (cfg), Sam2Game | `20` [Sam2Game] |
| `sam_fEndOfRoundDelay` | float | Sam2Game |  |
| `sam_fEnemyWithNoFoePurgeTime` | float | Sam2Game |  |
| `sam_fManualCameraAngVelocity` | float | Sam2Game |  |
| `sam_fManualCameraVelocity` | float | Sam2Game |  |
| `sam_fSafeAreaSize` | float | Sam2Game |  |
| `sam_fVoteDuration` | float | Sam2Game |  |
| `sam_fVotePercentageToPass` | float | Sam2Game |  |
| `sam_iEnableCheats` | int | Sam2Game |  |
| `sam_iShowAllLevels` | int | Sam2Game |  |
| `sam_strConnect` | string | Sam2Game |  |
| `sam_strDatabaseRootDir` | string | Sam2Game |  |
| `sam_strDefaultMapDir` | string | Sam2Game |  |
| `sam_strDemoFile` | string | (cfg), Sam2Game | `""` [Sam2Game] |
| `sam_strDisabledItems` | string | Sam2Game |  |
| `sam_strDisabledVoteTypes` | string | Sam2Game |  |
| `sam_strFavoriteChapter` | string | (cfg), Sam2Game | `""` [Sam2Game] |
| `sam_strGraphicCardName` | string | Sam2Game |  |
| `sam_strLevelToStart` | string | Sam2Game |  |
| `sam_strMainMenuName` | string | (cfg), Sam2Game | `"SMainMenu"` [Sam2Game] |
| `sam_strMapList` | string | Sam2Game |  |
| `sam_strMapListFile` | string | Sam2Game |  |
| `sam_strMultiplayerSessionName` | string | Sam2Game |  |
| `sam_strOptionsScript` | string | Sam2Game |  |
| `sam_strServerName` | string | (cfg), Sam2Game | `""` [Sam2Game] |

## `ser_` (2)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `ser_bAllowSend` | bool | Engine, Sam2Game |  |
| `ser_strBanList` | string | Engine |  |

## `sfx_` (15)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `sfx_bAllowEAX` | bool | (cfg), Engine, Sam2Game, SfxDSD, SfxOAL | `0` [Engine] |
| `sfx_ctAdapters` | count | Engine, SfxDSD, SfxOAL |  |
| `sfx_ctStreamingPackets` | count | (cfg), Engine | `8` [Engine] |
| `sfx_eCurrentAPI` | enum | Engine, SfxDSD, SfxOAL |  |
| `sfx_fAttenuationByVoice` | float | (cfg), Engine, Sam2Game | `0.75` [Engine] |
| `sfx_fMasterVolume` | float | (cfg), Engine, Sam2Game | `1` [Engine] |
| `sfx_fSpawnDelay` | float | Sam2Game |  |
| `sfx_iAPI` | int | (cfg), Engine, Sam2Game | `2` [Engine] |
| `sfx_iCurrentAdapter` | int | Engine, SfxDSD, SfxOAL |  |
| `sfx_iMaxActiveSameSounds` | int | (cfg), Engine | `5` [Engine] |
| `sfx_iMaxActiveSounds` | int | (cfg), Engine, Sam2Game | `20` [Engine] |
| `sfx_iStreamingPacketSize` | int | (cfg), Engine | `8` [Engine] |
| `sfx_slMaxFrequency` | long | Engine, SfxDSD, SfxOAL |  |
| `sfx_slMinFrequency` | long | Engine, SfxDSD, SfxOAL |  |
| `sfx_ulFlags` | unsigned long | Engine, SfxDSD, SfxOAL |  |

## `sha_` (25)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `sha_bAllowOffsetMapping` | bool | (cfg) | `0` [Shaders] |
| `sha_bAllowRefractions` | bool | (cfg), Sam2Game, Shaders | `0` [Shaders] |
| `sha_bFastFlatCubeReflections` | bool | (cfg), Sam2Game, Shaders | `1` [Shaders] |
| `sha_bForceFlatCubeReflections` | bool | (cfg), Sam2Game, Shaders | `0` [Shaders] |
| `sha_bNoBillboardShader` | bool | Shaders |  |
| `sha_bNoBitmapShader` | bool | Shaders |  |
| `sha_bNoBlendLayersShader` | bool | Shaders |  |
| `sha_bNoOffsetMapping` | bool | Sam2Game, Shaders |  |
| `sha_bNoPolyBumpShader` | bool | Shaders |  |
| `sha_bNoReflectionShader` | bool | Shaders |  |
| `sha_bNoRefractionShader` | bool | Shaders |  |
| `sha_bNoShadeShader` | bool | Shaders |  |
| `sha_bNoSpecularShader` | bool | Shaders |  |
| `sha_bNoTerrainShader` | bool | Shaders |  |
| `sha_bNoUberShader` | bool | Shaders |  |
| `sha_bNoWaterShader` | bool | Shaders |  |
| `sha_bUberNoDetailMap` | bool | Shaders |  |
| `sha_bUberNoOrenNayar` | bool | Shaders |  |
| `sha_bUberNoReflection` | bool | Shaders |  |
| `sha_bUberNoSmoothHeight` | bool | Shaders |  |
| `sha_bUberNoSpecular` | bool | Shaders |  |
| `sha_bWireOverlay` | bool | Engine |  |
| `sha_ctMaxPrimitives` | count | Engine |  |
| `sha_iRefractionMip` | int | (cfg), Sam2Game, Shaders | `0` [Shaders] |
| `sha_iRenderingMode` | int | Engine |  |

## `sim_` (7)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `sim_bCheckWorldVersion` | bool | Engine |  |
| `sim_bReportMemoryUsage` | bool | Engine |  |
| `sim_bReportOpenedFiles` | bool | Engine |  |
| `sim_fRealTimeFactor` | float | Engine, Sam2Game |  |
| `sim_fStepMax` | float | Engine |  |
| `sim_fStepMin` | float | Engine |  |
| `sim_fSyncRate` | float | Engine |  |

## `snd_` (3)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `snd_fEffectsVolume` | float | (cfg), Engine, Sam2Game | `1` [Engine] |
| `snd_fMovieVolume` | float | (cfg), Engine, Sam2Game | `1` [Engine] |
| `snd_fMusicVolume` | float | (cfg), Engine, Sam2Game | `1` [Engine] |

## `str_` (4)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `str_fMultiplier` | float | Sam2Game |  |
| `str_strCurrentLocale` | string | Core, Engine |  |
| `str_strLocaleDir` | string | Core, Engine |  |
| `str_strTransPrefix` | string | Core, Engine |  |

## `tex_` (9)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `tex_bColorizeMipmaps` | bool | Engine |  |
| `tex_fLODBias` | float | (cfg), Engine, Sam2Game | `0` [Engine] |
| `tex_fLowCoverageThreshold` | float | Engine |  |
| `tex_iAnisotropy` | int | (cfg), Engine, Sam2Game | `1` [Engine] |
| `tex_iBaseMip` | int | (cfg), Engine, Sam2Game | `0` [Engine] |
| `tex_iFiltering` | int | (cfg), Engine, Sam2Game | `21` [Engine] |
| `tex_iMaxSize` | int | (cfg), Engine, Sam2Game | `8` [Engine] |
| `tex_iQuality` | int | (cfg), Engine, Sam2Game | `0` [Engine] |
| `tex_iReportAnomalies` | int | Engine |  |

## `thr_` (5)

| Name | Type | Seen in | Default |
|---|---|---|---|
| `thr_ctActiveThreads` | count | Core |  |
| `thr_iAffinityStrictness` | int | Core, Engine |  |
| `thr_iCurWorkerThreads` | int | Core, Engine |  |
| `thr_iMaxWorkerThreads` | int | Core, Engine |  |
| `thr_ulMainThreadID` | unsigned long | Core |  |

_Total: 450 names in 40 families._
