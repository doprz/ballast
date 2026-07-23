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
    ]);

    // NOTE: there must be a better way to do this
    let widths = [
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
    ];

    // TODO: get info
    let rows = [
        Row::new(["sda", "-", "-", "-", "-", "-", "-", "-", "-", "-"]),
        Row::new(["sdb", "-", "-", "-", "-", "-", "-", "-", "-", "-"]),
        Row::new(["sdc", "-", "-", "-", "-", "-", "-", "-", "-", "-"]),
    ];

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
        _ => {}
    }
}
