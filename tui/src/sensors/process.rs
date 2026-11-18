// ─── Process Sensor ─────────────────────────────────────────────────────
// Reads every visible process from /proc/[pid]/stat and /proc/[pid]/comm.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static CPU_SNAPSHOT: OnceLock<Mutex<(u64, HashMap<u32, (u64, u64)>)>> = OnceLock::new();
static USER_NAMES: OnceLock<HashMap<u32, String>> = OnceLock::new();
static PAGE_SIZE: OnceLock<u64> = OnceLock::new();
static PROCESS_CACHE: OnceLock<Mutex<(Option<Instant>, Vec<ProcessInfo>)>> = OnceLock::new();
const PROCESS_CACHE_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone)]
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
    let cache = PROCESS_CACHE.get_or_init(|| Mutex::new((None, Vec::new())));
    let mut cache = cache.lock().unwrap_or_else(|error| error.into_inner());
    if cache.0.is_some_and(|last_read| last_read.elapsed() < PROCESS_CACHE_INTERVAL) {
        return cache.1.clone();
    }
    let total_mem = get_total_mem_kb();
    let processes = collect_processes(total_mem);
    cache.0 = Some(Instant::now());
    cache.1.clone_from(&processes);
    processes
}

fn collect_processes(total_mem: f64) -> Vec<ProcessInfo> {
    let total_ticks = read_total_cpu_ticks();
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
                // Normalize to total system capacity, matching btop's default
                // process percentage instead of reporting one core as 100%.
                ticks.saturating_sub(*old_ticks) as f64 / total_delta as f64 * 100.0
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
    let page_size = *PAGE_SIZE.get_or_init(|| unsafe { libc::sysconf(libc::_SC_PAGESIZE) as u64 });
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
    let fields: Vec<&str> = content[close_paren + 2..].split_whitespace().collect();
    // This slice begins at field 3 (state), so after skipping it, the numeric
    // slice begins at field 4. Parse the needed positions directly: fields
    // such as cutime, priority, and nice can be negative and must not shift RSS.
    if fields.len() < 22 {
        return None;
    }
    Some(vec![
        fields[11].parse().ok()?, // utime (field 14)
        fields[12].parse().ok()?, // stime (field 15)
        fields[19].parse().ok()?, // starttime (field 22)
        fields[21].parse::<i64>().ok()?.max(0) as u64, // rss pages (field 24)
    ])
}

fn build_proc_stat(fields: &[u64]) -> Option<ProcStat> {
    if fields.len() >= 4 {
        Some(ProcStat {
            utime: fields[0],
            stime: fields[1],
            starttime: fields[2],
            rss: fields[3],
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
