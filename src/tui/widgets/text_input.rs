//! Single-line text input widget.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

/// Single-line text input with cursor.
#[derive(Debug, Clone)]
pub struct TextInput {
    content: String,
    cursor: usize,
    label: String,
}

impl TextInput {
    /// Create a new text input with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            content: String::new(),
            cursor: 0,
            label: label.into(),
        }
    }

    /// Set initial content.
    #[must_use]
    pub fn with_initial(mut self, value: impl Into<String>) -> Self {
        self.content = value.into();
        self.cursor = self.content.len();
        self
    }

    /// Get the current content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Check if the input is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Handle a key event.
    ///
    /// Returns `true` if the event was handled.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(c) => {
                // Handle Ctrl+key combinations first
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match c {
                        'u' => {
                            // Ctrl+U: Clear line
                            self.content.clear();
                            self.cursor = 0;
                            return true;
                        }
                        'w' => {
                            // Ctrl+W: Delete word backward
                            while self.cursor > 0
                                && self.content.chars().nth(self.cursor - 1) == Some(' ')
                            {
                                self.cursor -= 1;
                                self.content.remove(self.cursor);
                            }
                            while self.cursor > 0
                                && self.content.chars().nth(self.cursor - 1) != Some(' ')
                            {
                                self.cursor -= 1;
                                self.content.remove(self.cursor);
                            }
                            return true;
                        }
                        _ => return false, // Let other Ctrl combinations bubble up
                    }
                }
                // Regular character input
                self.content.insert(self.cursor, c);
                self.cursor += 1;
                true
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.content.remove(self.cursor);
                }
                true
            }
            KeyCode::Delete => {
                if self.cursor < self.content.len() {
                    self.content.remove(self.cursor);
                }
                true
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                true
            }
            KeyCode::Right => {
                if self.cursor < self.content.len() {
                    self.cursor += 1;
                }
                true
            }
            KeyCode::Home => {
                self.cursor = 0;
                true
            }
            KeyCode::End => {
                self.cursor = self.content.len();
                true
            }
            _ => false,
        }
    }

    /// Render the widget.
    pub fn render(&self, area: Rect, buf: &mut Buffer, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {} ", self.label));

        let inner = block.inner(area);
        block.render(area, buf);

        // Render content with cursor
        if focused {
            let (before, after) = self.content.split_at(self.cursor);
            let cursor_char = after.chars().next().unwrap_or(' ');
            let after_cursor = if after.is_empty() {
                String::new()
            } else {
                after.chars().skip(1).collect()
            };

            let line = Line::from(vec![
                Span::raw(before),
                Span::styled(
                    cursor_char.to_string(),
                    Style::default()
                        .bg(Color::White)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(after_cursor),
            ]);

            Paragraph::new(line).render(inner, buf);
        } else {
            Paragraph::new(self.content.as_str()).render(inner, buf);
        }
    }
}
