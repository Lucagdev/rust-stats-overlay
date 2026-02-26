use nvml_wrapper::{
    enum_wrappers::device::{Clock, TemperatureSensor},
    Nvml,
};

pub struct GpuStats {
    pub percent: Option<u32>,
    pub temp: Option<u32>,
    pub power_w: Option<u32>,
    pub clock_mhz: Option<u32>,
    pub vram_used_mb: Option<u32>,
    pub vram_total_mb: Option<u32>,
}

impl Default for GpuStats {
    fn default() -> Self {
        Self {
            percent: None,
            temp: None,
            power_w: None,
            clock_mhz: None,
            vram_used_mb: None,
            vram_total_mb: None,
        }
    }
}

pub struct GpuMonitor {
    nvml: Nvml,
}

impl GpuMonitor {
    pub fn new() -> Result<Self, String> {
        let nvml = Nvml::init().map_err(|e| format!("NVML init failed: {}", e))?;
        Ok(Self { nvml })
    }

    pub fn refresh(&mut self) -> GpuStats {
        let device = match self.nvml.device_by_index(0) {
            Ok(d) => d,
            Err(_) => return GpuStats::default(),
        };

        let percent = device.utilization_rates().ok().map(|u| u.gpu);
        let temp = device.temperature(TemperatureSensor::Gpu).ok();
        let power_w = device.power_usage().ok().map(|mw| mw / 1000);
        let clock_mhz = device.clock_info(Clock::Graphics).ok();
        let (vram_used_mb, vram_total_mb) = match device.memory_info() {
            Ok(mem) => (
                Some((mem.used / 1_048_576) as u32),
                Some((mem.total / 1_048_576) as u32),
            ),
            Err(_) => (None, None),
        };

        GpuStats { percent, temp, power_w, clock_mhz, vram_used_mb, vram_total_mb }
    }
}
