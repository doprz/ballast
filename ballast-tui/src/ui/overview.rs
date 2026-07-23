use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders},
};

pub fn render(frame: &mut Frame, area: Rect) {
    let layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(area);

    let row1 = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(layout[0]);

    let agg_throughput = Block::default()
        .borders(Borders::ALL)
        .title("Aggregate Throughput");

    let arc_cache = Block::default().borders(Borders::ALL).title("Arc Cache");

    frame.render_widget(agg_throughput, row1[0]);
    frame.render_widget(arc_cache, row1[1]);

    let capacity = Block::default()
        .borders(Borders::ALL)
        .title("Capacity by pool / array");
    let alerts = Block::default().borders(Borders::ALL).title("Alerts");
    frame.render_widget(capacity, layout[1]);
    frame.render_widget(alerts, layout[2]);
}
