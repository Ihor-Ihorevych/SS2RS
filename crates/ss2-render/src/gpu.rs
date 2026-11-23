//! GPU start-up and recovery (roadmap T04-001): which backend and adapter to ask for, which limits to
//! request, what the device turned out to be (the report), and what to do when the surface or the
//! device is lost.
//!
//! The decisions are plain values and functions so they can be tested without a GPU
//! ([`GpuConfig`], [`LimitTier`], [`RecoveryTracker`], [`DeviceReport::lines`]); the code that calls
//! wgpu with them is small ([`request_gpu`]) and the [`crate::Renderer`] uses it to start and to
//! rebuild itself. See `docs/GPU_INIT.md`.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::RenderError;

/// The environment variable that picks a graphics API: `vulkan`, `dx12`, `metal`, `gl` or `auto`.
pub const BACKEND_ENV: &str = "SS2_GPU_BACKEND";
/// The environment variable that picks which adapter to prefer: `high` (default) or `low` power.
pub const POWER_ENV: &str = "SS2_GPU_POWER";

/// Which graphics API to ask wgpu for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackendChoice {
    /// Whatever wgpu prefers on this platform (Vulkan, Metal or DX12; GL only as a last resort).
    #[default]
    Auto,
    /// Vulkan (Linux, Windows, Android).
    Vulkan,
    /// Direct3D 12 (Windows).
    Dx12,
    /// Metal (macOS, iOS).
    Metal,
    /// OpenGL or GLES (the fallback of old devices).
    Gl,
}

impl BackendChoice {
    /// Parse a user's word for a backend, ignoring case and spaces. `None` for anything else.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "" | "auto" | "default" => Some(Self::Auto),
            "vulkan" | "vk" => Some(Self::Vulkan),
            "dx12" | "d3d12" | "directx12" => Some(Self::Dx12),
            "metal" | "mtl" => Some(Self::Metal),
            "gl" | "gles" | "opengl" => Some(Self::Gl),
            _ => None,
        }
    }

    /// The backends to enable, or `None` to leave wgpu's own default.
    pub fn backends(self) -> Option<wgpu::Backends> {
        match self {
            Self::Auto => None,
            Self::Vulkan => Some(wgpu::Backends::VULKAN),
            Self::Dx12 => Some(wgpu::Backends::DX12),
            Self::Metal => Some(wgpu::Backends::METAL),
            Self::Gl => Some(wgpu::Backends::GL),
        }
    }

    /// The word for this choice, as [`parse`](Self::parse) takes it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Vulkan => "vulkan",
            Self::Dx12 => "dx12",
            Self::Metal => "metal",
            Self::Gl => "gl",
        }
    }
}

/// Which kind of adapter to prefer when there is a choice (a laptop with two GPUs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PowerChoice {
    /// The fast one (the default for a game).
    #[default]
    High,
    /// The one that saves the battery.
    Low,
}

impl PowerChoice {
    /// Parse `high` or `low` (and the words `performance` and `battery`), ignoring case.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "" | "high" | "performance" => Some(Self::High),
            "low" | "battery" | "integrated" => Some(Self::Low),
            _ => None,
        }
    }

    fn to_wgpu(self) -> wgpu::PowerPreference {
        match self {
            Self::High => wgpu::PowerPreference::HighPerformance,
            Self::Low => wgpu::PowerPreference::LowPower,
        }
    }
}

/// How to start the GPU: which API, which adapter, and whether to insist on the software one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GpuConfig {
    /// The graphics API.
    pub backend: BackendChoice,
    /// Which adapter to prefer.
    pub power: PowerChoice,
    /// Use only the software (CPU) adapter: for a result that does not depend on the machine.
    pub force_software: bool,
}

/// A [`GpuConfig`] and what was wrong with the words that made it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParsedConfig {
    /// The configuration, with the default for every word that could not be understood.
    pub config: GpuConfig,
    /// One line for each word that was not understood, to log.
    pub warnings: Vec<String>,
}

