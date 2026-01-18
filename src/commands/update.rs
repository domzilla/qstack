//! # Update Command
//!
//! Updates an existing queuestack item.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::path::PathBuf;

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::{config::Config, item::normalize_identifier, storage, ui};

/// Arguments for the update command
pub struct UpdateArgs {
    pub id: Option<String>,
    pub file: Option<PathBuf>,
    pub title: Option<String>,
    pub labels: Vec<String>,
    pub remove_labels: Vec<String>,
    pub category: Option<String>,
    pub remove_category: bool,
}

/// Executes the update command.
pub fn execute(args: UpdateArgs) -> Result<()> {
    // Validate title is not empty (if provided)
    if let Some(ref title) = args.title {
        if title.trim().is_empty() {
            anyhow::bail!("Title cannot be empty");
        }
    }

    // Validate labels are not empty
    for label in &args.labels {
        if label.trim().is_empty() {
            anyhow::bail!("Label cannot be empty");
        }
    }

    // Validate remove labels are not empty
    for label in &args.remove_labels {
        if label.trim().is_empty() {
            anyhow::bail!("Label cannot be empty");
        }
    }

    // Validate category is not empty (if provided)
    if let Some(ref cat) = args.category {
        if cat.trim().is_empty() {
            anyhow::bail!("Category cannot be empty");
        }
    }

    let config = Config::load()?;

    // Resolve item from --id or --file
    let item_ref = storage::ItemRef::from_options(args.id, args.file)?;
    let storage::LoadedItem { mut path, mut item } = item_ref.resolve(&config)?;

    let mut changed = false;
    let old_filename = item.filename();

    // Update title
    if let Some(new_title) = args.title {
        if new_title != item.title() {
            item.set_title(new_title);
            changed = true;
        }
    }

    // Add labels
    for label in &args.labels {
        item.add_label(label);
        changed = true;
    }

    // Remove labels
    for label in &args.remove_labels {
        let normalized = normalize_identifier(label);
        if item.labels().contains(&normalized) {
            item.remove_label(&normalized);
            changed = true;
        }
    }

    // Normalize category
    let new_category = args.category.as_deref().map(normalize_identifier);

    // Check for category change (derived from path, not stored in metadata)
    let current_category = storage::derive_category(&config, &path);
    let category_changed = if args.remove_category {
        current_category.is_some()
    } else if let Some(ref cat) = new_category {
        current_category.as_deref() != Some(cat.as_str())
    } else {
        false
    };

    if category_changed {
        changed = true;
    }

    if !changed {
        println!("{}", "No changes to apply.".dimmed());
        return Ok(());
    }

    // Save updated frontmatter
    item.save(&path)?;

    // Handle filename change (title changed)
    let new_filename = item.filename();
    if old_filename != new_filename {
        path = storage::rename_item(&path, &new_filename)?;
    }

    // Handle category change (move to different directory)
    if category_changed {
        let category = if args.remove_category {
            None
        } else {
            new_category.as_deref()
        };
        let (new_path, warnings) = storage::move_to_category(&config, &path, category)?;
        path = new_path;

        // Print any attachment move warnings
        ui::print_warnings(&warnings);
    }

    ui::print_success("Updated", &config, &path);

    Ok(())
}
