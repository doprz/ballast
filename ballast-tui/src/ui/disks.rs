use std::collections::HashMap;

use ballast_core::model::disk::{DeviceKind, DiskDevice};
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

#[derive(Default)]
pub struct DisksState {
    pub table_state: TableState,
    pub show_partitions: bool,
    pub show_loopback: bool,
}

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let header = Row::new([
        "device",
        "fstype",
        "used",
        "used/total",
        "read",
        "write",
        "iops",
        "lat_ms",
        "qd",
        "mount",
    ])
    .style(Style::new().bold());

    // NOTE: there must be a better way to do this
    let widths = [
        Constraint::Length(12),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(24),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
    ];

    let block_devices = enumerate_block_devices().unwrap();
    let state = &mut app.disks;
    let dev_tree_rows = build_dev_tree(&block_devices, state.show_partitions, state.show_loopback);

    // TODO: get I/O throughput
    let rows: Vec<Row> = dev_tree_rows
        .iter()
        .map(|(dev, prefix)| {
            let used_percent = match (dev.used, dev.size) {
                (Some(used), Some(size)) if size > 0 => {
                    format!("{:.1}%", (used as f64 / size as f64) * 100.0)
                }
                _ => "-".to_string(),
            };
            Row::new([
                format!("{prefix}{}", dev.id),
                dev.fstype.clone().unwrap_or_else(|| "-".to_string()),
                used_percent,
                format!("{}/{}", format_bytes(dev.used), format_bytes(dev.size)),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
                "-".to_string(),
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