impl GpuConfig {
    /// Build a configuration from the values of [`BACKEND_ENV`], [`POWER_ENV`] and
    /// [`crate::golden::FORCE_SOFTWARE_ENV`]. A word that is not understood is a warning and the
    /// default is used, never a crash: a typo must not stop the game from starting.
    pub fn from_values(
        backend: Option<&str>,
        power: Option<&str>,
        force_software: Option<&str>,
    ) -> ParsedConfig {
        let mut warnings = Vec::new();
        let backend = match backend {
            None => BackendChoice::Auto,
            Some(word) => BackendChoice::parse(word).unwrap_or_else(|| {
                warnings.push(format!(
                    "{BACKEND_ENV}={word:?} is not one of auto, vulkan, dx12, metal, gl; using auto"
                ));
                BackendChoice::Auto
            }),
        };
        let power = match power {
            None => PowerChoice::High,
            Some(word) => PowerChoice::parse(word).unwrap_or_else(|| {
                warnings.push(format!(
                    "{POWER_ENV}={word:?} is not high or low; using high"
                ));
                PowerChoice::High
            }),
        };
        ParsedConfig {
            config: GpuConfig {
                backend,
                power,
                force_software: crate::golden::force_software_requested_from(force_software),
            },
            warnings,
        }
    }

    /// The configuration from the process environment; warnings are logged.
    pub fn from_env() -> GpuConfig {
        let get = |name: &str| std::env::var(name).ok();
        let parsed = Self::from_values(
            get(BACKEND_ENV).as_deref(),
            get(POWER_ENV).as_deref(),
            get(crate::golden::FORCE_SOFTWARE_ENV).as_deref(),
        );
        for warning in &parsed.warnings {
            tracing::warn!("{warning}");
        }
        parsed.config
    }
}

/// How much to ask of the device. A device that cannot give wgpu's default limits (an old phone, a
/// GLES driver) may still run this game's 2D and simple 3D, so the second tier asks for less.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitTier {
    /// wgpu's default limits (what a PC or a recent phone has).
    Default,
    /// The "downlevel" limits (the least a GLES 3.0 or old Vulkan device offers), raised to the
    /// adapter's own texture size.
    Downlevel,
}

impl LimitTier {
    /// The tiers to try, best first.
    pub const ORDER: [LimitTier; 2] = [LimitTier::Default, LimitTier::Downlevel];

    /// The limits to request from an adapter that offers `adapter`.
    pub fn limits(self, adapter: &wgpu::Limits) -> wgpu::Limits {
        match self {
            Self::Default => wgpu::Limits::default(),
            Self::Downlevel => wgpu::Limits::downlevel_defaults().using_resolution(adapter.clone()),
        }
    }

    /// A word for logs and the report.
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Downlevel => "downlevel",
        }
    }
}

/// Written by the device-lost callback (from any thread), read by the renderer once a frame.
#[derive(Clone, Debug, Default)]
pub struct LostSignal(Arc<Mutex<Option<String>>>);

impl LostSignal {
    /// A signal that has not fired.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that the device is lost, and why. The first reason is kept.
    pub fn set(&self, reason: impl Into<String>) {
        let mut slot = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if slot.is_none() {
            *slot = Some(reason.into());
        }
    }

    /// Whether it has fired.
    pub fn is_set(&self) -> bool {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    }

    /// Take the reason and rearm the signal; `None` if the device is fine.
    pub fn take(&self) -> Option<String> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
}

/// What the tracker tells the renderer to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Configure the surface again and skip the frame.
    Reconfigure,
    /// Make a new device (and adapter) and rebuild everything on it.
    RebuildDevice,
    /// Too many failures: report the error instead of trying again.
    GiveUp,
}

/// How patient to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoveryPolicy {
    /// How many times the device may be rebuilt within [`window_ms`](Self::window_ms) before giving up.
    pub max_rebuilds: usize,
    /// The window for counting rebuilds, in milliseconds.
    pub window_ms: u64,
    /// This many frames in a row with an unusable surface also count as a lost device (the surface
    /// stays lost until its window comes back, and some drivers only recover with a new device).
    pub bad_surface_frames: u32,
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self {
            max_rebuilds: 3,
            window_ms: 60_000,
            bad_surface_frames: 30,
        }
    }
}

/// Counts failures and decides between reconfiguring, rebuilding and giving up. Time is given by
/// the caller (milliseconds from any fixed start), so it is testable.
#[derive(Clone, Debug)]
pub struct RecoveryTracker {
    policy: RecoveryPolicy,
    rebuilds: VecDeque<u64>,
    bad_frames: u32,
}

impl RecoveryTracker {
    /// A tracker with no failures yet.
    pub fn new(policy: RecoveryPolicy) -> Self {
        Self {
            policy,
            rebuilds: VecDeque::new(),
            bad_frames: 0,
        }
    }

