// ─── Power Readings (Watts) ─────────────────────────────────────────────
// Reads real-time power consumption in Watts for each component:
// - CPU: RAPL energy delta from powercap
// - iGPU: amdgpu hwmon power1_input
// - GPU: nvidia-smi power.draw
// - NPU: no sensor available yet

use std::process::Command;

pub fn read_cpu_power() -> Option<f64> {
    read_rapl_power()
}

fn read_file_with_sudo(path: &str) -> Option<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("echo '12345678' | sudo -S cat {}", path))
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn read_rapl_energy() -> Option<u64> {
    let path = "/sys/class/powercap/intel-rapl:0/energy_uj";
    // Try without sudo first, fallback to sudo
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(val) = content.trim().parse() {
            return Some(val);
        }
    }
    read_file_with_sudo(path)?.trim().parse().ok()
}

fn read_rapl_power() -> Option<f64> {
    let e1 = read_rapl_energy()?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    let e2 = read_rapl_energy()?;
    let delta_uj = e2.saturating_sub(e1);
    Some(delta_uj as f64 / 500_000.0)
}

pub fn read_igpu_power() -> Option<f64> {
    read_amdgpu_power()
}

fn find_amdgpu_hwmon() -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir("/sys/class/hwmon").ok()?;
    for entry in entries.filter_map(|e| e.ok()) {
        let name = std::fs::read_to_string(entry.path().join("name")).ok()?;
        if name.trim() == "amdgpu" {
            return Some(entry.path());
        }
    }
    None
}

fn read_amdgpu_power() -> Option<f64> {
    let path = find_amdgpu_hwmon()?;
    let content = std::fs::read_to_string(path.join("power1_input")).ok()?;
    let uw: f64 = content.trim().parse().ok()?;
    Some(uw / 1_000_000.0)
}

pub fn read_gpu_power() -> Option<f64> {
    let output = Command::new("nvidia-smi")
        .args(&["--query-gpu=power.draw", "--format=csv,noheader"])
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    // Output is "1.95 W\n" — strip non-numeric suffix
    let stripped = text.trim().trim_end_matches(|c: char| !c.is_ascii_digit() && c != '.');
    let watts: f64 = stripped.parse().ok()?;
    Some(watts)
}

pub fn read_npu_power() -> Option<f64> {
    None
}

/// Returns total system power in Watts by summing CPU + iGPU + GPU power.
/// Falls back to battery power if available.
pub fn read_total_power() -> Option<f64> {
    let cpu = read_cpu_power().unwrap_or(0.0);
    let igpu = read_igpu_power().unwrap_or(0.0);
    let gpu = read_gpu_power().unwrap_or(0.0);
    let total = cpu + igpu + gpu;
    if total > 0.0 {
        Some(total)
    } else {
        // Fallback: read battery power from sysfs
        read_battery_power()
    }
}

fn read_battery_power() -> Option<f64> {
    // voltage_now in microvolts, current_now in microamps
    let voltage = read_sys_u64("/sys/class/power_supply/BAT0/voltage_now")?;
    let current = read_sys_u64("/sys/class/power_supply/BAT0/current_now")?;
    // Power = V * I, convert from uV*uA to W
    let power_w = (voltage as f64 * current as f64) / 1_000_000_000.0;
    Some(power_w.abs())
}

fn read_sys_u64(path: &str) -> Option<u64> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}
