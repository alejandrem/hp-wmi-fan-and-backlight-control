// ─── NPU Usage Detector ─────────────────────────────────────────────────
// Detects AMD XDNA NPU presence and attempts to read usage metrics.
// Searches multiple locations where future NPU patches may expose data:
// - /dev/xdna* character devices
// - sysfs under amd-pmf, amdxdna, or hwmon
// - debugfs under amdxdna or npu
// - devfreq for NPU frequency-based usage estimation
// When a NPU patch adds metrics to any of these locations, this code
// will automatically detect and read them without modification.

/// Returns (npu_model_name, npu_usage_percent).
/// npu_usage_percent is None if the NPU has no available metrics yet.
pub fn read_npu_usage() -> (String, Option<f64>) {
    let detected = detect_npu_presence();
    let model = npu_model_name(&detected);
    let usage = npu_usage(&detected);
    (model, usage)
}

struct NpuDetection {
    has_driver: bool,
    has_dev: bool,
    pci_found: bool,
}

fn detect_npu_presence() -> NpuDetection {
    let has_driver = check_driver_loaded();
    let has_dev = check_xdna_device();
    let pci_found = check_pci_device();
    NpuDetection {
        has_driver,
        has_dev,
        pci_found,
    }
}

fn check_driver_loaded() -> bool {
    std::fs::read_to_string("/sys/module/amdxdna/initstate")
        .ok()
        .map(|s| s.trim() == "live")
        .unwrap_or(false)
}

fn check_xdna_device() -> bool {
    if let Ok(entries) = std::fs::read_dir("/dev") {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("xdna") {
                return true;
            }
        }
    }
    false
}

fn check_pci_device() -> bool {
    // AMD XDNA NPU vendor ID: 1530
    if let Ok(content) = std::fs::read_to_string("/proc/bus/pci/devices") {
        for line in content.lines() {
            if line.contains("1530") {
                return true;
            }
        }
    }
    false
}

fn npu_model_name(detected: &NpuDetection) -> String {
    if !detected.has_driver && !detected.pci_found && !detected.has_dev {
        return "N/A".to_string();
    }
    detect_model_from_cpuinfo()
        .unwrap_or("AMD XDNA".to_string())
}

fn detect_model_from_cpuinfo() -> Option<String> {
    let content = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    for line in content.lines() {
        if line.starts_with("model name") {
            let name = line.split(':').last().map(|s| s.trim()).unwrap_or("");
            if name.contains("8645") || name.contains("8040")
                || name.contains("8045") || name.contains("8000")
            {
                return Some("Ryzen AI".to_string());
            }
        }
    }
    None
}

fn npu_usage(_detected: &NpuDetection) -> Option<f64> {
    if let Some(val) = read_amd_pmf_usage() {
        return Some(val);
    }
    if let Some(val) = read_npu_hwmon_usage() {
        return Some(val);
    }
    if let Some(val) = read_npu_devfreq_usage() {
        return Some(val);
    }
    if let Some(val) = read_npu_debugfs_usage() {
        return Some(val);
    }
    None
}

fn read_amd_pmf_usage() -> Option<f64> {
    let paths = [
        "/sys/devices/platform/amd-pmf/npu_usage",
        "/sys/devices/platform/amd-pmf/usage",
    ];
    for path in &paths {
        if let Some(val) = read_sys_f64(path) {
            return Some(val);
        }
    }
    None
}

fn read_npu_hwmon_usage() -> Option<f64> {
    let entries = std::fs::read_dir("/sys/class/hwmon").ok()?;
    for entry in entries.filter_map(|e| e.ok()) {
        let name = std::fs::read_to_string(
            entry.path().join("name")
        ).ok()?;
        if name.trim().contains("npu") || name.trim().contains("xdna") {
            if let Some(val) = read_sys_f64(
                entry.path().join("in1_input").to_str().unwrap()
            ) {
                return Some(val);
            }
        }
    }
    None
}

fn read_npu_devfreq_usage() -> Option<f64> {
    let entries = std::fs::read_dir("/sys/class/devfreq").ok()?;
    for entry in entries.filter_map(|e| e.ok()) {
        let name = std::fs::read_to_string(
            entry.path().join("name")
        ).ok()?;
        if name.trim().contains("npu") || name.trim().contains("xdna") {
            let cur = read_sys_f64(
                entry.path().join("cur_freq").to_str().unwrap()
            );
            let max = read_sys_f64(
                entry.path().join("max_freq").to_str().unwrap()
            );
            if let (Some(c), Some(m)) = (cur, max) {
                if m > 0.0 {
                    return Some((c / m) * 100.0);
                }
            }
        }
    }
    None
}

fn read_npu_debugfs_usage() -> Option<f64> {
    let paths = [
        "/sys/kernel/debug/amdxdna/usage",
        "/sys/kernel/debug/npu/usage",
        "/sys/kernel/debug/dri/0/npu_usage",
    ];
    for path in &paths {
        if let Some(val) = read_sys_f64(path) {
            return Some(val);
        }
    }
    None
}

fn read_sys_f64(path: &str) -> Option<f64> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .and_then(|s| s.parse().ok())
}