    /// The device was lost at `now_ms`: rebuild it, unless that has already been tried too often
    /// recently (a device that is lost again at once will not be saved by a fourth try).
    pub fn device_lost(&mut self, now_ms: u64) -> Verdict {
        while let Some(&oldest) = self.rebuilds.front() {
            if now_ms.saturating_sub(oldest) >= self.policy.window_ms {
                self.rebuilds.pop_front();
            } else {
                break;
            }
        }
        if self.rebuilds.len() >= self.policy.max_rebuilds {
            return Verdict::GiveUp;
        }
        self.rebuilds.push_back(now_ms);
        self.bad_frames = 0;
        Verdict::RebuildDevice
    }

    /// The surface was outdated or lost on this frame.
    pub fn surface_unusable(&mut self, now_ms: u64) -> Verdict {
        self.bad_frames += 1;
        if self.bad_frames >= self.policy.bad_surface_frames {
            self.device_lost(now_ms)
        } else {
            Verdict::Reconfigure
        }
    }

    /// A frame was drawn: the run of bad surface frames is over.
    pub fn frame_ok(&mut self) {
        self.bad_frames = 0;
    }

    /// How many rebuilds are still counted against the policy's window.
    pub fn recent_rebuilds(&self) -> usize {
        self.rebuilds.len()
    }
}

impl Default for RecoveryTracker {
    fn default() -> Self {
        Self::new(RecoveryPolicy::default())
    }
}

/// What the adapter and device turned out to be: for the log at start-up and a bug report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceReport {
    /// The adapter's name.
    pub adapter: String,
    /// The graphics API in use.
    pub backend: String,
    /// "discrete GPU", "integrated GPU", "virtual GPU", "software (CPU)" or "other".
    pub device_type: String,
    /// The driver's name and its own description.
    pub driver: String,
    /// PCI vendor and device ids as the API reports them.
    pub vendor_device: (u32, u32),
    /// Which limits were requested.
    pub limit_tier: LimitTier,
    /// The limits of the device that matter to this renderer.
    pub limits: Vec<(String, u64)>,
    /// The features the device was created with.
    pub features: Vec<String>,
    /// The downlevel capabilities the adapter lacks (empty on a full desktop API).
    pub missing_downlevel: Vec<String>,
    /// The surface's formats and present modes, if there is a surface.
    pub surface: Option<(Vec<String>, Vec<String>)>,
    /// The multisample counts (above 1) the device supports, for the main target formats.
    pub msaa: Vec<(String, Vec<u32>)>,
}

impl DeviceReport {
    /// Whether this is a software (CPU) adapter: slow, but the same everywhere.
    pub fn is_software(&self) -> bool {
        self.device_type.starts_with("software")
    }

