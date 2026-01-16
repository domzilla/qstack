//! # New Command
//!
//! Creates a new qstack item with the given title.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::collections::HashSet;
use std::io::IsTerminal;

use anyhow::{Context, Result};
use chrono::Utc;
use owo_colors::OwoColorize;

use crate::{
    config::Config,
    editor, id,
    item::{normalize_identifier, Frontmatter, Item, Status},
    storage,
    tui::{self, screens::NewItemWizard},
    ui::{self, InteractiveArgs},
};

/// Arguments for the new command
pub struct NewArgs {
    pub title: Option<String>,
    pub labels: Vec<String>,
    pub category: Option<String>,
    pub attachments: Vec<String>,
    pub interactive: InteractiveArgs,
    pub as_template: bool,
    #[allow(clippy::option_option)]
    pub from_template: Option<Option<String>>,
}

/// Executes the new command.
pub fn execute(args: NewArgs) -> Result<()> {
    let mut config = Config::load()?;

    // Handle --from-template
    if let Some(ref template_ref) = args.from_template {
        return execute_from_template(&mut config, &args, template_ref.as_deref());
    }

    // If no title provided and we're in a terminal, launch the wizard
    if args.title.is_none() {
        if !std::io::stdout().is_terminal() {
            anyhow::bail!("Title is required in non-interactive mode");
        }
        return execute_wizard(&config, args.as_template);
    }

    let title = args.title.unwrap();

    // Validate title is not empty
    if title.trim().is_empty() {
        anyhow::bail!("Title cannot be empty");
    }

    // Validate labels are not empty
    for label in &args.labels {
        if label.trim().is_empty() {
            anyhow::bail!("Label cannot be empty");
        }
    }

    // Validate category is not empty
    if let Some(ref cat) = args.category {
        if cat.trim().is_empty() {
            anyhow::bail!("Category cannot be empty");
        }
    }

    // Get author name (prompts if not available)
    let author = config.user_name_or_prompt()?;

    // Generate ID
    let id = id::generate(config.id_pattern());

    // Normalize labels and category (spaces -> hyphens)
    let labels: Vec<String> = args
        .labels
        .iter()
        .map(|l| normalize_identifier(l))
        .collect();
    let category = args.category.as_deref().map(normalize_identifier);

    // Determine status based on --as-template flag
    let status = if args.as_template {
        Status::Template
    } else {
        Status::Open
    };

    // Create frontmatter
    let frontmatter = Frontmatter {
        id,
        title,
        author,
        created_at: Utc::now(),
        status,
        labels,
        attachments: vec![],
    };

    // Create item
    let mut item = Item::new(frontmatter);

    // Save to disk (category determines folder placement)
    let path = if args.as_template {
        storage::create_template(&config, &item, category.as_deref())?
    } else {
        storage::create_item(&config, &item, category.as_deref())?
    };

    // Process attachments if any
    if !args.attachments.is_empty() {
        ui::process_and_save_attachments(&mut item, &path, &args.attachments)?;
    }

    // Resolve interactive mode (editor doesn't require terminal check)
    let interactive = args.interactive.is_enabled(&config);

    // Open editor if interactive
    if interactive {
        editor::open(&path, &config).context("Failed to open editor")?;
    }

    // Output the path (for scripting)
    println!("{}", config.relative_path(&path).display());

    Ok(())
}

/// Collect existing categories and labels from all items.
pub fn collect_existing_metadata(config: &Config) -> (Vec<String>, Vec<String>) {
    let mut categories: HashSet<String> = HashSet::new();
    let mut labels: HashSet<String> = HashSet::new();

    let paths: Vec<_> = storage::walk_all(config).collect();

    for path in paths {
        if let Ok(item) = Item::load(&path) {
            // Derive category from path
            if let Some(cat) = storage::derive_category(config, &path) {
                categories.insert(cat);
            }
            for label in item.labels() {
                labels.insert(label.clone());
            }
        }
    }

    let mut categories: Vec<_> = categories.into_iter().collect();
    let mut labels: Vec<_> = labels.into_iter().collect();
    categories.sort();
    labels.sort();

    (categories, labels)
}

/// Execute the wizard flow for creating a new item.
fn execute_wizard(config: &Config, as_template: bool) -> Result<()> {
    // Collect existing metadata
    let (existing_categories, existing_labels) = collect_existing_metadata(config);

    // Run the wizard
    let wizard = NewItemWizard::new(existing_categories, existing_labels);
    let Some(output) = tui::run(wizard)? else {
        println!("{}", "Cancelled.".dimmed());
        return Ok(());
    };

    // Get author name
    let mut config = Config::load()?;
    let author = config.user_name_or_prompt()?;

    // Generate ID
    let id = id::generate(config.id_pattern());

    // Normalize labels and category (spaces -> hyphens)
    let labels: Vec<String> = output
        .labels
        .iter()
        .map(|l| normalize_identifier(l))
        .collect();
    let category = output.category.as_deref().map(normalize_identifier);

    // Determine status based on --as-template flag
    let status = if as_template {
        Status::Template
    } else {
        Status::Open
    };

    // Create frontmatter from wizard output
    let frontmatter = Frontmatter {
        id,
        title: output.title,
        author,
        created_at: Utc::now(),
        status,
        labels,
        attachments: vec![],
    };

    // Create item with content
    let mut item = Item::new(frontmatter);
    item.body = output.content;

    // Save to disk (category determines folder placement)
    let path = if as_template {
        storage::create_template(&config, &item, category.as_deref())?
    } else {
        storage::create_item(&config, &item, category.as_deref())?
    };

    // Process attachments
    if !output.attachments.is_empty() {
        ui::process_and_save_attachments(&mut item, &path, &output.attachments)?;
    }

    // Output the path
    println!("{}", config.relative_path(&path).display());

    Ok(())
}

