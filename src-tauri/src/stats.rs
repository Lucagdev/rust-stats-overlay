use serde::Serialize;
use std::sync::Mutex;
use sysinfo::{Networks, System};

#[derive(Debug, Clone, Serialize, Default)]
pub struct SystemStats {
    pub cpu_percent: f32,
    pub cpu_freq_ghz: f32,
    pub ram_percent: f32,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
    pub gpu_percent: Option<u32>,
    pub gpu_temp: Option<u32>,
    pub gpu_power_w: Option<u32>,
    pub gpu_clock_mhz: Option<u32>,
    pub vram_used_mb: Option<u32>,
    pub vram_total_mb: Option<u32>,
    pub disk_read_mb: f64,
    pub disk_write_mb: f64,
    pub net_down_mb: f64,
    pub net_up_mb: f64,
}

// ---------------------------------------------------------------------------
// Disk I/O via PDH (Performance Data Helper)
// Uses Windows on-demand counters, unlike IOCTL_DISK_PERFORMANCE which
// requires `diskperf -y` to be enabled (legacy approach).
// ---------------------------------------------------------------------------
#[cfg(target_os = "windows")]
mod disk_pdh {
    use std::mem::zeroed;
    use std::ptr::null;

    // PDH handles are isize on Windows (HANDLE = *mut c_void, but windows-sys does not
    // export PDH_HQUERY/PDH_HCOUNTER as types — we declare them manually.)
    type PdhHQuery = isize;
    type PdhHCounter = isize;

    #[repr(C)]
    union PdhFmtAnon {
        pub long_value: i32,
        pub double_value: f64,
        pub large_value: i64,
    }

    #[repr(C)]
    struct PdhFmtCounterValue {
        pub c_status: u32,
        pub value: PdhFmtAnon,
    }

    #[link(name = "pdh")]
    extern "system" {
        fn PdhOpenQueryW(src: *const u16, userdata: usize, query: *mut PdhHQuery) -> u32;
        fn PdhAddEnglishCounterW(query: PdhHQuery, path: *const u16, userdata: usize, counter: *mut PdhHCounter) -> u32;
        fn PdhCollectQueryData(query: PdhHQuery) -> u32;
        fn PdhGetFormattedCounterValue(counter: PdhHCounter, fmt: u32, ctype: *mut u32, value: *mut PdhFmtCounterValue) -> u32;
        fn PdhCloseQuery(query: PdhHQuery) -> u32;
    }

    const PDH_FMT_DOUBLE: u32 = 0x00000200;

    pub struct PdhDisk {
        query: PdhHQuery,
        counter_read: PdhHCounter,
        counter_write: PdhHCounter,
    }

    impl PdhDisk {
        pub fn new() -> Option<Self> {
            let mut query: PdhHQuery = 0;
            if unsafe { PdhOpenQueryW(null(), 0, &mut query) } != 0 {
                return None;
            }

            let mut add = |path: &str, counter: &mut PdhHCounter| -> bool {
                let wide: Vec<u16> = path.encode_utf16().chain([0]).collect();
                unsafe { PdhAddEnglishCounterW(query, wide.as_ptr(), 0, counter) == 0 }
            };

            let mut counter_read: PdhHCounter = 0;
            let mut counter_write: PdhHCounter = 0;

            if !add("\\PhysicalDisk(_Total)\\Disk Read Bytes/sec", &mut counter_read)
                || !add("\\PhysicalDisk(_Total)\\Disk Write Bytes/sec", &mut counter_write)
            {
                unsafe { PdhCloseQuery(query) };
                return None;
            }

            // First collection to initialize rate counters
            unsafe { PdhCollectQueryData(query) };

            Some(Self { query, counter_read, counter_write })
        }

        pub fn collect(&self) -> (f64, f64) {
            if unsafe { PdhCollectQueryData(self.query) } != 0 {
                return (0.0, 0.0);
            }
            (self.get_mb(self.counter_read), self.get_mb(self.counter_write))
        }

