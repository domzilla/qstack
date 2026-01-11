//! # Labels Command
//!
//! Lists all unique labels used across items.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::collections::HashMap;

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::{config::Config, storage, ui};

use super::{list, ListFilter, SortBy};

/// Arguments for the labels command
pub struct LabelsArgs {
    pub interactive: bool,
    pub no_interactive: bool,
}

/// Executes the labels command.
pub fn execute(args: &LabelsArgs) -> Result<()> {
    let config = Config::load()?;

    // Collect all items (both open and archived)
    let paths: Vec<_> = storage::walk_items(&config)
        .chain(storage::walk_archived(&config))
        .collect();

    // Count labels
    let mut label_counts: HashMap<String, usize> = HashMap::new();
    for path in paths {
        if let Ok(item) = crate::item::Item::load(&path) {
            for label in item.labels() {
                *label_counts.entry(label.clone()).or_insert(0) += 1;
            }
        }
    }

    if label_counts.is_empty() {
        println!("{}", "No labels found.".dimmed());
        return Ok(());
    }

    // Sort by count (descending), then alphabetically
    let mut labels: Vec<_> = label_counts.into_iter().collect();
    labels.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    // Display table
    print_table(&labels);

    // Check interactive mode
    if !ui::should_run_interactive(args.interactive, args.no_interactive, &config) {
        return Ok(());
    }

    // Interactive selection
    let options: Vec<String> = labels
        .iter()
        .map(|(label, count)| format!("{label} ({count})"))
        .collect();

    let selection = ui::select_from_list("Select a label to filter by", &options)?;
    let selected_label = &labels[selection].0;

    println!("\n{} {}\n", "Items with label:".bold(), selected_label);

    // Show items with selected label using list command
    list::execute(&ListFilter {
        open: false,
        closed: false,
        label: Some(selected_label.clone()),
        author: None,
        sort: SortBy::Id,
        interactive: args.interactive,
        no_interactive: args.no_interactive,
    })?;

    Ok(())
}

fn print_table(labels: &[(String, usize)]) {
    let mut table = ui::create_table();
    table.set_header(vec!["Label", "Count"]);

    for (label, count) in labels {
        table.add_row(vec![label.as_str(), &count.to_string()]);
    }

    println!("{table}");
}
