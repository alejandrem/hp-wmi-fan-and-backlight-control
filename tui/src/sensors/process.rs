// ─── Process Sensor ─────────────────────────────────────────────────────
// Reads every visible process from /proc/[pid]/stat and /proc/[pid]/comm.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static CPU_SNAPSHOT: OnceLock<Mutex<(u64, HashMap<u32, (u64, u64)>)>> = OnceLock::new();
static USER_NAMES: OnceLock<HashMap<u32, String>> = OnceLock::new();

pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub user: String,
    #[allow(dead_code)]
    pub mem_pct: f64,
    pub cpu_pct: f64,
    pub mem_mb: f64,
}

pub fn read_processes() -> Vec<ProcessInfo> {
    let total_mem = get_total_mem_kb();
    collect_processes(total_mem)
}

fn collect_processes(total_mem: f64) -> Vec<ProcessInfo> {
    let total_ticks = read_total_cpu_ticks();
    let cpu_count = get_num_cpus().max(1) as f64;
    let snapshot = CPU_SNAPSHOT.get_or_init(|| Mutex::new((0, HashMap::new())));
    let mut previous = snapshot.lock().unwrap_or_else(|e| e.into_inner());
    let total_delta = total_ticks.saturating_sub(previous.0);
    let mut processes = Vec::new();
    let mut current = HashMap::new();
    let Ok(entries) = std::fs::read_dir("/proc") else { return processes };
    for entry in entries.filter_map(|e| e.ok()) {
        let Some(pid) = parse_pid(&entry) else { continue };
        let Some(stat) = read_proc_stat(pid) else { continue };
        let name = read_proc_comm(pid).unwrap_or_else(|| "?".into());
        let user = get_user_for_pid(pid).unwrap_or_else(|| "?".into());
        let ticks = stat.utime.saturating_add(stat.stime);
        let cpu_pct = previous.1.get(&pid)
            .filter(|(start, _)| *start == stat.starttime)
            .map(|(_, old_ticks)| if total_delta == 0 { 0.0 } else {
                ticks.saturating_sub(*old_ticks) as f64 / total_delta as f64 * cpu_count * 100.0
            }).unwrap_or(0.0);
        current.insert(pid, (stat.starttime, ticks));
        let mem_kb = calc_mem_kb(&stat);
        let mem_pct = if total_mem > 0.0 { mem_kb / total_mem * 100.0 } else { 0.0 };
        let info = ProcessInfo { pid, name, user, mem_pct, cpu_pct, mem_mb: mem_kb / 1024.0 };
        processes.push(info);
    }
    previous.0 = total_ticks;
    previous.1 = current;
    processes
}

fn parse_pid(entry: &std::fs::DirEntry) -> Option<u32> {
    let pid_str = entry.file_name().to_string_lossy().to_string();
    pid_str.parse::<u32>().ok()
}

fn read_total_cpu_ticks() -> u64 {
    std::fs::read_to_string("/proc/stat").ok()
        .and_then(|text| text.lines().find(|line| line.starts_with("cpu ")).map(str::to_owned))
        .map(|line| line.split_whitespace().skip(1).filter_map(|v| v.parse::<u64>().ok()).sum())
        .unwrap_or(0)
}

fn calc_mem_kb(stat: &ProcStat) -> f64 {
    let page_size = get_page_size();
    (stat.rss as f64 * page_size as f64) / 1024.0
}

struct ProcStat {
    utime: u64,
    stime: u64,
    starttime: u64,
    rss: u64,
}

fn read_proc_stat(pid: u32) -> Option<ProcStat> {
    let content = std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    let fields = parse_stat_fields(&content)?;
    build_proc_stat(&fields)
}

fn parse_stat_fields(content: &str) -> Option<Vec<u64>> {
    let close_paren = content.rfind(')')?;
    let fields: Vec<u64> = content[close_paren + 2..]
        .split_whitespace()
        .filter_map(|f| f.parse().ok())
        .collect();
    // The state token (field 3) is non-numeric and is skipped. The remaining
    // vector starts at field 4, so utime/stime/starttime/rss are 10/11/18/20.
    if fields.len() >= 21 {
        Some(fields)
    } else {
        None
    }
}

fn build_proc_stat(fields: &[u64]) -> Option<ProcStat> {
    if fields.len() >= 21 {
        Some(ProcStat {
            utime: fields[10],
            stime: fields[11],
            starttime: fields[18],
            rss: fields[20],
        })
    } else {
        None
    }
}

fn read_proc_comm(pid: u32) -> Option<String> {
    std::fs::read_to_string(format!("/proc/{}/comm", pid))
        .ok()
        .map(|s| s.trim().to_string())
}

fn get_total_mem_kb() -> f64 {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|content| {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        return parts[1].parse::<f64>().ok();
                    }
                }
            }
            None
        })
        .unwrap_or(1.0)
}

fn get_num_cpus() -> usize {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .map(|content| content.lines().filter(|l| l.starts_with("processor\t")).count())
        .unwrap_or(1)
}

fn get_page_size() -> u64 {
    unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 }
}

fn get_user_for_pid(pid: u32) -> Option<String> {
    let content = std::fs::read_to_string(format!("/proc/{}/status", pid)).ok()?;
    for line in content.lines() {
        if line.starts_with("Uid:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let uid = parts[1].parse::<u32>().ok()?;
                return Some(get_username(uid));
            }
        }
    }
    None
}

fn get_username(uid: u32) -> String {
    let users = USER_NAMES.get_or_init(|| {
        let mut users = HashMap::new();
        if let Ok(content) = std::fs::read_to_string("/etc/passwd") {
            for line in content.lines() {
                let mut parts = line.split(':');
                let (Some(name), Some(_), Some(uid)) = (parts.next(), parts.next(), parts.next()) else {
                    continue;
                };
                if let Ok(uid) = uid.parse::<u32>() {
                    users.insert(uid, name.to_owned());
                }
            }
        }
        users
    });
    users.get(&uid).cloned().unwrap_or_else(|| uid.to_string())
}
