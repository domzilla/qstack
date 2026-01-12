//! # Search
//!
//! Item search and matching logic.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use super::Item;

/// Check if an item matches the search query (case-insensitive).
///
/// Searches the item's title and ID. When `full_text` is true,
/// also searches the body content.
pub fn matches_query(item: &Item, query: &str, full_text: bool) -> bool {
    let query_lower = query.to_lowercase();

    // Always search title
    if item.title().to_lowercase().contains(&query_lower) {
        return true;
    }

    // Always search ID
    if item.id().to_lowercase().contains(&query_lower) {
        return true;
    }

    // Optionally search body
    if full_text && item.body.to_lowercase().contains(&query_lower) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::item::{Frontmatter, Status};

    fn sample_item(title: &str, body: &str) -> Item {
        let frontmatter = Frontmatter {
            id: "260109-02F7K9M".to_string(),
            title: title.to_string(),
            author: "Test".to_string(),
            created_at: Utc::now(),
            status: Status::Open,
            labels: vec![],
            attachments: vec![],
        };
        let mut item = Item::new(frontmatter);
        item.body = body.to_string();
        item
    }

    #[test]
    fn test_matches_title() {
        let item = sample_item("Fix Login Bug", "");
        assert!(matches_query(&item, "login", false));
        assert!(matches_query(&item, "LOGIN", false)); // case insensitive
        assert!(matches_query(&item, "LoGiN", false)); // mixed case
        assert!(matches_query(&item, "bug", false));
    }

    #[test]
    fn test_matches_id() {
        let item = sample_item("Some Title", "");
        assert!(matches_query(&item, "260109", false));
        assert!(matches_query(&item, "02f7k9m", false)); // case insensitive
    }

    #[test]
    fn test_no_match() {
        let item = sample_item("Fix Login Bug", "Some body content");
        assert!(!matches_query(&item, "xyz", false));
        assert!(!matches_query(&item, "body", false)); // not full_text
    }

    #[test]
    fn test_full_text_matches_body() {
        let item = sample_item("Title", "The error occurs in production");
        assert!(!matches_query(&item, "production", false));
        assert!(matches_query(&item, "production", true));
    }

    #[test]
    fn test_full_text_still_matches_title_and_id() {
        let item = sample_item("Important Task", "Body text");
        assert!(matches_query(&item, "important", true));
        assert!(matches_query(&item, "260109", true));
    }
}
