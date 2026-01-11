//! # UI Utilities
//!
//! Shared user interface utilities for interactive dialogs, table formatting,
//! and common UI patterns used across commands.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::io::IsTerminal;

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};

use crate::{
    config::Config,
    editor, id,
    item::{Item, Status},
    tui::screens::select_from_list as tui_select,
};

// =============================================================================
// Interactive Mode Resolution
// =============================================================================

/// Common interactive mode flags used across commands.
///
/// Consolidates the `--interactive` / `--no-interactive` flag pattern.
#[derive(Debug, Clone, Copy, Default)]
pub struct InteractiveArgs {
    /// Force interactive mode
    pub interactive: bool,
    /// Force non-interactive mode
    pub no_interactive: bool,
}

impl InteractiveArgs {
    /// Resolves interactive mode from flags and config.
    ///
    /// Priority: explicit `--interactive` > explicit `--no-interactive` > config default
    pub const fn resolve(&self, config_default: bool) -> bool {
        if self.interactive {
            true
        } else if self.no_interactive {
            false
        } else {
            config_default
        }
    }

    /// Checks if we should run interactive mode (combines flag resolution with terminal check).
    pub fn should_run(&self, config: &Config) -> bool {
        self.resolve(config.interactive()) && std::io::stdout().is_terminal()
    }
}

/// Resolves interactive mode from CLI flags and config.
///
/// Priority: explicit `--interactive` flag > explicit `--no-interactive` flag > config default
pub const fn resolve_interactive(
    interactive_flag: bool,
    no_interactive_flag: bool,
    config_default: bool,
) -> bool {
    if interactive_flag {
        true
    } else if no_interactive_flag {
        false
    } else {
        config_default
    }
}

/// Checks if we should run interactive mode (combines flag resolution with terminal check).
pub fn should_run_interactive(
    interactive_flag: bool,
    no_interactive_flag: bool,
    config: &Config,
) -> bool {
    resolve_interactive(interactive_flag, no_interactive_flag, config.interactive())
        && std::io::stdout().is_terminal()
}

// =============================================================================
// Interactive Selection
// =============================================================================

/// Generic interactive selection dialog.
///
/// Displays a list of options and returns the index of the selected item.
pub fn select_from_list<T: ToString>(prompt: &str, options: &[T]) -> Result<usize> {
    tui_select(prompt, options)
}

/// Interactive selection for items - returns index.
///
/// Formats items as "{id} - {title}" for display.
pub fn select_item(prompt: &str, items: &[Item]) -> Result<usize> {
    let options: Vec<String> = items
        .iter()
        .map(|item| format!("{} - {}", item.id(), item.title()))
        .collect();

    select_from_list(prompt, &options)
}

/// Interactive selection for item references - returns index.
pub fn select_item_ref(prompt: &str, items: &[&Item]) -> Result<usize> {
    let options: Vec<String> = items
        .iter()
        .map(|item| format!("{} - {}", item.id(), item.title()))
        .collect();

    select_from_list(prompt, &options)
}

/// Opens an item in the editor and prints its relative path.
pub fn open_item_in_editor(item: &Item, config: &Config) -> Result<()> {
    let path = item.path.as_ref().context("Item has no path")?;
    println!("{}", config.relative_path(path).display());
    editor::open(path, config).context("Failed to open editor")
}

// =============================================================================
// String Utilities
// =============================================================================

/// Truncates a string to the specified maximum length, adding ellipsis if truncated.
pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

// =============================================================================
// Table Building
// =============================================================================

/// Creates a new table with default styling.
pub fn create_table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

/// Prints an item table with standard columns: ID, Status, Title, Labels, Category.
pub fn print_items_table(items: &[Item]) {
    print_items_table_ref(&items.iter().collect::<Vec<_>>());
}

/// Prints an item table from references.
pub fn print_items_table_ref(items: &[&Item]) {
    let mut table = create_table();
    table.set_header(vec!["ID", "Status", "Title", "Labels", "Category"]);

    for item in items {
        let status_cell = status_cell(item.status());
        let labels = item.labels().join(", ");
        let category = item.category().unwrap_or("-");
        let short_id = id::short_form(item.id());

        table.add_row(vec![
            Cell::new(short_id),
            status_cell,
            Cell::new(truncate(item.title(), 40)),
            Cell::new(truncate(&labels, 20)),
            Cell::new(category),
        ]);
    }

    println!("{table}");
}

/// Prints a compact item table without the category column.
pub fn print_items_table_compact(items: &[&Item]) {
    let mut table = create_table();
    table.set_header(vec!["ID", "Status", "Title", "Labels"]);

    for item in items {
        let status_cell = status_cell(item.status());
        let labels = item.labels().join(", ");
        let short_id = id::short_form(item.id());

        table.add_row(vec![
            Cell::new(short_id),
            status_cell,
            Cell::new(truncate(item.title(), 40)),
            Cell::new(truncate(&labels, 20)),
        ]);
    }

    println!("{table}");
}

/// Creates a colored status cell.
fn status_cell(status: Status) -> Cell {
    match status {
        Status::Open => Cell::new("open").fg(Color::Green),
        Status::Closed => Cell::new("closed").fg(Color::Red),
    }
}

/// Extracts the short ID for display.
///
/// Re-export of `id::short_form` for convenience.
pub fn short_id(id: &str) -> &str {
    id::short_form(id)
}
