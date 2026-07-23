mod overview;

const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Offset, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::Tabs,
};

use crate::app::{App, Tab};

pub fn draw(frame: &mut Frame, app: &App) {
    let root_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Percentage(100),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_name_ver(frame, root_layout[0]);
    draw_tab_header(frame, root_layout[0] + Offset::new(14, 0), app);
    draw_active_tab(frame, root_layout[1], app);
    draw_footer(frame, root_layout[2]);
}

fn draw_name_ver(frame: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled("ballast", Style::default().bold()),
        Span::raw(" "),
        Span::styled(VERSION, Style::default().dim()),
    ]);

    frame.render_widget(line, area);
}

fn draw_tab_header(frame: &mut Frame, area: Rect, app: &App) {
    let titles = Tab::ALL.iter().map(|t| t.title());
    let selected_idx = Tab::ALL.iter().position(|t| *t == app.tab).unwrap();

    let tabs = Tabs::new(titles)
        .style(Color::Gray)
        .highlight_style(Style::default().yellow().bold())
        .select(selected_idx)
        .divider("")
        .padding(" ", "");

    frame.render_widget(tabs, area);
}

fn draw_active_tab(frame: &mut Frame, area: Rect, app: &App) {
    match app.tab {
        Tab::Overview => overview::render(frame, area),
        // TODO: add the rest of the tabs
        _ => overview::render(frame, area),
    }
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    // TODO: add per-tab keyboard shortcuts
    let footer = Text::styled("[q]uit [tab]next", Style::default().dim());
    frame.render_widget(footer, area);
}