        fn get_mb(&self, counter: PdhHCounter) -> f64 {
            let mut val: PdhFmtCounterValue = unsafe { zeroed() };
            let status = unsafe {
                PdhGetFormattedCounterValue(counter, PDH_FMT_DOUBLE, std::ptr::null_mut(), &mut val)
            };
            // 0 = PDH_CSTATUS_VALID_DATA, 1 = PDH_CSTATUS_NEW_DATA (both are success)
            if status == 0 || status == 1 {
                (unsafe { val.value.double_value } / 1_048_576.0).max(0.0)
            } else {
                0.0
            }
        }
    }

    impl Drop for PdhDisk {
        fn drop(&mut self) {
            unsafe { PdhCloseQuery(self.query) };
        }
    }
}

pub struct StatsCollector {
    sys: System,
    networks: Networks,
    #[cfg(target_os = "windows")]
    disk_pdh: Option<disk_pdh::PdhDisk>,
    gpu: Option<crate::gpu::GpuMonitor>,
}

impl StatsCollector {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let networks = Networks::new_with_refreshed_list();

        Self {
            sys,
            networks,
            #[cfg(target_os = "windows")]
            disk_pdh: disk_pdh::PdhDisk::new(),
            gpu: crate::gpu::GpuMonitor::new().ok(),
        }
    }

    pub fn collect(&mut self) -> SystemStats {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.networks.refresh(false);

        let cpu_percent = self.sys.global_cpu_usage();

        let cpu_freq_ghz = {
            let cpus = self.sys.cpus();
            if cpus.is_empty() {
                0.0_f32
            } else {
                let avg_mhz =
                    cpus.iter().map(|c| c.frequency() as f64).sum::<f64>() / cpus.len() as f64;
                (avg_mhz / 1000.0) as f32
            }
        };

        let ram_total = self.sys.total_memory() as f64;
        let ram_used = self.sys.used_memory() as f64;
        let ram_percent = if ram_total > 0.0 {
            (ram_used / ram_total * 100.0) as f32
        } else {
            0.0
        };
        let ram_used_gb = (ram_used / 1_073_741_824.0) as f32;
        let ram_total_gb = (ram_total / 1_073_741_824.0) as f32;

        // Disk I/O via PDH — returns MB/s directly (PDH computes the rate)
        #[cfg(target_os = "windows")]
        let (disk_read_mb, disk_write_mb) = self
            .disk_pdh
            .as_ref()
            .map(|d| d.collect())
            .unwrap_or((0.0, 0.0));

        #[cfg(not(target_os = "windows"))]
        let (disk_read_mb, disk_write_mb) = (0.0, 0.0);

        // Network I/O — sysinfo returns delta since the last refresh()
        let mut net_rx: u64 = 0;
        let mut net_tx: u64 = 0;
        for (_, data) in &self.networks {
            net_rx += data.received();
            net_tx += data.transmitted();
        }
        let net_down_mb = net_rx as f64 / 1_048_576.0;
        let net_up_mb = net_tx as f64 / 1_048_576.0;

        // GPU stats via NVML
        let gpu = if let Some(ref mut gpu) = self.gpu {
            gpu.refresh()
        } else {
            crate::gpu::GpuStats::default()
        };

        SystemStats {
            cpu_percent,
            cpu_freq_ghz,
            ram_percent,
            ram_used_gb,
            ram_total_gb,
            gpu_percent: gpu.percent,
            gpu_temp: gpu.temp,
            gpu_power_w: gpu.power_w,
            gpu_clock_mhz: gpu.clock_mhz,
            vram_used_mb: gpu.vram_used_mb,
            vram_total_mb: gpu.vram_total_mb,
            disk_read_mb,
            disk_write_mb,
            net_down_mb,
            net_up_mb,
        }
    }
}

pub struct StatsState(pub Mutex<StatsCollector>);
