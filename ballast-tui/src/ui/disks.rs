use std::collections::HashMap;
use std::time::{Duration, Instant};

use ballast_core::model::disk::{DeviceKind, DiskDevice};
use ballast_platform_linux::disk_io::{DiskStatsSample, IOStat, read_disk_stats};
use ballast_platform_linux::enumerate_block_devices;
use ratatui::style::Style;
use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Rect},
    widgets::{Block, Borders, Row, Table, TableState},
};

use crate::app::App;

pub const LOCAL_KEYBINDS: &'static str = "[p]artitions [l]oopback";

/// How often we sample for disk stats.
/// Decoupled from render/frame rate.
const SAMPLE_INTERVAL: Duration = Duration::from_millis(1000);

pub struct DisksState {
    pub table_state: TableState,
    pub show_partitions: bool,
    pub show_loopback: bool,

    prev_samples: HashMap<String, DiskStatsSample>,
    last_stats: HashMap<String, IOStat>,
    last_sample_at: Option<Instant>,
}

impl Default for DisksState {
    fn default() -> Self {
        Self {
            table_state: TableState::default(),
            show_partitions: true,
            show_loopback: false,

            prev_samples: HashMap::new(),
            last_stats: HashMap::new(),
            last_sample_at: None,
        }
    }
}

impl DisksState {
    /// Re-samples /proc/diskstats if SAMPLE_INTERVAL has elapsed since the
    /// last sample, diffs against the previous raw snapshot per-device, and
    /// updates the cached IOStat map used for rendering. No-op (cheap) if
    /// called again before the interval elapses.
    fn maybe_resample(&mut self) {
        let now = Instant::now();
        let due = match self.last_sample_at {
            Some(last) => now.duration_since(last) >= SAMPLE_INTERVAL,
            None => true,
        };
        if !due {
            return;
        }

        let Ok(raw_map) = read_disk_stats() else {
            return; // keep showing stale cached values rather than blanking the UI
        };

        for (name, raw) in raw_map {
            let sample = DiskStatsSample { raw, taken_at: now };
            if let Some(prev) = self.prev_samples.get(&name) {
                self.last_stats.insert(name.clone(), sample.diff(prev));
            }
            self.prev_samples.insert(name, sample);
        }

        self.last_sample_at = Some(now);
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let header = Row::new([
        "device",
        "fstype",
        "used",
        "used/total",
        "read",
        "write",
        "iops (r/w)",
        "lat_ms (r/w)",
        "qd",
        "%util",
        "mount",
    ])
    .style(Style::new().bold());

    let widths = [
        Constraint::Length(12),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(24),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(6),
        Constraint::Length(6),
        Constraint::Length(12),
    ];

    let block_devices = enumerate_block_devices().unwrap();
    let state = &mut app.disks;
    state.maybe_resample();
    let dev_tree_rows = build_dev_tree(&block_devices, state.show_partitions, state.show_loopback);

    let rows: Vec<Row> = dev_tree_rows
        .iter()
        .map(|(dev, prefix)| {
            let used_percent = match (dev.used, dev.size) {
                (Some(used), Some(size)) if size > 0 => {
                    format!("{:.1}%", (used as f64 / size as f64) * 100.0)
                }
                _ => "-".to_string(),
            };
            let stat = state.last_stats.get(&dev.id);

            Row::new([
                format!("{prefix}{}", dev.id),
                dev.fstype.clone().unwrap_or_else(|| "-".to_string()),
                used_percent,
                format_used_total_col(dev.used, dev.size),
                format_rate(stat.map(|s| s.read_bytes_per_sec)),
                format_rate(stat.map(|s| s.write_bytes_per_sec)),
                format_pair(stat.map(|s| s.read_iops), stat.map(|s| s.write_iops), 0),
                format_pair(
                    stat.and_then(|s| s.read_latency_ms),
                    stat.and_then(|s| s.write_latency_ms),
                    1,
                ),
                stat.map(|s| format!("{:.1}", s.queue_depth_avg))
                    .unwrap_or_else(|| "-".to_string()),
                stat.map(|s| format!("{:.0}%", s.utilization_pct))
                    .unwrap_or_else(|| "-".to_string()),
                dev.mountpoints
                    .first()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ])
        })
        .collect();

    let state = &mut app.disks;
    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::new().yellow())
        .block(
            Block::default()
                .title("Disk Usage + I/O Throughput")
                .borders(Borders::ALL),
        );
    frame.render_stateful_widget(table, area, &mut state.table_state);
}

