// ─── Process Sensor ─────────────────────────────────────────────────────
// Reads top processes from /proc/[pid]/stat and /proc/[pid]/comm.

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
    let clk_tck = get_clk_tck();
    let uptime = get_uptime().unwrap_or(1.0);

    let mut processes = collect_processes(clk_tck, uptime, total_mem);
    sort_by_cpu(&mut processes);
    processes.truncate(10);
    processes
}

fn collect_processes(
    clk_tck: u64, uptime: f64, total_mem: f64,
) -> Vec<ProcessInfo> {
    let mut processes = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else { return processes };
    for entry in entries.filter_map(|e| e.ok()) {
        let Some(pid) = parse_pid(&entry) else { continue };
        let Some(stat) = read_proc_stat(pid) else { continue };
        let name = read_proc_comm(pid).unwrap_or_else(|| "?".into());
        let user = get_user_for_pid(pid).unwrap_or_else(|| "root".into());
        let info = build_process_info(pid, name, user, &stat, clk_tck, uptime, total_mem);
        processes.push(info);
    }
    processes
}

fn parse_pid(entry: &std::fs::DirEntry) -> Option<u32> {
    let pid_str = entry.file_name().to_string_lossy().to_string();
    pid_str.parse::<u32>().ok()
}

fn build_process_info(
    pid: u32, name: String, user: String,
    stat: &ProcStat, clk_tck: u64, uptime: f64, total_mem: f64,
) -> ProcessInfo {
    let cpu_pct = calc_cpu_pct(stat, clk_tck, uptime);
    let mem_kb = calc_mem_kb(stat);
    let mem_pct = if total_mem > 0.0 { (mem_kb / total_mem) * 100.0 } else { 0.0 };
    ProcessInfo { pid, name, user, mem_pct, cpu_pct, mem_mb: mem_kb / 1024.0 }
}

fn calc_cpu_pct(stat: &ProcStat, clk_tck: u64, uptime: f64) -> f64 {
    if clk_tck == 0 || stat.starttime == 0 {
        return 0.0;
    }
    let total_time = stat.utime + stat.stime;
    let seconds = uptime - (stat.starttime as f64 / clk_tck as f64);
    if seconds > 0.0 {
        ((total_time as f64 / clk_tck as f64) / seconds) * 100.0
    } else {
        0.0
    }
}

fn calc_mem_kb(stat: &ProcStat) -> f64 {
    let page_size = get_page_size();
    (stat.rss as f64 * page_size as f64) / 1024.0
}

fn sort_by_cpu(processes: &mut Vec<ProcessInfo>) {
    processes.sort_by(|a, b| {
        b.cpu_pct.partial_cmp(&a.cpu_pct).unwrap_or(std::cmp::Ordering::Equal)
    });
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
    if fields.len() >= 23 {
        Some(fields)
    } else {
        None
    }
}

fn build_proc_stat(fields: &[u64]) -> Option<ProcStat> {
    if fields.len() >= 23 {
        Some(ProcStat {
            utime: fields[0],
            stime: fields[1],
            starttime: fields[18],
            rss: fields[21],
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

#[allow(dead_code)]
fn get_num_cpus() -> usize {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .map(|content| content.lines().filter(|l| l.starts_with("processor\t")).count())
        .unwrap_or(1)
}

fn get_clk_tck() -> u64 {
    let val: i64 = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    val.unsigned_abs()
}

fn get_uptime() -> Option<f64> {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse().ok())
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
    // Try to look up username from /etc/passwd
    if let Ok(content) = std::fs::read_to_string("/etc/passwd") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                if let Ok(line_uid) = parts[2].parse::<u32>() {
                    if line_uid == uid {
                        return parts[0].to_string();
                    }
                }
            }
        }
    }
    uid.to_string()
}
