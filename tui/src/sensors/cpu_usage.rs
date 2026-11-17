// ─── CPU Usage Calculator ───────────────────────────────────────────────
// Reads /proc/stat twice with a short interval and calculates CPU usage %.
// Detects the number of logical cores and model name from /proc/cpuinfo.

/// Returns (total_cores, cpu_usage_percent, cpu_model_name).
/// cpu_usage_percent is 0..100 across all cores (average).
pub fn read_cpu_usage() -> (usize, f64, String) {
    let cores = count_cores();
    let usage = calculate_usage();
    let model = cpu_model();
    (cores, usage, model)
}

fn count_cores() -> usize {
    if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
        content.lines().filter(|l| l.starts_with("processor\t")).count()
    } else {
        1
    }
}

fn cpu_model() -> String {
    if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if line.starts_with("model name") {
                let name = line.split(':').last().map(|s| s.trim().to_string()).unwrap_or_default();
                // Shorten: "AMD Ryzen 5 8645HS w/ Radeon 760M Graphics" -> "Ryzen 5 8645HS"
                if let Some(short) = name.strip_prefix("AMD ") {
                    // Take up to "w/" or end
                    let end = short.find(" w/").unwrap_or(short.len());
                    return short[..end].to_string();
                }
                return name;
            }
        }
    }
    "CPU".into()
}

fn read_cpu_stat() -> Option<(u64, u64)> {
    let content = std::fs::read_to_string("/proc/stat").ok()?;
    let line = content.lines().find(|l| l.starts_with("cpu "))?;
    let fields: Vec<u64> = line.split_whitespace().skip(1).filter_map(|f| f.parse().ok()).collect();
    if fields.len() >= 4 {
        // user, nice, system, idle, iowait, irq, softirq, steal
        let used = fields[0] + fields[1] + fields[2]
            + fields.get(4).copied().unwrap_or(0) // iowait
            + fields.get(5).copied().unwrap_or(0) // irq
            + fields.get(6).copied().unwrap_or(0) // softirq
            + fields.get(7).copied().unwrap_or(0); // steal
        let total: u64 = fields.iter().sum();
        Some((used, total))
    } else {
        None
    }
}

fn calculate_usage() -> f64 {
    let s1 = match read_cpu_stat() {
        Some(v) => v,
        None => return 0.0,
    };
    std::thread::sleep(std::time::Duration::from_millis(500));
    let s2 = match read_cpu_stat() {
        Some(v) => v,
        None => return 0.0,
    };
    let delta_used = s2.0.saturating_sub(s1.0);
    let delta_total = s2.1.saturating_sub(s1.1);
    if delta_total > 0 {
        (delta_used as f64 / delta_total as f64) * 100.0
    } else {
        0.0
    }
}

/// Returns per-core usage percentages for each logical core.
/// Reads cpu0, cpu1, cpu2, ... from /proc/stat.
pub fn read_per_core_usage() -> Vec<f64> {
    let cores = count_cores();
    let snapshots1 = read_all_core_stats(cores);
    std::thread::sleep(std::time::Duration::from_millis(500));
    calc_per_core_usage(cores, snapshots1)
}

fn read_all_core_stats(cores: usize) -> Vec<(u64, u64)> {
    (0..cores).map(|i| read_core_stat(i).unwrap_or((0, 0))).collect()
}

fn calc_per_core_usage(
    cores: usize, snapshots1: Vec<(u64, u64)>,
) -> Vec<f64> {
    (0..cores).map(|i| {
        let s2 = read_core_stat(i).unwrap_or((0, 0));
        let s1 = snapshots1[i];
        calc_usage_delta(s1, s2)
    }).collect()
}

fn calc_usage_delta(s1: (u64, u64), s2: (u64, u64)) -> f64 {
    let delta_used = s2.0.saturating_sub(s1.0);
    let delta_total = s2.1.saturating_sub(s1.1);
    if delta_total > 0 {
        (delta_used as f64 / delta_total as f64) * 100.0
    } else {
        0.0
    }
}

fn read_core_stat(core: usize) -> Option<(u64, u64)> {
    let content = std::fs::read_to_string("/proc/stat").ok()?;
    let line = content.lines().find(|l| l.starts_with(&format!("cpu{}", core)))?;
    let fields: Vec<u64> = line.split_whitespace().skip(1).filter_map(|f| f.parse().ok()).collect();
    if fields.len() >= 4 {
        let used = fields[0] + fields[1] + fields[2]
            + fields.get(4).copied().unwrap_or(0)
            + fields.get(5).copied().unwrap_or(0)
            + fields.get(6).copied().unwrap_or(0)
            + fields.get(7).copied().unwrap_or(0);
        let total: u64 = fields.iter().sum();
        Some((used, total))
    } else {
        None
    }
}