    /// The report as lines of text.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!(
                "GPU: {} ({}, {})",
                self.adapter, self.backend, self.device_type
            ),
            format!(
                "driver: {}  ids: vendor {:#06x} device {:#06x}",
                if self.driver.is_empty() {
                    "unknown"
                } else {
                    &self.driver
                },
                self.vendor_device.0,
                self.vendor_device.1
            ),
            format!("limits requested: {}", self.limit_tier.name()),
        ];
        for (name, value) in &self.limits {
            lines.push(format!("  {name} = {value}"));
        }
        lines.push(if self.features.is_empty() {
            "features: none".to_string()
        } else {
            format!(
                "features ({}): {}",
                self.features.len(),
                self.features.join(", ")
            )
        });
        if !self.missing_downlevel.is_empty() {
            lines.push(format!(
                "missing downlevel capabilities: {}",
                self.missing_downlevel.join(", ")
            ));
        }
        if let Some((formats, modes)) = &self.surface {
            lines.push(format!("surface formats: {}", formats.join(", ")));
            lines.push(format!("present modes: {}", modes.join(", ")));
        }
        for (format, counts) in &self.msaa {
            lines.push(if counts.is_empty() {
                format!("msaa {format}: none")
            } else {
                let counts: Vec<String> = counts.iter().map(|c| format!("{c}x")).collect();
                format!("msaa {format}: {}", counts.join(" "))
            });
        }
        lines
    }

    fn collect(
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        tier: LimitTier,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Self {
        let info = adapter.get_info();
        let device_type = match info.device_type {
            wgpu::DeviceType::DiscreteGpu => "discrete GPU",
            wgpu::DeviceType::IntegratedGpu => "integrated GPU",
            wgpu::DeviceType::VirtualGpu => "virtual GPU",
            wgpu::DeviceType::Cpu => "software (CPU)",
            wgpu::DeviceType::Other => "other",
        };
        let driver = match (info.driver.is_empty(), info.driver_info.is_empty()) {
            (true, true) => String::new(),
            (false, true) => info.driver.clone(),
            (true, false) => info.driver_info.clone(),
            (false, false) => format!("{} {}", info.driver, info.driver_info),
        };
        let surface = surface.map(|s| {
            let caps = s.get_capabilities(adapter);
            (
                caps.formats.iter().map(|f| format!("{f:?}")).collect(),
                caps.present_modes
                    .iter()
                    .map(|m| format!("{m:?}"))
                    .collect(),
            )
        });
        let msaa = [
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Depth32Float,
        ]
        .into_iter()
        .map(|format| {
            let flags = adapter.get_texture_format_features(format).flags;
            let counts = [2u32, 4, 8]
                .into_iter()
                .filter(|&n| flags.sample_count_supported(n))
                .collect();
            (format!("{format:?}"), counts)
        })
        .collect();
        let downlevel = adapter.get_downlevel_capabilities();
        Self {
            adapter: info.name,
            backend: format!("{:?}", info.backend),
            device_type: device_type.to_string(),
            driver,
            vendor_device: (info.vendor, info.device),
            limit_tier: tier,
            limits: limit_rows(&device.limits()),
            features: device
                .features()
                .iter_names()
                .map(|(name, _)| name.to_string())
                .collect(),
            missing_downlevel: wgpu::DownlevelFlags::all()
                .difference(downlevel.flags)
                .iter_names()
                .map(|(name, _)| name.to_string())
                .collect(),
            surface,
            msaa,
        }
    }
}

/// The limits worth reporting (the ones the renderer sizes things by).
pub fn limit_rows(limits: &wgpu::Limits) -> Vec<(String, u64)> {
    let row = |name: &str, value: u64| (name.to_string(), value);
    vec![
        row(
            "max_texture_dimension_2d",
            u64::from(limits.max_texture_dimension_2d),
        ),
        row(
            "max_texture_array_layers",
            u64::from(limits.max_texture_array_layers),
        ),
        row("max_bind_groups", u64::from(limits.max_bind_groups)),
        row(
            "max_bindings_per_bind_group",
            u64::from(limits.max_bindings_per_bind_group),
        ),
        row(
            "max_sampled_textures_per_shader_stage",
            u64::from(limits.max_sampled_textures_per_shader_stage),
        ),
        row(
            "max_uniform_buffer_binding_size",
            limits.max_uniform_buffer_binding_size,
        ),
        row(
            "max_storage_buffer_binding_size",
            limits.max_storage_buffer_binding_size,
        ),
        row("max_buffer_size", limits.max_buffer_size),
        row(
            "max_storage_buffers_per_shader_stage",
            u64::from(limits.max_storage_buffers_per_shader_stage),
        ),
        row("max_vertex_buffers", u64::from(limits.max_vertex_buffers)),
        row(
            "max_color_attachments",
            u64::from(limits.max_color_attachments),
        ),
    ]
}

/// The wgpu objects the renderer lives on, with the settings they were made from.
pub(crate) struct Gpu {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    pub lost: LostSignal,
    pub report: DeviceReport,
    pub config: GpuConfig,
    /// How a window surface blends with what is behind it (`Auto` without a surface).
    pub alpha_mode: wgpu::CompositeAlphaMode,
}

/// A new instance for `config`: with only the chosen backend if one was chosen.
pub(crate) fn make_instance(
    config: &GpuConfig,
    display: Option<Box<dyn wgpu::wgt::WgpuHasDisplayHandle>>,
) -> wgpu::Instance {
    let mut descriptor = match display {
        Some(display) => wgpu::InstanceDescriptor::new_with_display_handle(display),
        None => wgpu::InstanceDescriptor::new_without_display_handle(),
    };
    if let Some(backends) = config.backend.backends() {
        descriptor.backends = backends;
    }
    wgpu::Instance::new(descriptor)
}

