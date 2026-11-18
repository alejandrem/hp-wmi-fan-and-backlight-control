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
    pub fn empty() -> Self {
        Self {
            cpu_temp: None, igpu_temp: None, gpu_temp: None, nvme_temp: None,
            acpi_temp: None, chipset_temp: None, battery_pct: None,
            battery_status: None, battery_time_remaining: None,
            fan1_rpm: None, fan2_rpm: None, cpu_cores: 0, cpu_usage: 0.0,
            cpu_model: "CPU".into(), per_core_usage: Vec::new(),
            igpu_usage: None, igpu_model: "iGPU".into(), gpu_usage: None,
            gpu_mem_usage: None, gpu_model: "GPU".into(), npu_usage: None,
            npu_model: "NPU".into(), cpu_power: None, igpu_power: None,
            gpu_power: None, npu_power: None, total_power: None,
            memory: MemoryInfo {
                total_gib: 0.0, used_gib: 0.0, available_gib: 0.0,
                cached_gib: 0.0, free_gib: 0.0,
            },
            disks: Vec::new(), network: None, processes: Vec::new(),
            uptime: "00:00:00".into(), load_avg: [0.0; 3],
        }
    }

    pub fn read() -> Self {
        let temps = read_temperatures();
        smoothing::record_temperature_usage([
            temps.cpu, temps.igpu, temps.gpu, temps.nvme, temps.acpi, temps.chipset,
        ]);
        let battery = read_battery();
        let fans = read_fans();
        let cpu = aggregate::read_cpu_data();
        let gpu = aggregate::read_gpu_data();
        let igpu = aggregate::read_igpu_data();
        let npu = npu_usage::read_npu_usage();
        let power = aggregate::read_power_data();
        let smoothed = aggregate::apply_smoothing(cpu, igpu, gpu, power);
        smoothing::record_usage_histories(
            smoothed.cpu_usage,
            &smoothed.per_core,
            smoothed.igpu_usage,
            smoothed.gpu_usage,
        );
        let system = aggregate::read_system_data();
        smoothing::record_memory_usage(
            system.memory.used_gib,
            system.memory.available_gib,
            system.memory.cached_gib,
            system.memory.free_gib,
            system.memory.total_gib,
        );
        smoothing::record_disk_usage(
            &system.disks.iter().map(|disk| (disk.mount.clone(), disk.usage_pct)).collect::<Vec<_>>(),
        );

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
        cpu: read_hwmon_temp(&["tctl", "tdie", "cpu", "package", "coretemp"]),
        igpu: read_hwmon_temp(&["amdgpu", "gpu"]),
        gpu: read_nvidia_temp(),
        nvme: read_hwmon_temp(&["nvme"]),
        acpi: read_hwmon_temp(&["acpitz", "acpi"]),
        chipset: read_hwmon_temp(&["pch", "chipset"]),
    }
}

// hwmonN numbering changes across boots and kernel/device configurations.
// Resolve a sensor by its stable hwmon name or temp label instead.
fn read_hwmon_temp(needles: &[&str]) -> Option<f64> {
    let entries = std::fs::read_dir("/sys/class/hwmon").ok()?;
    for entry in entries.filter_map(Result::ok) {
        let dir = entry.path();
        let name = std::fs::read_to_string(dir.join("name")).unwrap_or_default();
        let name = name.trim().to_ascii_lowercase();
        let Ok(files) = std::fs::read_dir(&dir) else { continue };
        for file in files.filter_map(Result::ok) {
            let filename = file.file_name().to_string_lossy().to_string();
            if !filename.starts_with("temp") || !filename.ends_with("_input") { continue; }
            let label_path = dir.join(filename.replace("_input", "_label"));
            let label = std::fs::read_to_string(label_path).unwrap_or_default().trim().to_ascii_lowercase();
            if needles.iter().any(|n| name.contains(n) || label.contains(n)) {
                if let Some(value) = read_sys_f64(file.path().to_str()?) {
                    return Some(value / 1000.0);
                }
            }
        }
    }
    None
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
        fan1: read_hwmon_fan(1),
        fan2: read_hwmon_fan(2),
    }
}

fn read_hwmon_fan(number: u8) -> Option<i64> {
    let entries = std::fs::read_dir("/sys/class/hwmon").ok()?;
    let file = format!("fan{number}_input");
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path().join(&file);
        if let Some(value) = path.to_str().and_then(read_sys_i64) {
            return Some(value);
        }
    }
    None
}

