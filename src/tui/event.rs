//! Event handling for the TUI.
//!
//! Provides a polling-based event handler using crossterm.

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};

/// Events that can occur in the TUI.
#[derive(Debug, Clone)]
pub enum TuiEvent {
    /// A key was pressed
    Key(KeyEvent),
    /// Terminal was resized
    Resize(u16, u16),
    /// Tick event for animations (if needed)
    Tick,
}

/// Polls for events with a timeout.
pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler with the given tick rate.
    pub const fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Wait for the next event.
    ///
    /// Returns `Ok(None)` on timeout (tick).
    pub fn next(&self) -> Result<TuiEvent> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                Event::Key(key) => {
                    // Only handle key press events, not release
                    if key.kind == KeyEventKind::Press {
                        Ok(TuiEvent::Key(key))
                    } else {
                        Ok(TuiEvent::Tick)
                    }
                }
                Event::Resize(w, h) => Ok(TuiEvent::Resize(w, h)),
                _ => Ok(TuiEvent::Tick),
            }
        } else {
            Ok(TuiEvent::Tick)
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(Duration::from_millis(100))
    }
}
