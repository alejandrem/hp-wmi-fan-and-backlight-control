// ─── iGPU (AMD) Usage Calculator ────────────────────────────────────────
// Reads AMD iGPU usage from hwmon (amdgpu) using sclk frequency and power.
// The Radeon 760M (RDNA3) has a max sclk of ~2800 MHz.
// Usage is estimated as: (current_sclk / max_sclk) * 100, clamped 0-100.

fn read_sys(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_sys_u64(path: &str) -> Option<u64> {
    read_sys(path).and_then(|s| s.parse().ok())
}

/// Max sclk for Radeon 760M (RDNA3 integrated) in Hz.
const MAX_SCLK_HZ: u64 = 2_800_000_000;

/// Returns the iGPU model name (e.g., "Radeon 760M").
pub fn igpu_model_name() -> String {
    // Try to get the device name from sysfs
    if let Some(device) = read_sys("/sys/class/drm/card0/device/name") {
        // Format: "AMD Radeon Graphics" or similar
        let name = device.trim();
        if name.contains("Radeon") {
            // Extract "Radeon 760M" or similar
            if let Some(start) = name.find("Radeon") {
                return name[start..].to_string();
            }
        }
        return name.to_string();
    }
    // Fallback: check from cpuinfo for the iGPU part
    if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if line.starts_with("model name") {
                let full = line.split(':').last().map(|s| s.trim()).unwrap_or("");
                if let Some(with) = full.find(" w/ ") {
                    let rest = &full[with + 4..];
                    // "Radeon 760M Graphics" -> "Radeon 760M"
                    if let Some(graphics) = rest.find(" Graphics") {
                        return rest[..graphics].to_string();
                    }
                    return rest.to_string();
                }
            }
        }
    }
    "iGPU".into()
}

/// Returns the iGPU usage percentage (0..100) if available.
/// Uses sclk frequency from amdgpu hwmon to estimate GPU utilization.
pub fn read_igpu_usage() -> Option<f64> {
    // Find the amdgpu hwmon directory by name
    let hwmon_path = find_amdgpu_hwmon()?;

    // Method 1: Use sclk frequency to estimate usage
    // The sclk (shader clock) is proportional to GPU activity.
    // At idle, sclk is ~200-400 MHz; at full load, ~2800 MHz.
    if let Some(sclk_hz) = read_sys_u64(&format!("{}/freq1_input", hwmon_path)) {
        let usage = (sclk_hz as f64 / MAX_SCLK_HZ as f64) * 100.0;
        return Some(usage.clamp(0.0, 100.0));
    }

    // Method 2: Use power consumption as fallback
    // Radeon 760M TDP is ~25W max. Estimate usage from power draw.
    if let Some(power_uw) = read_sys_u64(&format!("{}/power1_input", hwmon_path)) {
        let power_w = power_uw as f64 / 1_000_000.0;
        let usage = (power_w / 25.0) * 100.0;
        return Some(usage.clamp(0.0, 100.0));
    }

    None
}

/// Finds the hwmon path for the amdgpu device by matching the name.
fn find_amdgpu_hwmon() -> Option<String> {
    let hwmon_base = std::path::Path::new("/sys/class/hwmon");
    let entries = std::fs::read_dir(hwmon_base).ok()?;

    for entry in entries.filter_map(|e| e.ok()) {
        let name_path = entry.path().join("name");
        if let Ok(name) = std::fs::read_to_string(&name_path) {
            if name.trim() == "amdgpu" {
                return Some(entry.path().to_string_lossy().to_string());
            }
        }
    }
    None
}
