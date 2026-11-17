// ─── Sensor Data Model ──────────────────────────────────────────────────
// Aggregates all hardware sensor readings into a single struct.

use std::process::Command;

mod aggregate;
mod cpu_usage;
mod disk;
mod gpu_usage;
mod igpu_usage;
mod memory;
mod network;
mod npu_usage;
mod power_readings;
mod process;
pub mod smoothing;

pub use disk::DiskInfo;
pub use memory::MemoryInfo;
pub use network::NetworkInfo;
pub use process::ProcessInfo;

pub struct Sensors {
    pub cpu_temp: Option<f64>,
    pub igpu_temp: Option<f64>,
    pub gpu_temp: Option<f64>,
    pub nvme_temp: Option<f64>,
    pub acpi_temp: Option<f64>,
    pub chipset_temp: Option<f64>,
    pub battery_pct: Option<i64>,
    #[allow(dead_code)]
    pub battery_status: Option<String>,
    pub battery_time_remaining: Option<String>,
    pub fan1_rpm: Option<i64>,
    pub fan2_rpm: Option<i64>,
    pub cpu_cores: usize,
    pub cpu_usage: f64,
    pub cpu_model: String,
    pub per_core_usage: Vec<f64>,
    pub igpu_usage: Option<f64>,
    pub igpu_model: String,
    pub gpu_usage: Option<f64>,
    #[allow(dead_code)]
    pub gpu_mem_usage: Option<f64>,
    pub gpu_model: String,
    #[allow(dead_code)]
    pub npu_usage: Option<f64>,
    #[allow(dead_code)]
    pub npu_model: String,
    pub cpu_power: Option<f64>,
    pub igpu_power: Option<f64>,
    pub gpu_power: Option<f64>,
    #[allow(dead_code)]
    pub npu_power: Option<f64>,
    pub total_power: Option<f64>,
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
    pub network: Option<NetworkInfo>,
    pub processes: Vec<ProcessInfo>,
    pub uptime: String,
    pub load_avg: [f64; 3],
}

impl Sensors {
    pub fn read() -> Self {
        let temps = read_temperatures();
        let battery = read_battery();
        let fans = read_fans();
        let cpu = aggregate::read_cpu_data();
        let gpu = aggregate::read_gpu_data();
        let igpu = aggregate::read_igpu_data();
        let npu = npu_usage::read_npu_usage();
        let power = aggregate::read_power_data();
        let smoothed = aggregate::apply_smoothing(cpu, igpu, gpu, power);
        let system = aggregate::read_system_data();

        Sensors {
            cpu_temp: temps.cpu,
            igpu_temp: temps.igpu,
            gpu_temp: temps.gpu,
            nvme_temp: temps.nvme,
            acpi_temp: temps.acpi,
            chipset_temp: temps.chipset,
            battery_pct: battery.pct,
            battery_status: battery.status,
            battery_time_remaining: battery.time_remaining,
            fan1_rpm: fans.fan1,
            fan2_rpm: fans.fan2,
            cpu_cores: smoothed.cores,
            cpu_usage: smoothed.cpu_usage,
            cpu_model: smoothed.cpu_model,
            per_core_usage: smoothed.per_core,
            igpu_usage: smoothed.igpu_usage,
            igpu_model: smoothed.igpu_model,
            gpu_usage: smoothed.gpu_usage,
            gpu_mem_usage: smoothed.gpu_mem_usage,
            gpu_model: smoothed.gpu_model,
            npu_usage: npu.1,
            npu_model: npu.0,
            cpu_power: smoothed.cpu_power,
            igpu_power: smoothed.igpu_power,
            gpu_power: smoothed.gpu_power,
            npu_power: smoothed.npu_power,
            total_power: smoothed.total_power,
            memory: system.memory,
            disks: system.disks,
            network: system.network,
            processes: system.processes,
            uptime: system.uptime,
            load_avg: system.load_avg,
        }
    }
}

fn read_sys(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_sys_i64(path: &str) -> Option<i64> {
    read_sys(path).and_then(|s| s.parse().ok())
}

fn read_sys_f64(path: &str) -> Option<f64> {
    read_sys(path).and_then(|s| s.parse().ok())
}

fn read_nvidia_temp() -> Option<f64> {
    let output = Command::new("nvidia-smi")
        .args(&["--query-gpu=temperature.gpu", "--format=csv,noheader"])
        .output()
        .ok()?;
    String::from_utf8(output.stdout)
        .ok()?
        .trim()
        .parse()
        .ok()
}

struct Temps {
    cpu: Option<f64>,
    igpu: Option<f64>,
    gpu: Option<f64>,
    nvme: Option<f64>,
    acpi: Option<f64>,
    chipset: Option<f64>,
}

fn read_temperatures() -> Temps {
    Temps {
        cpu: read_sys_f64("/sys/class/hwmon/hwmon4/temp1_input").map(|v| v / 1000.0),
        igpu: read_sys_f64("/sys/class/hwmon/hwmon9/temp1_input").map(|v| v / 1000.0),
        gpu: read_nvidia_temp(),
        nvme: read_sys_f64("/sys/class/hwmon/hwmon3/temp1_input").map(|v| v / 1000.0),
        acpi: read_sys_f64("/sys/class/hwmon/hwmon1/temp1_input").map(|v| v / 1000.0),
        chipset: read_sys_f64("/sys/class/hwmon/hwmon5/temp1_input").map(|v| v / 1000.0),
    }
}

struct Battery {
    pct: Option<i64>,
    status: Option<String>,
    time_remaining: Option<String>,
}

fn read_battery() -> Battery {
    let time_sec = read_sys_i64("/sys/class/power_supply/BAT0/time_to_empty_now");
    let time_remaining = time_sec.and_then(|sec| {
        if sec > 0 {
            let hours = sec / 3600;
            let mins = (sec % 3600) / 60;
            Some(format!("{:02}:{:02}", hours, mins))
        } else {
            None
        }
    });

    Battery {
        pct: read_sys_i64("/sys/class/power_supply/BAT0/capacity"),
        status: read_sys("/sys/class/power_supply/BAT0/status"),
        time_remaining,
    }
}

struct Fans {
    fan1: Option<i64>,
    fan2: Option<i64>,
}

fn read_fans() -> Fans {
    Fans {
        fan1: read_sys_i64("/sys/class/hwmon/hwmon7/fan1_input"),
        fan2: read_sys_i64("/sys/class/hwmon/hwmon7/fan2_input"),
    }
}

