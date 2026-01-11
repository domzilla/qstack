//! Generic selection screen.
//!
//! Replaces dialoguer's Select for item selection.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::{
    event::TuiEvent,
    run,
    widgets::{SelectAction, SelectList},
    AppResult, TuiApp,
};

/// Selection screen application.
struct SelectScreen {
    list: SelectList,
    prompt: String,
}

impl SelectScreen {
    fn new(prompt: impl Into<String>, items: Vec<String>) -> Self {
        let list = SelectList::new(items).with_title("Select");
        Self {
            list,
            prompt: prompt.into(),
        }
    }
}

impl TuiApp for SelectScreen {
    type Output = usize;

    fn handle_event(&mut self, event: &TuiEvent) -> Option<AppResult<Self::Output>> {
        match event {
            TuiEvent::Key(key) => {
                // Handle Ctrl+C
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Some(AppResult::Cancelled);
                }

                match self.list.handle_key(*key) {
                    SelectAction::Confirm => self.list.selected_index().map(AppResult::Done),
                    SelectAction::Cancel => Some(AppResult::Cancelled),
                    SelectAction::None => None,
                }
            }
            _ => None,
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Layout: prompt at top, list below, help at bottom
        let chunks = Layout::vertical([
            Constraint::Length(3), // Prompt
            Constraint::Min(5),    // List
            Constraint::Length(3), // Help
        ])
        .split(area);

        // Prompt
        let prompt_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Select ");

        let prompt = Paragraph::new(self.prompt.as_str()).block(prompt_block);
        frame.render_widget(prompt, chunks[0]);

        // List
        let mut list_clone = self.list.clone();
        list_clone.render(chunks[1], frame.buffer_mut(), true);

        // Help
        let help = Paragraph::new(Line::from(vec![
            ratatui::text::Span::styled("↑↓", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::raw(" Navigate  "),
            ratatui::text::Span::styled("Enter", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::raw(" Select  "),
            ratatui::text::Span::styled("Esc", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::raw(" Cancel"),
        ]))
        .block(Block::default().borders(Borders::ALL));

        frame.render_widget(help, chunks[2]);
    }
}

/// Select from a list of options.
///
/// Returns the index of the selected item, or an error if cancelled.
pub fn select_from_list<T: ToString>(prompt: &str, options: &[T]) -> Result<usize> {
    let items: Vec<String> = options.iter().map(ToString::to_string).collect();

    if items.is_empty() {
        anyhow::bail!("No items to select from");
    }

    let app = SelectScreen::new(prompt, items);

    match run(app)? {
        Some(index) => Ok(index),
        None => anyhow::bail!("Selection cancelled"),
    }
}
