// ─── Disk Sensor ────────────────────────────────────────────────────────
// Reads disk usage from df command for mounted partitions.

use std::process::Command;

pub struct DiskInfo {
    pub mount: String,
    pub total_gib: f64,
    #[allow(dead_code)]
    pub used_gib: f64,
    #[allow(dead_code)]
    pub available_gib: f64,
    pub usage_pct: f64,
}

pub fn read_disks() -> Vec<DiskInfo> {
    let mut disks = parse_df_output();
    if let Some(swap) = read_swap() {
        disks.push(swap);
    }
    disks
}

fn parse_df_output() -> Vec<DiskInfo> {
    let output = Command::new("df")
        .args(&["--block-size=1G", "--output=target,size,used,avail,pcent", "--total"])
        .output()
        .ok();
    let mut disks = Vec::new();
    let Some(out) = output else { return disks };
    let text = String::from_utf8(out.stdout).unwrap_or_default();
    for line in text.lines().skip(1) {
        if let Some(disk) = parse_df_line(&line) {
            disks.push(disk);
        }
    }
    disks
}

fn parse_df_line(line: &str) -> Option<DiskInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }
    let mount = parts[0].to_string();
    if mount == "total" {
        return None;
    }
    if !is_mounted_partition(&mount) {
        return None;
    }
    let total = parse_gib(parts[1]);
    let used = parse_gib(parts[2]);
    let avail = parse_gib(parts[3]);
    let pct = parts[4].trim_end_matches('%').parse::<f64>().unwrap_or(0.0);
    Some(DiskInfo { mount, total_gib: total, used_gib: used, available_gib: avail, usage_pct: pct })
}

fn is_mounted_partition(mount: &str) -> bool {
    mount.starts_with("/dev/") || mount == "/"
        || mount.starts_with("/home") || mount.starts_with("/root")
        || mount.starts_with("/srv")
}

fn parse_gib(s: &str) -> f64 {
    s.trim_end_matches('G').parse::<f64>().unwrap_or(0.0)
}

fn read_swap() -> Option<DiskInfo> {
    let (total, used) = read_swap_meminfo()?;
    if total <= 0.0 {
        return None;
    }
    let pct = (used / total) * 100.0;
    Some(DiskInfo {
        mount: "swap".to_string(),
        total_gib: total,
        used_gib: used,
        available_gib: total - used,
        usage_pct: pct,
    })
}

fn read_swap_meminfo() -> Option<(f64, f64)> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total_kb = 0.0;
    let mut free_kb = 0.0;
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let val = parts[1].parse::<f64>().unwrap_or(0.0);
        match parts[0] {
            "SwapTotal:" => total_kb = val,
            "SwapFree:" => free_kb = val,
            _ => {}
        }
    }
    let total = total_kb / 1024.0 / 1024.0;
    let used = total - (free_kb / 1024.0 / 1024.0);
    if total > 0.0 {
        Some((total, used))
    } else {
        None
    }
}
