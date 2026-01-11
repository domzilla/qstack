//! # New Command
//!
//! Creates a new qstack item with the given title.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use anyhow::{Context, Result};
use chrono::Utc;
use owo_colors::OwoColorize;

use crate::{
    config::Config,
    editor, id,
    item::{Frontmatter, Item, Status},
    storage::{self, AttachmentResult},
    ui,
};

/// Arguments for the new command
pub struct NewArgs {
    pub title: String,
    pub labels: Vec<String>,
    pub category: Option<String>,
    pub attachments: Vec<String>,
    pub interactive: bool,
    pub no_interactive: bool,
}

/// Executes the new command.
pub fn execute(args: NewArgs) -> Result<()> {
    let mut config = Config::load()?;

    // Get author name (prompts if not available)
    let author = config.user_name_or_prompt()?;

    // Generate ID
    let id = id::generate(config.id_pattern());

    // Create frontmatter
    let frontmatter = Frontmatter {
        id,
        title: args.title,
        author,
        created_at: Utc::now(),
        status: Status::Open,
        labels: args.labels,
        category: args.category,
        attachments: vec![],
    };

    // Create item
    let mut item = Item::new(frontmatter);

    // Save to disk
    let path = storage::create_item(&config, &item)?;

    // Process attachments if any
    if !args.attachments.is_empty() {
        let item_dir = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid item path"))?;
        let item_id = item.id().to_string();

        for source in &args.attachments {
            match storage::process_attachment(source, &mut item, item_dir, &item_id)? {
                AttachmentResult::UrlAdded(url) => {
                    println!("  {} {}", "+".green(), url);
                }
                AttachmentResult::FileCopied { original, new_name } => {
                    println!("  {} {} -> {}", "+".green(), original, new_name);
                }
                AttachmentResult::FileNotFound(path) => {
                    eprintln!("  {} File not found: {}", "!".yellow(), path);
                }
            }
        }

        // Save updated item with attachments
        item.save(&path)?;
    }

    // Resolve interactive mode
    let interactive =
        ui::resolve_interactive(args.interactive, args.no_interactive, config.interactive());

    // Open editor if interactive
    if interactive {
        editor::open(&path, &config).context("Failed to open editor")?;
    }

    // Output the path (for scripting)
    println!("{}", config.relative_path(&path).display());

    Ok(())
}
