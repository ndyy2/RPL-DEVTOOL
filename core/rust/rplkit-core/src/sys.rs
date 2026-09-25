//! sys — snapshot sistem via `sysinfo` (Fase 2).
//!
//! Satu-satunya sumber data SYSTEM: dipakai native ops (system-info,
//! process, memory, disk) DAN panel TUI Overview — angka selalu konsisten.
//! Satuan mentah apa adanya dari sysinfo; format tampil via [`human_bytes`].

/// Ringkasan satu CPU.
#[derive(Debug, Clone)]
pub struct CpuStat {
    pub name: String,
    pub usage_pct: f32,
}

/// Snapshot memori + swap (satuan sama dengan sysinfo).
#[derive(Debug, Clone)]
pub struct MemStat {
    pub total: u64,
    pub used: u64,
    pub total_swap: u64,
    pub used_swap: u64,
}

/// Satu disk/mount.
#[derive(Debug, Clone)]
pub struct DiskStat {
    pub name: String,
    pub mount: String,
    pub fs: String,
    pub total: u64,
    pub available: u64,
}

/// Satu proses (peringkat pemakaian memori).
#[derive(Debug, Clone)]
pub struct ProcStat {
    pub name: String,
    pub memory: u64,
    pub cpu_pct: f32,
}

/// Snapshot penuh untuk `system-info` + Overview TUI.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub os: String,
    pub kernel: String,
    pub arch: String,
    pub hostname: String,
    pub cpu_count: usize,
    pub global_cpu_pct: f32,
    pub cpus: Vec<CpuStat>,
    pub mem: MemStat,
    pub disks: Vec<DiskStat>,
}

fn opt(o: Option<String>, fallback: &str) -> String {
    o.filter(|s| !s.is_empty()).unwrap_or_else(|| fallback.to_string())
}

fn plain(s: String, fallback: &str) -> String {
    if s.is_empty() {
        fallback.to_string()
    } else {
        s
    }
}

/// Ambil snapshot. Ada jeda ~200ms agar `cpu_usage` bermakna
/// (butuh dua sampel; sampel pertama selalu ~0).
pub fn snapshot() -> Snapshot {
    use sysinfo::{Disks, System};
    let mut sys = System::new_all();
    sys.refresh_all();
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_all();

    let cpus = sys
        .cpus()
        .iter()
        .map(|c| CpuStat {
            name: c.name().to_string(),
            usage_pct: c.cpu_usage(),
        })
        .collect::<Vec<_>>();
    let disks = Disks::new_with_refreshed_list();
    Snapshot {
        os: opt(System::name(), "unknown"),
        kernel: opt(System::kernel_version(), "unknown"),
        arch: plain(System::cpu_arch(), "unknown"),
        hostname: opt(System::host_name(), "unknown"),
        cpu_count: cpus.len(),
        global_cpu_pct: sys.global_cpu_usage(),
        cpus,
        mem: MemStat {
            total: sys.total_memory(),
            used: sys.used_memory(),
            total_swap: sys.total_swap(),
            used_swap: sys.used_swap(),
        },
        disks: disks
            .list()
            .iter()
            .map(|d| DiskStat {
                name: d.name().to_string_lossy().into_owned(),
                mount: d.mount_point().to_string_lossy().into_owned(),
                fs: d.file_system().to_string_lossy().into_owned(),
                total: d.total_space(),
                available: d.available_space(),
            })
            .collect(),
    }
}

/// N proses teratas menurut memori.
pub fn top_processes(n: usize) -> Vec<ProcStat> {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_all();
    let mut procs: Vec<ProcStat> = sys
        .processes()
        .values()
        .map(|p| ProcStat {
            name: p.name().to_string_lossy().into_owned(),
            memory: p.memory(),
            cpu_pct: p.cpu_usage(),
        })
        .collect();
    procs.sort_by(|a, b| b.memory.cmp(&a.memory));
    procs.truncate(n.clamp(1, 50));
    procs
}

/// 1024 → "1.0 KiB", 0 → "0 B". Terima satuan mentah apa pun.
pub fn human_bytes(n: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut v = n as f64;
    for u in UNITS {
        if v < 1024.0 || *u == "PiB" {
            return if *u == "B" {
                format!("{v:.0} {u}")
            } else {
                format!("{v:.1} {u}")
            };
        }
        v /= 1024.0;
    }
    format!("{v:.1} PiB")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_sane() {
        let s = snapshot();
        assert!(!s.arch.is_empty());
        assert!(s.cpu_count >= 1);
        assert!(s.mem.total > 0);
        assert!(s.mem.used <= s.mem.total);
        assert!(!s.disks.is_empty());
    }

    #[test]
    fn memory_matches_proc_meminfo() {
        // Validasi satuan sysinfo terhadap kernel, toleransi 5%.
        let meminfo = std::fs::read_to_string("/proc/meminfo").expect("/proc/meminfo");
        let kb: u64 = meminfo
            .lines()
            .find(|l| l.starts_with("MemTotal:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse().ok())
            .expect("MemTotal");
        let s = snapshot();
        let ratio = s.mem.total as f64 / (kb * 1024) as f64;
        assert!(
            (0.95..=1.05).contains(&ratio),
            "sysinfo bytes vs meminfo: ratio {ratio}"
        );
    }

    #[test]
    fn top_processes_ordered() {
        let ps = top_processes(5);
        assert!(!ps.is_empty());
        for w in ps.windows(2) {
            assert!(w[0].memory >= w[1].memory);
        }
    }

    #[test]
    fn human_bytes_vectors() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(1024), "1.0 KiB");
        assert_eq!(human_bytes(1536 * 1024), "1.5 MiB");
    }
}
