// ─── Memory Sensor ──────────────────────────────────────────────────────
// Reads /proc/meminfo for RAM usage statistics.

pub struct MemoryInfo {
    pub total_gib: f64,
    pub used_gib: f64,
    pub available_gib: f64,
    pub cached_gib: f64,
    pub free_gib: f64,
}

pub fn read_memory() -> MemoryInfo {
    let content = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mem = parse_meminfo(&content);
    let used = calc_used(&mem);
    MemoryInfo {
        total_gib: mem.total / 1024.0,
        used_gib: used / 1024.0,
        available_gib: mem.available / 1024.0,
        cached_gib: (mem.cached + mem.buffers + mem.sreclaimable) / 1024.0,
        free_gib: mem.free / 1024.0,
    }
}

struct MemValues {
    total: f64,
    free: f64,
    available: f64,
    cached: f64,
    buffers: f64,
    sreclaimable: f64,
}

fn parse_meminfo(content: &str) -> MemValues {
    let mut m = MemValues {
        total: 0.0, free: 0.0, available: 0.0,
        cached: 0.0, buffers: 0.0, sreclaimable: 0.0,
    };
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let key = parts[0].trim_end_matches(':');
        let val = parts[1].parse::<f64>().unwrap_or(0.0) / 1024.0;
        match key {
            "MemTotal" => m.total = val,
            "MemFree" => m.free = val,
            "MemAvailable" => m.available = val,
            "Cached" => m.cached = val,
            "Buffers" => m.buffers = val,
            "SReclaimable" => m.sreclaimable = val,
            _ => {}
        }
    }
    m
}

fn calc_used(mem: &MemValues) -> f64 {
    let used = mem.total - mem.free - mem.cached - mem.buffers - mem.sreclaimable;
    if used > 0.0 { used } else { mem.total - mem.available }
}
