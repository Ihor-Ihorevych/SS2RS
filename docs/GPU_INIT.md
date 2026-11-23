# Starting the GPU, and getting it back

`ss2_render::gpu` (T04-001) decides how the renderer asks wgpu for a device, says what it got, and recovers when the device or the surface is lost. The decisions are plain values and functions, tested without a GPU; the `Renderer` (`context.rs`) uses them to start, and to rebuild itself.

## Choosing the backend and adapter

`GpuConfig` has three settings, read from the environment by `GpuConfig::from_env()` (`Renderer::new_windowed` and `new_offscreen` call it):

| Variable | Values | Default |
|---|---|---|
| `SS2_GPU_BACKEND` | `auto`, `vulkan`, `dx12`, `metal`, `gl` (also `vk`, `d3d12`, `mtl`, `gles`, `opengl`; any case) | `auto` (wgpu's own choice) |
| `SS2_GPU_POWER` | `high`, `low` (`performance`, `battery`, `integrated`) | `high` |
| `SS2_FORCE_SOFTWARE_ADAPTER` | `1` or `true` | off (hardware first, then software) |

A value that is not understood is a **warning in the log and the default**, never a crash: a typo in an environment variable must not stop the game from starting (and one bad word does not spoil a good one beside it).

## Asking for limits

`request_gpu` asks for an adapter (hardware, then the software one unless forced), then for a device with each `LimitTier` in turn: **`Default`** (wgpu's default limits: a PC, a recent phone) and then **`Downlevel`** (the least a GLES 3.0 or old Vulkan device offers, raised to the adapter's own texture size). A device that cannot give the default limits is therefore not refused, it is given less, and the report says so. This is what a later Android port needs; **it has not been run on a device that needs the second tier** (the test only checks that an adapter able to give the default limits is asked for them).

## The report

`DeviceReport` (`Renderer::report()`, and logged at `info` at start-up, one line each) says what the device turned out to be: adapter name, backend, type (discrete, integrated, virtual, software), driver, vendor and device ids, which limit tier, the limits the renderer sizes things by, the features the device was created with, the downlevel capabilities the adapter lacks, the surface formats and present modes (if windowed), and the multisample counts for the main target formats. On the machine this was written on:

```
GPU: NVIDIA GeForce RTX 4050 Laptop GPU (Vulkan, discrete GPU)
driver: NVIDIA 616.56  ids: vendor 0x10de device 0x28e1
limits requested: default
  max_texture_dimension_2d = 8192
  ...
features: none
msaa Bgra8Unorm: 2x 4x 8x
```

(The same adapter answers as `Dx12` and as `Gl` when asked, and the software adapter as "Microsoft Basic Render Driver (Dx12, software (CPU))".)

## Losing the device

The device-lost callback only **sets a flag** (`LostSignal`; it runs on a wgpu thread). The renderer looks at the flag, after a no-wait poll, **at the start of every `render`, `render_to_image` and `capture_frame`**, and then asks the `RecoveryTracker`:

- **Rebuild.** It asks for a new adapter and device, puts the surface (reconfigured on the new device) or the offscreen texture on it, and builds every GPU object again (pipeline, buffers, text atlas). Nothing is lost that is not redrawn from the draw list the next frame. The tracker carries over, and `RenderStats::device_rebuilds` counts them.
- **Give up.** More than **3 rebuilds within 60 s** is `RenderError::DeviceLost`: a device that is lost again at once will not be saved by another try, and the caller gets an error instead of an endless loop. Rebuilds that are far apart never add up.
- **A lost surface** (outdated or lost; `Timeout` and `Occluded` are not failures, they just skip the frame) is reconfigured and the frame skipped; **30 such frames in a row** count as a lost device (the surface only comes back with its window, and some drivers only recover with a new device). A good frame ends the run. `Suboptimal` is reconfigured without counting.

## How it was tested

Forty-eight unit tests in the crate (twenty-two of them new with this task), **run on this machine's Vulkan, DX12, GL and software (WARP) adapters**:

- Pure: the backend and power words (every spelling, unknown words, one bad word beside a good one), the configuration, the limit tiers (the downlevel limits keep the adapter's texture size and ask for no more than the default), the lost signal (first reason kept, rearms when taken), the tracker (the window's edge, the cap's edge, a surface run's edge and its reset after a rebuild or a good frame, giving up and staying given up, far-apart rebuilds), the report as lines.
- **On a real device:** `device.destroy()` on a live offscreen renderer: the next frame rebuilds it and **draws the identical picture**, twice in a row; a device destroyed in a loop ends in `DeviceLost` after exactly three rebuilds; a healthy renderer never rebuilds; `render()` notices a lost device before anything else; and the report of a real device is filled in.
- **Sixteen deliberate breakages** of the logic and the Renderer wiring: thirteen failed a test at once (including dropping the no-wait poll, which shows that the callback needs it); **three survived my first tests** and are now covered: the tier order of the request, the word `integrated`, and the device check at the start of `render`.

## A hang found afterwards, and its fix

Twice, in full workspace test runs made while the machine was busy, the `ss2-render` lib test process stalled for over half an hour in its GPU tests (they create devices, and some destroy them, and the tests of one process run in parallel). It did not happen in 25 quiet runs. **Reproduced** by running six copies of the test binary at once, five rounds: **3 of 30 runs hung** (killed at 90 s). Every test that makes or destroys a device now takes one process-wide lock (`gpu::gpu_test_guard`), as the integration tests already did for the software adapter; the same load gives **0 of 30 hangs**. The cause inside wgpu or the driver was not found, so this is a fix of the symptom (devices of one process are no longer created and destroyed at the same time), and nothing says the renderer itself cannot hit it if a game ever does the same from two threads.

## Not covered (follow-up T04-001b)

- **A real window.** The windowed path (a lost or outdated surface, a rebuild with a surface) is the same code as the offscreen one plus the surface calls, and the tracker that drives it is tested, but **nothing here opened a window and lost it** (a minimise, a resolution change that makes the surface outdated, Android's suspend and resume).
- **Features.** The device is created with **no optional features** (so no timestamp queries, T04-019b, and no compressed texture formats, T04-008b); choosing and negotiating them is for those tasks.
- **Errors other than loss.** Out-of-memory and validation errors are not caught (wgpu's default handler applies); a wgpu error scope around resource creation is for the loader and streaming work (T04-003b).
- **Settings.** The choices are environment variables; a settings-file entry and a menu are for the settings screen (T10-012b). There is no `--gpu-report` command-line switch yet, only the log.
- **Weak devices.** The downlevel tier has never been needed on any machine used so far.
