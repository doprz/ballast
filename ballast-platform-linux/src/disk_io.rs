use std::{collections::HashMap, time::Instant};

/// Raw cumulative stats read directly from /proc/diskstats
/// See: https://www.kernel.org/doc/html/latest/admin-guide/iostats.html
#[derive(Debug, Clone, Copy, Default)]
pub struct RawDiskStats {
    pub reads_completed: u64,
    pub reads_merged: u64,
    pub sectors_read: u64,
    pub read_time_ms: u64,

    pub writes_completed: u64,
    pub writes_merged: u64,
    pub sectors_written: u64,
    pub write_time_ms: u64,

    pub io_in_progress: u64,
    pub io_time_ms: u64,
    pub weighted_io_time_ms: u64,

    pub discards_completed: u64,
    pub discards_merged: u64,
    pub sectors_discarded: u64,
    pub discard_time_ms: u64,

    pub flushes_completed: u64,
    pub flush_time_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct DiskStatsSample {
    pub raw: RawDiskStats,
    pub taken_at: Instant,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct IOStat {
    pub read_iops: f64,
    pub write_iops: f64,
    pub read_bytes_per_sec: f64,
    pub write_bytes_per_sec: f64,

    pub read_latency_ms: Option<f64>,
    pub write_latency_ms: Option<f64>,

    pub avg_read_request_bytes: Option<f64>,
    pub avg_write_request_bytes: Option<f64>,

    pub read_merge_ratio: Option<f64>,
    pub write_merge_ratio: Option<f64>,

    pub queue_depth_current: u64,
    pub queue_depth_avg: f64,
    pub utilization_pct: f64,

    pub discard_iops: f64,
    pub discard_bytes_per_sec: f64,
    pub discard_latency_ms: Option<f64>,

    pub flush_iops: f64,
    pub flush_latency_ms: Option<f64>,

    pub interval_secs: f64,
}

impl DiskStatsSample {
    /// Compute an IOStat by diffing `self` (later) against `prev` (earlier).
    pub fn diff(&self, prev: &DiskStatsSample) -> IOStat {
        // 512-byte sector as per the kernel ABI regardless of physical/logical sector size
        const SECTOR_BYTES: f64 = 512.0;

        let dt = (self.taken_at - prev.taken_at).as_secs_f64();
        if dt <= 0.0 {
            return IOStat::default();
        }

        // NOTE: saturating subs to handle hot-unplugged/replugged devices, the kernel stat reset, or a counter wraparound
        let d_reads = self
            .raw
            .reads_completed
            .saturating_sub(prev.raw.reads_completed);
        let d_writes = self
            .raw
            .writes_completed
            .saturating_sub(prev.raw.writes_completed);
        let d_read_sectors = self.raw.sectors_read.saturating_sub(prev.raw.sectors_read);
        let d_write_sectors = self
            .raw
            .sectors_written
            .saturating_sub(prev.raw.sectors_written);
        let d_read_time = self.raw.read_time_ms.saturating_sub(prev.raw.read_time_ms);
        let d_write_time = self
            .raw
            .write_time_ms
            .saturating_sub(prev.raw.write_time_ms);
        let d_reads_merged = self.raw.reads_merged.saturating_sub(prev.raw.reads_merged);
        let d_writes_merged = self
            .raw
            .writes_merged
            .saturating_sub(prev.raw.writes_merged);
        let d_io_time = self.raw.io_time_ms.saturating_sub(prev.raw.io_time_ms);
        let d_weighted_io_time = self
            .raw
            .weighted_io_time_ms
            .saturating_sub(prev.raw.weighted_io_time_ms);

        let d_discards = self
            .raw
            .discards_completed
            .saturating_sub(prev.raw.discards_completed);
        let d_discard_sectors = self
            .raw
            .sectors_discarded
            .saturating_sub(prev.raw.sectors_discarded);
        let d_discard_time = self
            .raw
            .discard_time_ms
            .saturating_sub(prev.raw.discard_time_ms);

        let d_flushes = self
            .raw
            .flushes_completed
            .saturating_sub(prev.raw.flushes_completed);
        let d_flush_time = self
            .raw
            .flush_time_ms
            .saturating_sub(prev.raw.flush_time_ms);

        // Avoid div-by-zero, return None when no ops happened
        let per_op = |time_ms: u64, ops: u64| (ops > 0).then(|| time_ms as f64 / ops as f64);
        let avg_size =
            |sectors: u64, ops: u64| (ops > 0).then(|| sectors as f64 * SECTOR_BYTES / ops as f64);
        let merge_ratio = |merged: u64, completed: u64| {
            let total = merged + completed;
            (total > 0).then(|| merged as f64 / total as f64)
        };

        IOStat {
            read_iops: d_reads as f64 / dt,
            write_iops: d_writes as f64 / dt,
            read_bytes_per_sec: d_read_sectors as f64 * SECTOR_BYTES / dt,
            write_bytes_per_sec: d_write_sectors as f64 * SECTOR_BYTES / dt,

            read_latency_ms: per_op(d_read_time, d_reads),
            write_latency_ms: per_op(d_write_time, d_writes),

            avg_read_request_bytes: avg_size(d_read_sectors, d_reads),
            avg_write_request_bytes: avg_size(d_write_sectors, d_writes),

            read_merge_ratio: merge_ratio(d_reads_merged, d_reads),
            write_merge_ratio: merge_ratio(d_writes_merged, d_writes),

            queue_depth_current: self.raw.io_in_progress,
            queue_depth_avg: d_weighted_io_time as f64 / (dt * 1000.0),
            utilization_pct: (d_io_time as f64 / (dt * 1000.0) * 100.0).min(100.0),

            discard_iops: d_discards as f64 / dt,
            discard_bytes_per_sec: d_discard_sectors as f64 * SECTOR_BYTES / dt,
            discard_latency_ms: per_op(d_discard_time, d_discards),

            flush_iops: d_flushes as f64 / dt,
            flush_latency_ms: per_op(d_flush_time, d_flushes),

            interval_secs: dt,
        }
    }
}

/// Parses /proc/diskstats into a map keyed by device name (e.g. "sda", "sda1", "nvme0n1p1").
/// Silently zero-fills discard/flush fields on kernels that don't report them (pre-4.18 / pre-5.5)
/// rather than erroring; absent fields should read as "unsupported", not crash the sampler.
pub fn read_disk_stats() -> std::io::Result<HashMap<String, RawDiskStats>> {
    let contents = std::fs::read_to_string("/proc/diskstats")?;
    let mut map = HashMap::with_capacity(32);

    for line in contents.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        // major, minor, name, then at least the 11 pre-4.18 counters
        if fields.len() < 14 {
            continue;
        }

        let name = fields[2].to_string();
        let get = |i: usize| -> u64 { fields.get(i).and_then(|s| s.parse().ok()).unwrap_or(0) };

        let raw = RawDiskStats {
            reads_completed: get(3),
            reads_merged: get(4),
            sectors_read: get(5),
            read_time_ms: get(6),
            writes_completed: get(7),
            writes_merged: get(8),
            sectors_written: get(9),
            write_time_ms: get(10),
            io_in_progress: get(11),
            io_time_ms: get(12),
            weighted_io_time_ms: get(13),
            discards_completed: get(14),
            discards_merged: get(15),
            sectors_discarded: get(16),
            discard_time_ms: get(17),
            flushes_completed: get(18),
            flush_time_ms: get(19),
        };

        map.insert(name, raw);
    }

    Ok(map)
}
