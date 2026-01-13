//! # UI Utilities
//!
//! Shared user interface utilities for interactive dialogs, list formatting,
//! and common UI patterns used across commands.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::io::IsTerminal;

use anyhow::{Context, Result};

use std::path::Path;

use owo_colors::OwoColorize;

use crate::{
    config::Config,
    constants::{UI_LABELS_TRUNCATE_LEN, UI_TITLE_TRUNCATE_LEN},
    editor,
    item::{Item, Status},
    storage::{self, AttachmentResult},
    tui::screens::{
        select_from_list as tui_select, select_from_list_filtered as tui_select_filtered,
        select_from_list_with_header,
    },
};

// =============================================================================
// Aggregation Utilities
// =============================================================================

use std::collections::HashMap;
use std::hash::Hash;

/// Counts occurrences by a single key extracted from each item.
///
/// For items that map to exactly one key (e.g., category).
pub fn count_by<T, K, F>(items: &[T], key_fn: F) -> HashMap<K, usize>
where
    K: Eq + Hash,
    F: Fn(&T) -> K,
{
    let mut counts = HashMap::new();
    for item in items {
        *counts.entry(key_fn(item)).or_insert(0) += 1;
    }
    counts
}

/// Counts occurrences by multiple keys extracted from each item.
///
/// For items that map to multiple keys (e.g., labels).
pub fn count_by_many<T, K, I, F>(items: &[T], keys_fn: F) -> HashMap<K, usize>
where
    K: Eq + Hash,
    I: IntoIterator<Item = K>,
    F: Fn(&T) -> I,
{
    let mut counts = HashMap::new();
    for item in items {
        for key in keys_fn(item) {
            *counts.entry(key).or_insert(0) += 1;
        }
    }
    counts
}

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

// =============================================================================
// Interactive Selection
// =============================================================================

/// Generic interactive selection dialog.
///
/// Returns `Some(index)` if an item was selected, `None` if cancelled.
pub fn select_from_list<T: ToString>(prompt: &str, options: &[T]) -> Result<Option<usize>> {
    tui_select(prompt, options)
}

/// Interactive selection with some items disabled.
///
/// Shows all options but only allows selecting items at `selectable_indices`.
/// Disabled items are shown dimmed and cannot be navigated to.
/// Returns `Some(index)` if an item was selected, `None` if cancelled.
pub fn select_from_list_filtered<T: ToString>(
    prompt: &str,
    options: &[T],
    selectable_indices: &[usize],
) -> Result<Option<usize>> {
    tui_select_filtered(prompt, options, selectable_indices)
}

/// Interactive selection for items - returns index.
///
/// Formats items as columns: ID | Status | Title | Labels | Category
/// Works with both `&[Item]` and `&[&Item]` via `AsRef<Item>`.
/// Returns `Some(index)` if an item was selected, `None` if cancelled.
pub fn select_item<T: AsRef<Item>>(
    prompt: &str,
    items: &[T],
    config: &Config,
) -> Result<Option<usize>> {
    let header = format!(
        "{:<15} {:>6}  {:<40}  {:<20}  {}",
        "ID", "Status", "Title", "Labels", "Category"
    );

    let options: Vec<String> = items
        .iter()
        .map(|item| {
            let item = item.as_ref();
            let status = match item.status() {
                Status::Open => "open",
                Status::Closed => "closed",
            };
            let labels = truncate(&item.labels().join(", "), UI_LABELS_TRUNCATE_LEN);
            let category_opt = item
                .path
                .as_ref()
                .and_then(|p| storage::derive_category(config, p));
            let category = category_opt.as_deref().unwrap_or("-");
            let title = truncate(item.title(), UI_TITLE_TRUNCATE_LEN);
            format!(
                "{:<15} {:>6}  {:<40}  {:<20}  {}",
                item.id(),
                status,
                title,
                labels,
                category
            )
        })
        .collect();

    select_from_list_with_header(prompt, &header, &options)
}

/// Opens an item in the editor and prints its relative path.
pub fn open_item_in_editor(item: &Item, config: &Config) -> Result<()> {
    let path = item.path.as_ref().context("Item has no path")?;
    println!("{}", config.relative_path(path).display());
    editor::open(path, config).context("Failed to open editor")
}

