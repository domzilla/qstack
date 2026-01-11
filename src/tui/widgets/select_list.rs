//! Single-select scrollable list widget.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget},
};

/// Actions from list interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectAction {
    /// No action, continue
    None,
    /// User confirmed selection
    Confirm,
    /// User cancelled
    Cancel,
}

/// Single-select scrollable list.
pub struct SelectList {
    items: Vec<String>,
    state: ListState,
    title: String,
}

impl SelectList {
    /// Create a new select list.
    pub fn new<T: ToString>(items: Vec<T>) -> Self {
        let items: Vec<String> = items.into_iter().map(|i| i.to_string()).collect();
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self {
            items,
            state,
            title: String::new(),
        }
    }

    /// Set the title/prompt.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Get the currently selected index.
    pub const fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Check if list is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Move selection up.
    pub fn select_previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: KeyEvent) -> SelectAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
                SelectAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                SelectAction::None
            }
            KeyCode::Enter => SelectAction::Confirm,
            KeyCode::Esc => SelectAction::Cancel,
            _ => SelectAction::None,
        }
    }

    /// Render the widget.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(if self.title.is_empty() {
                String::new()
            } else {
                format!(" {} ", self.title)
            });

        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if Some(i) == self.state.selected() {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let prefix = if Some(i) == self.state.selected() {
                    "> "
                } else {
                    "  "
                };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(item, style),
                ]))
            })
            .collect();

        let list = List::new(items).block(block).highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

        StatefulWidget::render(list, area, buf, &mut self.state);
    }
}

impl Clone for SelectList {
    fn clone(&self) -> Self {
        let mut new_list = Self::new(self.items.clone());
        new_list.title.clone_from(&self.title);
        if let Some(idx) = self.state.selected() {
            new_list.state.select(Some(idx));
        }
        new_list
    }
}
