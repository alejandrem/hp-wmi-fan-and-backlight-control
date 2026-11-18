// ─── CPU Usage Calculator ───────────────────────────────────────────────
// Calculates CPU usage from /proc/stat counter differences between refreshes.
// Detects the number of logical cores and model name from /proc/cpuinfo.

use std::sync::{Mutex, OnceLock};

static CPU_PREVIOUS: OnceLock<Mutex<Option<(u64, u64)>>> = OnceLock::new();
static CORE_PREVIOUS: OnceLock<Mutex<Option<Vec<(u64, u64)>>>> = OnceLock::new();
static CPU_INFO: OnceLock<(usize, String)> = OnceLock::new();

/// Returns (total_cores, cpu_usage_percent, cpu_model_name).
/// cpu_usage_percent is 0..100 across all cores (average).
pub fn read_cpu_usage() -> (usize, f64, String) {
    let cores = count_cores();
    let usage = calculate_usage();
    let model = cpu_model();
    (cores, usage, model)
}

fn count_cores() -> usize {
    cpu_info().0
}

fn cpu_model() -> String {
    cpu_info().1.clone()
}

fn cpu_info() -> &'static (usize, String) {
    CPU_INFO.get_or_init(|| {
        let content = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
        let cores = content.lines().filter(|line| line.starts_with("processor\t")).count().max(1);
        let model = content.lines()
            .find(|line| line.starts_with("model name"))
            .and_then(|line| line.split(':').nth(1))
            .map(str::trim)
            .map(|name| {
                let name = name.strip_prefix("AMD ").unwrap_or(name);
                name.split(" w/").next().unwrap_or(name).to_string()
            })
            .unwrap_or_else(|| "CPU".into());
        (cores, model)
    })
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
    let Some(current) = read_cpu_stat() else { return 0.0 };
    let previous = CPU_PREVIOUS.get_or_init(|| Mutex::new(None));
    let mut previous = previous.lock().unwrap_or_else(|e| e.into_inner());
    let usage = previous.map(|old| calc_usage_delta(old, current)).unwrap_or(0.0);
    *previous = Some(current);
    usage
}

/// Returns per-core usage percentages for each logical core.
/// Reads cpu0, cpu1, cpu2, ... together from one /proc/stat snapshot.
pub fn read_per_core_usage() -> Vec<f64> {
    let cores = count_cores();
    let current = read_all_core_stats(cores);
    let previous = CORE_PREVIOUS.get_or_init(|| Mutex::new(None));
    let mut previous = previous.lock().unwrap_or_else(|e| e.into_inner());
    let usage = match previous.as_ref() {
        Some(old) => calc_per_core_usage(cores, old, &current),
        None => vec![0.0; cores],
    };
    *previous = Some(current);
    usage
}

fn read_all_core_stats(cores: usize) -> Vec<(u64, u64)> {
    let Some(content) = std::fs::read_to_string("/proc/stat").ok() else {
        return vec![(0, 0); cores];
    };
    let mut stats = vec![(0, 0); cores];
    for line in content.lines() {
        let Some(number) = line.strip_prefix("cpu") else { continue };
        let Ok(index) = number.split_whitespace().next().unwrap_or("").parse::<usize>() else { continue };
        if index < cores {
            stats[index] = parse_cpu_line(line).unwrap_or((0, 0));
        }
    }
    stats
}

fn calc_per_core_usage(
    cores: usize,
    snapshots1: &[(u64, u64)],
    snapshots2: &[(u64, u64)],
) -> Vec<f64> {
    (0..cores).map(|i| {
        let Some(&s2) = snapshots2.get(i) else { return 0.0 };
        let Some(&s1) = snapshots1.get(i) else { return 0.0 };
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

fn parse_cpu_line(line: &str) -> Option<(u64, u64)> {
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
