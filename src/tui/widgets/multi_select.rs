//! Multi-select list widget with checkboxes.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget},
};

/// Actions from multi-select interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiSelectAction {
    /// No action, continue
    None,
    /// User confirmed selection
    Confirm,
    /// User cancelled
    Cancel,
}

/// Multi-select list with checkboxes.
pub struct MultiSelect {
    items: Vec<(String, bool)>,
    state: ListState,
    title: String,
}

impl MultiSelect {
    /// Create a new multi-select list.
    pub fn new<T: ToString>(items: Vec<T>) -> Self {
        let items: Vec<(String, bool)> =
            items.into_iter().map(|i| (i.to_string(), false)).collect();
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

    /// Set the title.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Pre-select items by their labels.
    #[must_use]
    pub fn with_selected(mut self, labels: &[String]) -> Self {
        for (item, selected) in &mut self.items {
            *selected = labels.contains(item);
        }
        self
    }

    /// Get the selected item labels.
    pub fn selected_items(&self) -> Vec<&str> {
        self.items
            .iter()
            .filter(|(_, selected)| *selected)
            .map(|(item, _)| item.as_str())
            .collect()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Get the currently highlighted index (cursor position).
    pub const fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Toggle the currently selected item.
    pub fn toggle_current(&mut self) {
        if let Some(i) = self.state.selected() {
            if let Some((_, selected)) = self.items.get_mut(i) {
                *selected = !*selected;
            }
        }
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

    /// Add a new item to the list.
    pub fn add_item(&mut self, item: impl Into<String>) {
        let item = item.into();
        // Don't add duplicates
        if !self.items.iter().any(|(i, _)| i == &item) {
            self.items.push((item, true)); // New items are selected by default
                                           // Select the new item
            self.state.select(Some(self.items.len() - 1));
        }
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: KeyEvent) -> MultiSelectAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
                MultiSelectAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                MultiSelectAction::None
            }
            KeyCode::Char(' ') => {
                self.toggle_current();
                MultiSelectAction::None
            }
            KeyCode::Enter => MultiSelectAction::Confirm,
            KeyCode::Esc => MultiSelectAction::Cancel,
            _ => MultiSelectAction::None,
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
            .map(|(i, (item, selected))| {
                let is_cursor = Some(i) == self.state.selected();
                let style = if is_cursor {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let checkbox = if *selected { "[x] " } else { "[ ] " };
                let cursor = if is_cursor { "> " } else { "  " };

                ListItem::new(Line::from(vec![
                    Span::styled(cursor, style),
                    Span::styled(checkbox, style),
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

impl Clone for MultiSelect {
    fn clone(&self) -> Self {
        let mut new_ms = Self {
            items: self.items.clone(),
            state: ListState::default(),
            title: self.title.clone(),
        };
        if let Some(idx) = self.state.selected() {
            new_ms.state.select(Some(idx));
        }
        new_ms
    }
}
