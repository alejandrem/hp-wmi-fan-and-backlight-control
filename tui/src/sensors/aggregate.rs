// ─── Sensor Aggregation ─────────────────────────────────────────────────
// Helper structs and functions for Sensors::read().

use crate::sensors::{
    cpu_usage, disk, gpu_usage, igpu_usage, memory, network,
    power_readings, process, smoothing,
};
use crate::sensors::{DiskInfo, MemoryInfo, NetworkInfo, ProcessInfo};

pub struct RawCpu {
    pub cores: usize,
    pub usage: f64,
    pub model: String,
    pub per_core: Vec<f64>,
}

pub fn read_cpu_data() -> RawCpu {
    let (cores, usage, model) = cpu_usage::read_cpu_usage();
    let per_core = cpu_usage::read_per_core_usage();
    RawCpu { cores, usage, model, per_core }
}

pub struct RawGpu {
    pub usage: Option<f64>,
    pub mem: Option<f64>,
    pub model: String,
}

pub fn read_gpu_data() -> RawGpu {
    RawGpu {
        usage: gpu_usage::read_gpu_usage(),
        mem: gpu_usage::read_gpu_mem_usage(),
        model: gpu_usage::gpu_model_name(),
    }
}

pub struct RawIgpu {
    pub usage: Option<f64>,
    pub model: String,
}

pub fn read_igpu_data() -> RawIgpu {
    RawIgpu {
        usage: igpu_usage::read_igpu_usage(),
        model: igpu_usage::igpu_model_name(),
    }
}

pub struct RawPower {
    pub cpu: Option<f64>,
    pub igpu: Option<f64>,
    pub gpu: Option<f64>,
    pub npu: Option<f64>,
    pub total: Option<f64>,
}

pub fn read_power_data() -> RawPower {
    RawPower {
        cpu: power_readings::read_cpu_power(),
        igpu: power_readings::read_igpu_power(),
        gpu: power_readings::read_gpu_power(),
        npu: power_readings::read_npu_power(),
        total: power_readings::read_total_power(),
    }
}

pub struct Smoothed {
    pub cores: usize,
    pub cpu_usage: f64,
    pub cpu_model: String,
    pub per_core: Vec<f64>,
    pub igpu_usage: Option<f64>,
    pub igpu_model: String,
    pub gpu_usage: Option<f64>,
    pub gpu_mem_usage: Option<f64>,
    pub gpu_model: String,
    pub cpu_power: Option<f64>,
    pub igpu_power: Option<f64>,
    pub gpu_power: Option<f64>,
    pub npu_power: Option<f64>,
    pub total_power: Option<f64>,
}

pub fn apply_smoothing(
    cpu: RawCpu, igpu: RawIgpu, gpu: RawGpu, power: RawPower,
) -> Smoothed {
    Smoothed {
        cores: cpu.cores,
        cpu_usage: smoothing::smooth_cpu_usage(cpu.usage),
        cpu_model: cpu.model,
        per_core: cpu.per_core,
        igpu_usage: igpu.usage.map(|v| smoothing::smooth_igpu_usage(v)),
        igpu_model: igpu.model,
        gpu_usage: gpu.usage.map(|v| smoothing::smooth_gpu_usage(v)),
        gpu_mem_usage: gpu.mem.map(|v| smoothing::smooth_gpu_mem_usage(v)),
        gpu_model: gpu.model,
        cpu_power: power.cpu.map(|v| smoothing::smooth_cpu_power(v)),
        igpu_power: power.igpu.map(|v| smoothing::smooth_igpu_power(v)),
        gpu_power: power.gpu.map(|v| smoothing::smooth_gpu_power(v)),
        npu_power: power.npu,
        total_power: power.total.map(|v| smoothing::smooth_total_power(v)),
    }
}

pub struct SystemData {
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
    pub network: Option<NetworkInfo>,
    pub processes: Vec<ProcessInfo>,
    pub uptime: String,
    pub load_avg: [f64; 3],
}

pub fn read_system_data() -> SystemData {
    SystemData {
        memory: memory::read_memory(),
        disks: disk::read_disks(),
        network: network::read_network(),
        processes: process::read_processes(),
        uptime: read_uptime(),
        load_avg: read_load_avg(),
    }
}

fn read_uptime() -> String {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| {
            let secs: f64 = s.split_whitespace().next()?.parse().ok()?;
            let days = secs / 86400.0;
            let hours = (secs % 86400.0) / 3600.0;
            let mins = (secs % 3600.0) / 60.0;
            Some(format!("{:02}:{:02}:{:02}", days as u64, hours as u64, mins as u64))
        })
        .unwrap_or_else(|| "00:00:00".to_string())
}

fn read_load_avg() -> [f64; 3] {
    std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|s| {
            let parts: Vec<f64> = s.split_whitespace()
                .take(3)
                .filter_map(|p| p.parse().ok())
                .collect();
            if parts.len() == 3 {
                Some([parts[0], parts[1], parts[2]])
            } else {
                None
            }
        })
        .unwrap_or([0.0, 0.0, 0.0])
}
