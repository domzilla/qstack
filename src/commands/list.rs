//! # List Command
//!
//! Lists qstack items with filtering and sorting options.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::{cmp::Reverse, io::IsTerminal};

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};
use dialoguer::{theme::ColorfulTheme, Select};
use owo_colors::OwoColorize;

use crate::{
    config::Config,
    editor,
    item::{Item, Status},
    storage,
};

/// Sort order for listing
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum SortBy {
    #[default]
    Id,
    Date,
    Title,
}

/// Filter options for listing
#[allow(clippy::struct_excessive_bools)]
pub struct ListFilter {
    pub open: bool,
    pub closed: bool,
    pub label: Option<String>,
    pub author: Option<String>,
    pub sort: SortBy,
    pub interactive: bool,
    pub no_interactive: bool,
}

impl Default for ListFilter {
    fn default() -> Self {
        Self {
            open: false,
            closed: false,
            label: None,
            author: None,
            sort: SortBy::Id,
            interactive: false,
            no_interactive: false,
        }
    }
}

/// Common filter options for item queries
pub struct ItemFilter {
    pub label: Option<String>,
    pub author: Option<String>,
}

/// Collects and filters items from storage.
///
/// If `include_archived` is true, collects from archive directory,
/// otherwise collects from the main stack directory.
pub fn collect_items(config: &Config, include_archived: bool, filter: &ItemFilter) -> Vec<Item> {
    let paths: Vec<_> = if include_archived {
        storage::walk_archived(config).collect()
    } else {
        storage::walk_items(config).collect()
    };

    paths
        .into_iter()
        .filter_map(|path| Item::load(&path).ok())
        .filter(|item| apply_item_filter(item, filter))
        .collect()
}

/// Sorts items in place by the given sort order.
pub fn sort_items(items: &mut [Item], sort: SortBy) {
    match sort {
        SortBy::Id => items.sort_by(|a, b| a.id().cmp(b.id())),
        SortBy::Date => items.sort_by_key(|item| Reverse(item.created_at())),
        SortBy::Title => items.sort_by_key(|item| item.title().to_lowercase()),
    }
}

fn apply_item_filter(item: &Item, filter: &ItemFilter) -> bool {
    // Label filter
    if let Some(ref label) = filter.label {
        if !item.labels().iter().any(|l| l.eq_ignore_ascii_case(label)) {
            return false;
        }
    }

    // Author filter
    if let Some(ref author) = filter.author {
        if !item.author().eq_ignore_ascii_case(author) {
            return false;
        }
    }

    true
}

/// Executes the list command.
pub fn execute(filter: &ListFilter) -> Result<()> {
    let config = Config::load()?;

    // Collect items based on status filter
    let item_filter = ItemFilter {
        label: filter.label.clone(),
        author: filter.author.clone(),
    };

    let mut items = if filter.closed {
        collect_items(&config, true, &item_filter)
    } else {
        // Default: show open items only
        collect_items(&config, false, &item_filter)
    };

    // Sort items
    sort_items(&mut items, filter.sort);

    // Display
    if items.is_empty() {
        println!("{}", "No items found.".dimmed());
        return Ok(());
    }

    print_table(&items);

    // Resolve interactive mode: flags override config
    let interactive = if filter.interactive {
        true
    } else if filter.no_interactive {
        false
    } else {
        config.interactive()
    };

    // Non-interactive mode: just show table
    if !interactive {
        return Ok(());
    }

    // Interactive selection (only in terminal)
    if !std::io::stdout().is_terminal() {
        return Ok(());
    }

    let selection = interactive_select(&items)?;
    let item = &items[selection];
    let path = item.path.as_ref().context("Item has no path")?;

    println!("{}", config.relative_path(path).display());
    editor::open(path, &config).context("Failed to open editor")?;

    Ok(())
}

/// Show interactive selection dialog.
fn interactive_select(items: &[Item]) -> Result<usize> {
    let options: Vec<String> = items
        .iter()
        .map(|item| format!("{} - {}", item.id(), item.title()))
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select an item to open")
        .items(&options)
        .default(0)
        .interact()
        .context("Selection cancelled")?;

    Ok(selection)
}

fn print_table(items: &[Item]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["ID", "Status", "Title", "Labels", "Category"]);

    for item in items {
        let status_cell = match item.status() {
            Status::Open => Cell::new("open").fg(Color::Green),
            Status::Closed => Cell::new("closed").fg(Color::Red),
        };

        let labels = item.labels().join(", ");
        let category = item.category().unwrap_or("-");

        // Truncate ID to first part for display
        let short_id = item.id().split('-').next().unwrap_or_else(|| item.id());

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

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