/// Ask for an adapter (hardware first, then software, as `config` allows) and a device on it, trying
/// each [`LimitTier`] in turn. Registers the device-lost callback on the device it returns.
pub(crate) async fn request_gpu(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
    format_override: Option<wgpu::TextureFormat>,
    config: &GpuConfig,
) -> Result<Gpu, RenderError> {
    let mut last_error = String::from("no adapter returned");
    for &fallback in crate::golden::adapter_modes(config.force_software) {
        let options = wgpu::RequestAdapterOptions {
            power_preference: config.power.to_wgpu(),
            force_fallback_adapter: fallback,
            compatible_surface: surface,
            ..Default::default()
        };
        let adapter = match instance.request_adapter(&options).await {
            Ok(adapter) => adapter,
            Err(e) => {
                last_error = e.to_string();
                continue;
            }
        };
        let adapter_limits = adapter.limits();
        let mut device_error = String::new();
        let mut made = None;
        for tier in LimitTier::ORDER {
            let descriptor = wgpu::DeviceDescriptor {
                label: Some("ss2 device"),
                required_limits: tier.limits(&adapter_limits),
                ..Default::default()
            };
            match adapter.request_device(&descriptor).await {
                Ok(pair) => {
                    made = Some((pair, tier));
                    break;
                }
                Err(e) => device_error = format!("{} limits: {e}", tier.name()),
            }
        }
        let Some(((device, queue), tier)) = made else {
            last_error = device_error;
            continue;
        };
        let lost = LostSignal::new();
        {
            let lost = lost.clone();
            device.set_device_lost_callback(move |reason, message| {
                // Dropping a device on purpose reports "Destroyed" too: not an error.
                if reason == wgpu::DeviceLostReason::Destroyed {
                    tracing::debug!("GPU device destroyed: {message}");
                } else {
                    tracing::error!("GPU device lost ({reason:?}): {message}");
                }
                lost.set(format!("{reason:?}: {message}"));
            });
        }
        let format = match (format_override, surface) {
            (Some(f), _) => f,
            (None, Some(s)) => {
                let caps = s.get_capabilities(&adapter);
                caps.formats
                    .iter()
                    .copied()
                    .find(|f| !f.is_srgb())
                    .or_else(|| caps.formats.first().copied())
                    .ok_or_else(|| RenderError::Surface("surface reports no formats".into()))?
            }
            (None, None) => wgpu::TextureFormat::Rgba8Unorm,
        };
        let alpha_mode = surface
            .and_then(|s| s.get_capabilities(&adapter).alpha_modes.first().copied())
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);
        let report = DeviceReport::collect(&adapter, &device, tier, surface);
        for line in report.lines() {
            tracing::info!("{line}");
        }
        return Ok(Gpu {
            instance: instance.clone(),
            device,
            queue,
            format,
            lost,
            report,
            config: config.clone(),
            alpha_mode,
        });
    }
    Err(RenderError::NoAdapter(last_error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_words_are_understood_in_any_case_and_unknown_ones_are_not() {
        assert_eq!(BackendChoice::parse("Vulkan"), Some(BackendChoice::Vulkan));
        assert_eq!(BackendChoice::parse(" DX12 "), Some(BackendChoice::Dx12));
        assert_eq!(BackendChoice::parse("d3d12"), Some(BackendChoice::Dx12));
        assert_eq!(BackendChoice::parse("metal"), Some(BackendChoice::Metal));
        assert_eq!(BackendChoice::parse("GLES"), Some(BackendChoice::Gl));
        assert_eq!(BackendChoice::parse("auto"), Some(BackendChoice::Auto));
        assert_eq!(BackendChoice::parse(""), Some(BackendChoice::Auto));
        assert_eq!(BackendChoice::parse("directx11"), None);
        assert_eq!(BackendChoice::parse("vulkan,gl"), None);
        for choice in [
            BackendChoice::Auto,
            BackendChoice::Vulkan,
            BackendChoice::Dx12,
            BackendChoice::Metal,
            BackendChoice::Gl,
        ] {
            assert_eq!(BackendChoice::parse(choice.name()), Some(choice));
        }
    }

    #[test]
    fn each_backend_choice_enables_exactly_its_backend_and_auto_enables_none_explicitly() {
        assert_eq!(BackendChoice::Auto.backends(), None);
        assert_eq!(
            BackendChoice::Vulkan.backends(),
            Some(wgpu::Backends::VULKAN)
        );
        assert_eq!(BackendChoice::Dx12.backends(), Some(wgpu::Backends::DX12));
        assert_eq!(BackendChoice::Metal.backends(), Some(wgpu::Backends::METAL));
        assert_eq!(BackendChoice::Gl.backends(), Some(wgpu::Backends::GL));
    }

    #[test]
    fn power_words_are_understood() {
        assert_eq!(PowerChoice::parse("LOW"), Some(PowerChoice::Low));
        assert_eq!(PowerChoice::parse("battery"), Some(PowerChoice::Low));
        assert_eq!(PowerChoice::parse("integrated"), Some(PowerChoice::Low));
        assert_eq!(PowerChoice::parse("high"), Some(PowerChoice::High));
        assert_eq!(PowerChoice::parse("performance"), Some(PowerChoice::High));
        assert_eq!(PowerChoice::parse("medium"), None);
        assert_eq!(
            PowerChoice::High.to_wgpu(),
            wgpu::PowerPreference::HighPerformance
        );
        assert_eq!(PowerChoice::Low.to_wgpu(), wgpu::PowerPreference::LowPower);
    }

    #[test]
    fn nothing_set_gives_the_default_configuration_without_warnings() {
        let parsed = GpuConfig::from_values(None, None, None);
        assert_eq!(parsed.config, GpuConfig::default());
        assert!(parsed.warnings.is_empty());
        assert!(!parsed.config.force_software);
    }

    #[test]
    fn set_values_are_used() {
        let parsed = GpuConfig::from_values(Some("vulkan"), Some("low"), Some("1"));
        assert_eq!(
            parsed.config,
            GpuConfig {
                backend: BackendChoice::Vulkan,
                power: PowerChoice::Low,
                force_software: true,
            }
        );
        assert!(parsed.warnings.is_empty());
    }

    #[test]
    fn a_word_that_is_not_understood_is_a_warning_and_the_default_not_a_failure() {
        let parsed = GpuConfig::from_values(Some("glide"), Some("turbo"), Some("0"));
        assert_eq!(parsed.config, GpuConfig::default());
        assert_eq!(parsed.warnings.len(), 2, "{:?}", parsed.warnings);
        assert!(parsed.warnings[0].contains("glide") && parsed.warnings[0].contains(BACKEND_ENV));
        assert!(parsed.warnings[1].contains("turbo") && parsed.warnings[1].contains(POWER_ENV));
        // One bad word does not spoil a good one next to it.
        let parsed = GpuConfig::from_values(Some("glide"), Some("low"), None);
        assert_eq!(parsed.config.power, PowerChoice::Low);
        assert_eq!(parsed.config.backend, BackendChoice::Auto);
        assert_eq!(parsed.warnings.len(), 1);
    }

    #[test]
    fn the_default_tier_asks_for_wgpus_defaults_and_the_downlevel_one_for_less_but_keeps_the_adapters_textures()
     {
        let adapter = wgpu::Limits {
            max_texture_dimension_2d: 4096,
            ..wgpu::Limits::downlevel_defaults()
        };
        assert_eq!(LimitTier::ORDER[0], LimitTier::Default, "best tier first");
        assert_eq!(LimitTier::Default.limits(&adapter), wgpu::Limits::default());
        let down = LimitTier::Downlevel.limits(&adapter);
        assert_eq!(down.max_texture_dimension_2d, 4096);
        assert!(
            down.max_bind_groups <= wgpu::Limits::default().max_bind_groups,
            "downlevel asks for no more than the default"
        );
        assert!(
            down.max_texture_dimension_2d < wgpu::Limits::default().max_texture_dimension_2d,
            "and for smaller textures when the adapter has no more"
        );
        assert!(
            down.check_limits(&adapter),
            "an adapter that offers `adapter` can give these limits"
        );
    }

    #[test]
    fn the_lost_signal_keeps_the_first_reason_and_rearms_when_taken() {
        let signal = LostSignal::new();
        assert!(!signal.is_set());
        assert_eq!(signal.take(), None);
        let from_callback = signal.clone();
        from_callback.set("first");
        signal.set("second");
        assert!(signal.is_set());
        assert_eq!(signal.take().as_deref(), Some("first"));
        assert!(!signal.is_set());
        signal.set("third");
        assert_eq!(signal.take().as_deref(), Some("third"));
    }

    #[test]
    fn a_lost_device_is_rebuilt_a_limited_number_of_times_within_the_window() {
        let mut tracker = RecoveryTracker::new(RecoveryPolicy {
            max_rebuilds: 3,
            window_ms: 1000,
            bad_surface_frames: 30,
        });
        assert_eq!(tracker.device_lost(0), Verdict::RebuildDevice);
        assert_eq!(tracker.device_lost(10), Verdict::RebuildDevice);
        assert_eq!(tracker.device_lost(20), Verdict::RebuildDevice);
        assert_eq!(tracker.recent_rebuilds(), 3);
        assert_eq!(
            tracker.device_lost(30),
            Verdict::GiveUp,
            "a fourth loss within the window is not worth another try"
        );
        assert_eq!(
            tracker.device_lost(999),
            Verdict::GiveUp,
            "the first rebuild (at 0) is still inside the window at 999"
        );
        assert_eq!(
            tracker.device_lost(1000),
            Verdict::RebuildDevice,
            "at 1000 the first one has left the window (exactly window_ms old)"
        );
    }

    #[test]
    fn rebuilds_that_are_far_apart_never_add_up() {
        let mut tracker = RecoveryTracker::new(RecoveryPolicy {
            max_rebuilds: 1,
            window_ms: 100,
            bad_surface_frames: 30,
        });
        for i in 0..20u64 {
            assert_eq!(tracker.device_lost(i * 100), Verdict::RebuildDevice, "{i}");
        }
        assert_eq!(tracker.recent_rebuilds(), 1);
    }

    #[test]
    fn a_lost_surface_is_reconfigured_and_only_a_long_run_of_them_rebuilds_the_device() {
        let mut tracker = RecoveryTracker::new(RecoveryPolicy {
            max_rebuilds: 3,
            window_ms: 60_000,
            bad_surface_frames: 5,
        });
        for _ in 0..4 {
            assert_eq!(tracker.surface_unusable(0), Verdict::Reconfigure);
        }
        assert_eq!(
            tracker.surface_unusable(0),
            Verdict::RebuildDevice,
            "the fifth bad frame in a row"
        );
        assert_eq!(tracker.recent_rebuilds(), 1);
        for _ in 0..4 {
            assert_eq!(
                tracker.surface_unusable(1),
                Verdict::Reconfigure,
                "the run starts again after a rebuild"
            );
        }
    }

    #[test]
    fn a_good_frame_ends_the_run_of_bad_surface_frames() {
        let mut tracker = RecoveryTracker::new(RecoveryPolicy {
            max_rebuilds: 3,
            window_ms: 60_000,
            bad_surface_frames: 3,
        });
        for _ in 0..100 {
            assert_eq!(tracker.surface_unusable(0), Verdict::Reconfigure);
            assert_eq!(tracker.surface_unusable(0), Verdict::Reconfigure);
            tracker.frame_ok();
        }
        assert_eq!(tracker.recent_rebuilds(), 0);
    }

    #[test]
    fn a_surface_that_never_comes_back_ends_in_giving_up_and_stays_given_up() {
        let mut tracker = RecoveryTracker::new(RecoveryPolicy {
            max_rebuilds: 2,
            window_ms: 60_000,
            bad_surface_frames: 2,
        });
        let verdicts: Vec<Verdict> = (0..8).map(|_| tracker.surface_unusable(0)).collect();
        assert_eq!(
            verdicts,
            [
                Verdict::Reconfigure,
                Verdict::RebuildDevice,
                Verdict::Reconfigure,
                Verdict::RebuildDevice,
                Verdict::Reconfigure,
                Verdict::GiveUp,
                Verdict::GiveUp,
                Verdict::GiveUp,
            ]
        );
    }

    fn sample_report() -> DeviceReport {
        DeviceReport {
            adapter: "Test GPU 9000".into(),
            backend: "Vulkan".into(),
            device_type: "discrete GPU".into(),
            driver: "testdrv 1.2".into(),
            vendor_device: (0x10de, 0x2684),
            limit_tier: LimitTier::Default,
            limits: vec![("max_texture_dimension_2d".into(), 16384)],
            features: vec!["TIMESTAMP_QUERY".into(), "PUSH_CONSTANTS".into()],
            missing_downlevel: vec![],
            surface: Some((
                vec!["Bgra8Unorm".into(), "Rgba16Float".into()],
                vec!["Fifo".into(), "Mailbox".into()],
            )),
            msaa: vec![
                ("Bgra8Unorm".into(), vec![2, 4]),
                ("Depth32Float".into(), vec![]),
            ],
        }
    }

    #[test]
    fn the_report_reads_as_lines() {
        let lines = sample_report().lines();
        assert_eq!(
            lines,
            [
                "GPU: Test GPU 9000 (Vulkan, discrete GPU)",
                "driver: testdrv 1.2  ids: vendor 0x10de device 0x2684",
                "limits requested: default",
                "  max_texture_dimension_2d = 16384",
                "features (2): TIMESTAMP_QUERY, PUSH_CONSTANTS",
                "surface formats: Bgra8Unorm, Rgba16Float",
                "present modes: Fifo, Mailbox",
                "msaa Bgra8Unorm: 2x 4x",
                "msaa Depth32Float: none",
            ]
        );
    }

    #[test]
    fn the_report_shows_what_a_weak_device_lacks_and_says_so_when_there_is_no_driver_name() {
        let mut report = sample_report();
        report.driver.clear();
        report.features.clear();
        report.surface = None;
        report.missing_downlevel = vec!["COMPUTE_SHADERS".into(), "INDIRECT_EXECUTION".into()];
        report.limit_tier = LimitTier::Downlevel;
        let lines = report.lines();
        assert!(lines[1].starts_with("driver: unknown  ids:"), "{lines:?}");
        assert!(lines.contains(&"limits requested: downlevel".to_string()));
        assert!(lines.contains(&"features: none".to_string()));
        assert!(
            lines.contains(
                &"missing downlevel capabilities: COMPUTE_SHADERS, INDIRECT_EXECUTION".to_string()
            ),
            "{lines:?}"
        );
        assert!(
            !lines.iter().any(|l| l.starts_with("surface formats")),
            "no surface, no surface lines"
        );
    }

    #[test]
    fn only_a_cpu_adapter_is_software() {
        let mut report = sample_report();
        assert!(!report.is_software());
        report.device_type = "software (CPU)".into();
        assert!(report.is_software());
    }

    #[test]
    fn the_limit_rows_name_what_they_hold() {
        let limits = wgpu::Limits {
            max_texture_dimension_2d: 1234,
            max_buffer_size: 99,
            ..wgpu::Limits::default()
        };
        let rows = limit_rows(&limits);
        let get = |name: &str| rows.iter().find(|(n, _)| n == name).map(|(_, v)| *v);
        assert_eq!(get("max_texture_dimension_2d"), Some(1234));
        assert_eq!(get("max_buffer_size"), Some(99));
        assert_eq!(
            get("max_bind_groups"),
            Some(u64::from(limits.max_bind_groups))
        );
    }

    #[test]
    fn a_real_device_reports_itself() {
        let config = GpuConfig::from_env();
        let instance = make_instance(&config, None);
        let gpu = match pollster::block_on(request_gpu(&instance, None, None, &config)) {
            Ok(gpu) => gpu,
            Err(RenderError::NoAdapter(reason)) => {
                eprintln!("SKIPPED (no GPU adapter): {reason}");
                return;
            }
            Err(error) => panic!("device creation failed: {error}"),
        };
        let lines = gpu.report.lines();
        eprintln!(
            "{}",
            lines.join(
                "
"
            )
        );
        assert!(lines[0].starts_with("GPU: "), "{lines:?}");
        assert!(!gpu.report.adapter.is_empty());
        assert!(!gpu.report.backend.is_empty());
        let texture = gpu
            .report
            .limits
            .iter()
            .find(|(n, _)| n == "max_texture_dimension_2d")
            .map(|(_, v)| *v)
            .unwrap();
        assert!(
            texture >= 2048,
            "even the weakest device draws a 2048 texture: {texture}"
        );
        assert!(gpu.report.surface.is_none(), "offscreen has no surface");
        assert_eq!(
            gpu.report.msaa.len(),
            3,
            "one line per checked format: {lines:?}"
        );
        assert!(!gpu.lost.is_set(), "a fresh device is not lost");

        // The best tier that the adapter can give is the one that was used.
        let options = wgpu::RequestAdapterOptions {
            power_preference: config.power.to_wgpu(),
            force_fallback_adapter: gpu.report.is_software(),
            compatible_surface: None,
            ..Default::default()
        };
        let adapter = pollster::block_on(instance.request_adapter(&options)).unwrap();
        if wgpu::Limits::default().check_limits(&adapter.limits()) {
            assert_eq!(
                gpu.report.limit_tier,
                LimitTier::Default,
                "an adapter that can give the default limits is asked for them"
            );
        }
    }
}