/// Execute the from-template flow.
///
/// If `template_ref` is `None`, shows template selection TUI.
/// Otherwise, loads the template by ID/title reference.
fn execute_from_template(
    config: &mut Config,
    args: &NewArgs,
    template_ref: Option<&str>,
) -> Result<()> {
    // Load template
    let template = if let Some(reference) = template_ref {
        // Direct reference - find by ID or title
        let template_path = storage::find_template(config, reference)?;
        Item::load(&template_path)?
    } else {
        // No reference - show template selection TUI
        if !std::io::stdout().is_terminal() {
            anyhow::bail!("Template reference required in non-interactive mode");
        }
        let Some(selected) = select_template(config)? else {
            println!("{}", "Cancelled.".dimmed());
            return Ok(());
        };
        selected
    };

    // Get template's category (may be inherited)
    let template_category = template
        .path
        .as_ref()
        .and_then(|p| storage::derive_category(config, p));

    // Use CLI category if specified, otherwise inherit from template
    let category = args
        .category
        .as_deref()
        .map(normalize_identifier)
        .or(template_category);

    // Merge labels: template labels + CLI labels
    let mut labels: Vec<String> = template.labels().to_vec();
    for label in &args.labels {
        let normalized = normalize_identifier(label);
        if !labels.contains(&normalized) {
            labels.push(normalized);
        }
    }

    // If no title provided, launch wizard with template data pre-filled
    if args.title.is_none() {
        if !std::io::stdout().is_terminal() {
            anyhow::bail!("Title is required in non-interactive mode");
        }
        return execute_wizard_from_template(config, &template, category.as_deref(), &labels);
    }

    let title = args.title.clone().unwrap();

    // Validate title is not empty
    if title.trim().is_empty() {
        anyhow::bail!("Title cannot be empty");
    }

    // Get author name
    let author = config.user_name_or_prompt()?;

    // Generate new ID
    let id = id::generate(config.id_pattern());

    // Create frontmatter
    let frontmatter = Frontmatter {
        id,
        title,
        author,
        created_at: Utc::now(),
        status: Status::Open,
        labels,
        attachments: vec![],
    };

    // Create item with template's body content
    let mut item = Item::new(frontmatter);
    item.body.clone_from(&template.body);

    // Save to disk
    let path = storage::create_item(config, &item, category.as_deref())?;

    // Process CLI attachments if any
    if !args.attachments.is_empty() {
        ui::process_and_save_attachments(&mut item, &path, &args.attachments)?;
    }

    // Resolve interactive mode
    let interactive = args.interactive.is_enabled(config);

    // Open editor if interactive
    if interactive {
        editor::open(&path, config).context("Failed to open editor")?;
    }

    // Output the path
    println!("{}", config.relative_path(&path).display());

    Ok(())
}

/// Execute wizard flow with template data pre-filled.
fn execute_wizard_from_template(
    config: &Config,
    template: &Item,
    category: Option<&str>,
    labels: &[String],
) -> Result<()> {
    // Collect existing metadata for autocomplete
    let (existing_categories, existing_labels) = collect_existing_metadata(config);

    // Create pre-populated wizard
    let wizard = NewItemWizard::new(existing_categories, existing_labels)
        .with_content(&template.body)
        .with_category(category.map(String::from))
        .with_labels(labels);

    let Some(output) = tui::run(wizard)? else {
        println!("{}", "Cancelled.".dimmed());
        return Ok(());
    };

    // Get author name
    let mut config = Config::load()?;
    let author = config.user_name_or_prompt()?;

    // Generate ID
    let id = id::generate(config.id_pattern());

    // Normalize category
    let category = output.category.as_deref().map(normalize_identifier);

    // Create frontmatter
    let frontmatter = Frontmatter {
        id,
        title: output.title,
        author,
        created_at: Utc::now(),
        status: Status::Open,
        labels: output.labels,
        attachments: vec![],
    };

    // Create item
    let mut item = Item::new(frontmatter);
    item.body = output.content;

    // Save to disk
    let path = storage::create_item(&config, &item, category.as_deref())?;

    // Process attachments
    if !output.attachments.is_empty() {
        ui::process_and_save_attachments(&mut item, &path, &output.attachments)?;
    }

    // Output the path
    println!("{}", config.relative_path(&path).display());

    Ok(())
}

/// Show template selection TUI and return selected template.
fn select_template(config: &Config) -> Result<Option<Item>> {
    let templates: Vec<Item> = storage::walk_templates(config)
        .filter_map(|path| Item::load(&path).ok())
        .collect();

    if templates.is_empty() {
        anyhow::bail!(
            "No templates found. Create one with: qstack new --as-template \"Template Name\""
        );
    }

    let options: Vec<String> = templates
        .iter()
        .map(|t| {
            if t.labels().is_empty() {
                t.title().to_string()
            } else {
                format!("{} [{}]", t.title(), t.labels().join(", "))
            }
        })
        .collect();

    let Some(selection) = ui::select_from_list("Select a template", &options)? else {
        return Ok(None);
    };

    Ok(Some(templates.into_iter().nth(selection).unwrap()))
}