pub fn handle_key(key: KeyEvent, app: &mut App) {
    let state = &mut app.disks;
    match key.code {
        // Vim keybinds
        KeyCode::Char('j') | KeyCode::Down => state.table_state.select_next(),
        KeyCode::Char('k') | KeyCode::Up => state.table_state.select_previous(),
        KeyCode::Char('g') => state.table_state.select_first(),
        KeyCode::Char('G') => state.table_state.select_last(),
        // Local-keybinds
        KeyCode::Char('p') => state.show_partitions = !state.show_partitions,
        KeyCode::Char('l') => state.show_loopback = !state.show_loopback,
        _ => {}
    }
}

fn format_bytes(bytes: Option<u64>) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    let Some(bytes) = bytes else {
        return "-".to_string();
    };

    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}

/// Format bytes/sec -> "12.3 MiB/s", tolerating fractional bytes from the rate calc.
fn format_rate(bytes_per_sec: Option<f64>) -> String {
    match bytes_per_sec {
        Some(b) => format!("{}/s", format_bytes(Some(b.round() as u64))),
        None => "-".to_string(),
    }
}

/// Format a (read, write) pair as "r/w", e.g. iops "120/45" or latency "0.5/1.2".
/// `decimals` controls formatting precision; None on either side prints "-".
fn format_pair(read: Option<f64>, write: Option<f64>, decimals: usize) -> String {
    let fmt = |v: Option<f64>| match v {
        Some(x) => format!("{:.*}", decimals, x),
        None => "-".to_string(),
    };
    format!("{}/{}", fmt(read), fmt(write))
}

/// Formats used/total as two fixed-width, right-aligned fields so the
/// separating '/' lines up across rows regardless of unit.
fn format_used_total_col(used: Option<u64>, size: Option<u64>) -> String {
    format!("{:>10}/{:>10}", format_bytes(used), format_bytes(size))
}

fn build_dev_tree(
    devices: &[DiskDevice],
    show_partitions: bool,
    show_loopback: bool,
) -> Vec<(&DiskDevice, &'static str)> {
    let mut parents: Vec<&DiskDevice> = Vec::new();
    let mut children: HashMap<&str, Vec<&DiskDevice>> = HashMap::new();

    for dev in devices {
        match &dev.kind {
            DeviceKind::Partition { parent, .. } => {
                children.entry(parent.as_str()).or_default().push(dev);
            }
            DeviceKind::Loopback { .. } if !show_loopback => continue,
            _ => parents.push(dev),
        }
    }

    // `fs::read_dir` order isn't guaranteed; sort for stable output
    parents.sort_by(|a, b| a.id.cmp(&b.id));
    for kids in children.values_mut() {
        kids.sort_by_key(|d| match &d.kind {
            DeviceKind::Partition { part_num, .. } => *part_num,
            _ => 0,
        });
    }

    let mut rows = Vec::with_capacity(devices.len());
    for parent in parents {
        rows.push((parent, ""));

        if !show_partitions {
            continue;
        }

        if let Some(kids) = children.get(parent.id.as_str()) {
            let last_idx = kids.len().saturating_sub(1);
            for (i, kid) in kids.iter().enumerate() {
                let prefix = if i == last_idx { "└─" } else { "├─" };
                rows.push((*kid, prefix));
            }
        }
    }
    rows
}