// =============================================================================
// Success Messages
// =============================================================================

/// Prints a success message with an item path.
///
/// Format: `✓ {verb} item: {relative_path}`
pub fn print_success(verb: &str, config: &Config, path: &Path) {
    println!(
        "{} {} item: {}",
        "✓".green(),
        verb,
        config.relative_path(path).display()
    );
}

/// Prints warnings with yellow prefix.
pub fn print_warnings(warnings: &[String]) {
    for warning in warnings {
        eprintln!("{} {}", "warning:".yellow(), warning);
    }
}

// =============================================================================
// Attachment Processing
// =============================================================================

/// Processes attachments and prints results.
///
/// This is a shared utility for `new` and `attach` commands that handles:
/// - Setting up the item's attachment directory
/// - Processing each attachment source
/// - Printing colored output for each result
/// - Saving the updated item
///
/// Returns the number of successfully added attachments.
pub fn process_and_save_attachments(
    item: &mut Item,
    path: &Path,
    sources: &[String],
) -> Result<usize> {
    use crate::storage;

    // Set path so attachment_dir() works
    item.path = Some(path.to_path_buf());

    let item_dir = item
        .attachment_dir()
        .ok_or_else(|| anyhow::anyhow!("Invalid item path"))?
        .to_path_buf();
    let item_id = item.id().to_string();

    let mut added_count = 0;

    for source in sources {
        match storage::process_attachment(source, item, &item_dir, &item_id)? {
            AttachmentResult::UrlAdded(url) => {
                println!("  {} {}", "+".green(), url);
                added_count += 1;
            }
            AttachmentResult::FileCopied { original, new_name } => {
                println!("  {} {} -> {}", "+".green(), original, new_name);
                added_count += 1;
            }
            AttachmentResult::FileNotFound(p) => {
                eprintln!("  {} File not found: {}", "!".yellow(), p);
            }
        }
    }

    // Save updated item with attachments
    item.save(path)?;

    Ok(added_count)
}

// =============================================================================
// String Utilities
// =============================================================================

/// Truncates a string to the specified maximum character count, adding ellipsis if truncated.
pub fn truncate(s: &str, max: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max {
        s.to_string()
    } else {
        // Find the byte index at the (max-1)th character boundary
        let byte_index = s.char_indices().nth(max - 1).map_or(s.len(), |(i, _)| i);
        format!("{}…", &s[..byte_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_ascii() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hell…");
        assert_eq!(truncate("abc", 3), "abc");
        assert_eq!(truncate("abcd", 3), "ab…");
    }

    #[test]
    fn test_truncate_utf8_no_truncation() {
        // No truncation needed - should return as-is
        assert_eq!(truncate("日本語", 5), "日本語");
        assert_eq!(truncate("über", 10), "über");
        assert_eq!(truncate("한글", 3), "한글");
    }

    #[test]
    fn test_truncate_utf8_with_truncation() {
        // Truncate at character boundaries, not byte boundaries
        // Result is (max-1) chars + ellipsis = max chars total
        assert_eq!(truncate("日本語中文", 3), "日本…"); // 2 chars + …
        assert_eq!(truncate("über änderung", 5), "über…"); // 4 chars + …
        assert_eq!(truncate("한글 제목입니다", 4), "한글 …"); // 3 chars + …
        assert_eq!(truncate("العربية", 4), "الع…"); // 3 chars + …
    }

    #[test]
    fn test_truncate_mixed_ascii_utf8() {
        // Mixed content should count characters correctly
        // "Test UTF-8: 日本語" is 15 chars (T,e,s,t, ,U,T,F,-,8,:, ,日,本,語)
        assert_eq!(truncate("Test UTF-8: 日本語", 15), "Test UTF-8: 日本語");
        assert_eq!(truncate("Test UTF-8: 日本語", 14), "Test UTF-8: 日…"); // 13 chars + …
        assert_eq!(truncate("café résumé", 6), "café …"); // 5 chars + …
    }

    #[test]
    fn test_truncate_empty_and_edge_cases() {
        assert_eq!(truncate("", 5), "");
        assert_eq!(truncate("a", 1), "a");
        assert_eq!(truncate("日", 1), "日");
    }
}
