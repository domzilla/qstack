//! # Categories Command
//!
//! Lists all unique categories used across items.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::{collections::HashMap, io::IsTerminal};

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

/// Arguments for the categories command
pub struct CategoriesArgs {
    pub interactive: bool,
    pub no_interactive: bool,
}

/// Executes the categories command.
pub fn execute(args: &CategoriesArgs) -> Result<()> {
    let config = Config::load()?;

    // Collect all items (both open and archived)
    let items: Vec<Item> = storage::walk_items(&config)
        .chain(storage::walk_archived(&config))
        .filter_map(|path| Item::load(&path).ok())
        .collect();

    // Count categories
    let mut category_counts: HashMap<Option<String>, usize> = HashMap::new();
    for item in &items {
        let key = item.category().map(String::from);
        *category_counts.entry(key).or_insert(0) += 1;
    }

    if category_counts.is_empty() {
        println!("{}", "No items found.".dimmed());
        return Ok(());
    }

    // Sort by count (descending), then alphabetically (None last)
    let mut categories: Vec<_> = category_counts.into_iter().collect();
    categories.sort_by(|a, b| {
        b.1.cmp(&a.1).then_with(|| match (&a.0, &b.0) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(a), Some(b)) => a.cmp(b),
        })
    });

    // Display table
    print_table(&categories);

    // Resolve interactive mode
    let interactive = if args.interactive {
        true
    } else if args.no_interactive {
        false
    } else {
        config.interactive()
    };

    if !interactive || !std::io::stdout().is_terminal() {
        return Ok(());
    }

    // Interactive selection
    let selection = interactive_select(&categories)?;
    let selected_category = &categories[selection].0;

    let category_name = selected_category.as_deref().unwrap_or("(uncategorized)");
    println!("\n{} {}\n", "Items in category:".bold(), category_name);

    // Filter and display items in selected category
    let filtered: Vec<&Item> = items
        .iter()
        .filter(|item| item.category().map(String::from) == *selected_category)
        .collect();

    if filtered.is_empty() {
        println!("{}", "No items found.".dimmed());
        return Ok(());
    }

    print_items_table(&filtered);

    // Second interactive selection for items
    if !interactive || !std::io::stdout().is_terminal() {
        return Ok(());
    }

    let item_selection = interactive_item_select(&filtered)?;
    let item = filtered[item_selection];
    let path = item.path.as_ref().context("Item has no path")?;

    println!("{}", config.relative_path(path).display());
    editor::open(path, &config).context("Failed to open editor")?;

    Ok(())
}

fn print_table(categories: &[(Option<String>, usize)]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Category", "Count"]);

    for (category, count) in categories {
        let name = category.as_deref().unwrap_or("(uncategorized)");
        table.add_row(vec![name, &count.to_string()]);
    }

    println!("{table}");
}

fn print_items_table(items: &[&Item]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["ID", "Status", "Title", "Labels"]);

    for item in items {
        let status_cell = match item.status() {
            Status::Open => Cell::new("open").fg(Color::Green),
            Status::Closed => Cell::new("closed").fg(Color::Red),
        };

        let labels = item.labels().join(", ");
        let short_id = item.id().split('-').next().unwrap_or_else(|| item.id());

        table.add_row(vec![
            Cell::new(short_id),
            status_cell,
            Cell::new(truncate(item.title(), 40)),
            Cell::new(truncate(&labels, 20)),
        ]);
    }

    println!("{table}");
}

fn interactive_select(categories: &[(Option<String>, usize)]) -> Result<usize> {
    let options: Vec<String> = categories
        .iter()
        .map(|(cat, count)| {
            let name = cat.as_deref().unwrap_or("(uncategorized)");
            format!("{name} ({count})")
        })
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a category to filter by")
        .items(&options)
        .default(0)
        .interact()
        .context("Selection cancelled")?;

    Ok(selection)
}

fn interactive_item_select(items: &[&Item]) -> Result<usize> {
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

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
