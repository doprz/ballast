use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;

use crate::ui::{self, disks::DisksState};

pub const POLL_TIME_MS: u64 = 1000;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Overview,
    Disks,
    Zfs,
    Smart,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Overview, Tab::Disks, Tab::Zfs, Tab::Smart];

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Overview => "[1]Overview",
            Tab::Disks => "[2]Disks",
            Tab::Zfs => "[3]ZFS",
            Tab::Smart => "[4]SMART",
        }
    }

    fn next(self) -> Self {
        let idx = Tab::ALL.iter().position(|t| *t == self).unwrap();
        Tab::ALL[(idx + 1) % Tab::ALL.len()]
    }

    fn prev(self) -> Self {
        let idx = Tab::ALL.iter().position(|t| *t == self).unwrap();
        Tab::ALL[(idx + Tab::ALL.len() - 1) % Tab::ALL.len()]
    }
}

#[derive(Default)]
pub struct App {
    pub tab: Tab,
    pub should_quit: bool,

    pub disks: DisksState,
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| ui::draw(frame, &mut self))?;
            self.handle_events()?;
        }

        Ok(())
    }

    pub fn handle_events(&mut self) -> color_eyre::Result<()> {
        if event::poll(Duration::from_millis(POLL_TIME_MS))? {
            let Event::Key(key) = event::read()? else {
                return Ok(());
            };
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Tab => self.tab = self.tab.next(),
                KeyCode::BackTab => self.tab = self.tab.prev(),
                KeyCode::Char(c @ '1'..='4') => {
                    let idx = c.to_digit(10).unwrap() as usize - 1;
                    self.tab = Tab::ALL[idx];
                }
                // Forward non-global keybinds
                _ => ui::handle_key(key, self),
            }
        }

        Ok(())
    }
}
