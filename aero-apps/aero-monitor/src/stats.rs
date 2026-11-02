//! System statistics collected from /proc and /sys.

use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct DiskStat {
    pub mount: String,
    pub device: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub percent: f64,
}

#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub cpu_percent: f64,
    pub mem_total_gb: f64,
    pub mem_used_gb: f64,
    pub mem_percent: f64,
    pub swap_total_gb: f64,
    pub swap_used_gb: f64,
    pub disks: Vec<DiskStat>,
}

impl SystemSnapshot {
    /// Collect a system snapshot from /proc.
    pub async fn collect() -> Result<Self> {
        let cpu = read_cpu_percent().await?;
        let (mem_total, mem_used, swap_total, swap_used) = read_meminfo().await?;
        let disks = read_disk_usage().await;

        let mem_percent = if mem_total > 0.0 {
            mem_used / mem_total * 100.0
        } else {
            0.0
        };

        Ok(Self {
            cpu_percent: cpu,
            mem_total_gb: mem_total / 1024.0 / 1024.0,
            mem_used_gb: mem_used / 1024.0 / 1024.0,
            mem_percent,
            swap_total_gb: swap_total / 1024.0 / 1024.0,
            swap_used_gb: swap_used / 1024.0 / 1024.0,
            disks,
        })
    }
}

/// Read CPU usage by comparing two /proc/stat samples 200ms apart.
async fn read_cpu_percent() -> Result<f64> {
    let s1 = read_proc_stat_cpu().await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    let s2 = read_proc_stat_cpu().await?;

    let idle_delta = s2.idle.saturating_sub(s1.idle);
    let total_delta = s2.total.saturating_sub(s1.total);

    if total_delta == 0 {
        return Ok(0.0);
    }
    Ok((1.0 - idle_delta as f64 / total_delta as f64) * 100.0)
}

struct CpuStat {
    idle: u64,
    total: u64,
}

async fn read_proc_stat_cpu() -> Result<CpuStat> {
    let text = tokio::fs::read_to_string("/proc/stat")
        .await
        .map_err(|e| anyhow!("/proc/stat: {}", e))?;

    for line in text.lines() {
        if !line.starts_with("cpu ") {
            continue;
        }
        let nums: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|s| s.parse().ok())
            .collect();
        // Fields: user nice system idle iowait irq softirq steal guest guest_nice
        if nums.len() < 4 {
            return Err(anyhow!("unexpected /proc/stat format"));
        }
        let idle = nums[3] + nums.get(4).copied().unwrap_or(0); // idle + iowait
        let total: u64 = nums.iter().sum();
        return Ok(CpuStat { idle, total });
    }
    Err(anyhow!("no cpu line in /proc/stat"))
}

/// Read memory info from /proc/meminfo. Returns (total_kb, used_kb, swap_total_kb, swap_used_kb).
async fn read_meminfo() -> Result<(f64, f64, f64, f64)> {
    let text = tokio::fs::read_to_string("/proc/meminfo")
        .await
        .map_err(|e| anyhow!("/proc/meminfo: {}", e))?;

    let mut mem_total = 0u64;
    let mut mem_free = 0u64;
    let mut mem_available = 0u64;
    let mut buffers = 0u64;
    let mut cached = 0u64;
    let mut swap_total = 0u64;
    let mut swap_free = 0u64;

    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let key = parts.next().unwrap_or("").trim_end_matches(':');
        let val: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        match key {
            "MemTotal" => mem_total = val,
            "MemFree" => mem_free = val,
            "MemAvailable" => mem_available = val,
            "Buffers" => buffers = val,
            "Cached" => cached = val,
            "SwapTotal" => swap_total = val,
            "SwapFree" => swap_free = val,
            _ => {}
        }
    }

    // Prefer MemAvailable (kernel's estimate of reclaimable memory). Fall back
    // to the classic free+buffers+cached calculation on older kernels.
    let used_kb = if mem_available > 0 {
        mem_total.saturating_sub(mem_available)
    } else {
        mem_total.saturating_sub(mem_free + buffers + cached)
    };
    let mem_used = used_kb as f64;
    let swap_used = swap_total.saturating_sub(swap_free) as f64;
    Ok((mem_total as f64, mem_used, swap_total as f64, swap_used))
}

/// Read disk usage via `df`. Falls back to empty list on error.
async fn read_disk_usage() -> Vec<DiskStat> {
    let Ok(output) = tokio::process::Command::new("df")
        .args([
            "--output=source,target,size,used,pcent",
            "--block-size=1K",
            "-x",
            "tmpfs",
            "-x",
            "devtmpfs",
        ])
        .output()
        .await
    else {
        return vec![];
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let mut disks = Vec::new();

    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 5 {
            continue;
        }
        let device = cols[0].to_string();
        let mount = cols[1].to_string();
        let total_kb: f64 = cols[2].parse().unwrap_or(0.0);
        let used_kb: f64 = cols[3].parse().unwrap_or(0.0);
        let percent: f64 = cols[4].trim_end_matches('%').parse().unwrap_or(0.0);

        disks.push(DiskStat {
            device,
            mount,
            total_gb: total_kb / 1024.0 / 1024.0,
            used_gb: used_kb / 1024.0 / 1024.0,
            percent,
        });
    }
    disks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn collect_snapshot_succeeds() {
        let snap = SystemSnapshot::collect().await;
        assert!(snap.is_ok(), "collect failed: {:?}", snap);
        let s = snap.unwrap();
        assert!(s.cpu_percent >= 0.0 && s.cpu_percent <= 100.0);
        assert!(s.mem_total_gb > 0.0);
        assert!(s.mem_used_gb <= s.mem_total_gb);
    }
}
