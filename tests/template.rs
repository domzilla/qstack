//! # Template Tests
//!
//! Tests for the template feature: creating templates, listing templates,
//! and creating items from templates.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

mod common;

use common::{GlobalConfigBuilder, TestEnv};
use qstack::commands::{self, InteractiveArgs, ListMode, ListOptions, NewArgs, StatusFilter};

// =============================================================================
// Creating Templates
// =============================================================================

#[test]
fn test_new_as_template() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("Bug Report Template".to_string()),
        labels: vec!["bug".to_string()],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: true,
        from_template: None,
    };

    commands::new(args).expect("new should succeed");

    // Template should be in .templates directory
    let templates = env.list_template_files();
    assert_eq!(templates.len(), 1, "Should have one template");

    let content = env.read_item(&templates[0]);
    assert!(
        content.contains("title: Bug Report Template"),
        "Should have correct title"
    );
    assert!(
        content.contains("status: template"),
        "Should have template status"
    );
    assert!(content.contains("- bug"), "Should have bug label");
}

#[test]
fn test_new_as_template_with_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("Feature Request Template".to_string()),
        labels: vec!["feature".to_string()],
        category: Some("features".to_string()),
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: true,
        from_template: None,
    };

    commands::new(args).expect("new should succeed");

    // Template should be in .templates/features directory
    let templates = env.list_template_files();
    assert_eq!(templates.len(), 1, "Should have one template");

    let template_path = &templates[0];
    assert!(
        template_path
            .to_string_lossy()
            .contains(".templates/features/"),
        "Template should be in features category: {}",
        template_path.display()
    );
}

// =============================================================================
// Listing Templates
// =============================================================================

#[test]
fn test_list_templates() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    // Create two templates
    for title in ["Bug Report", "Feature Request"] {
        let args = NewArgs {
            title: Some(title.to_string()),
            labels: vec![],
            category: None,
            attachments: vec![],
            interactive: InteractiveArgs {
                interactive: false,
                no_interactive: true,
            },
            as_template: true,
            from_template: None,
        };
        commands::new(args).expect("new should succeed");
    }

    // List templates
    let options = ListOptions {
        mode: ListMode::Templates,
        status: StatusFilter::Open,
        labels: vec![],
        author: None,
        category: None,
        sort: commands::SortBy::Id,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        id: None,
        file: None,
    };

    // Should not error
    commands::list(&options).expect("list templates should succeed");

    // Verify templates exist
    let templates = env.list_template_files();
    assert_eq!(templates.len(), 2, "Should have two templates");
}

// =============================================================================
// Creating from Templates
// =============================================================================

#[test]
fn test_new_from_template() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    // First, create a template
    let template_args = NewArgs {
        title: Some("Bug Report".to_string()),
        labels: vec!["bug".to_string()],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: true,
        from_template: None,
    };
    commands::new(template_args).expect("create template should succeed");

    // Get template ID for reference
    let templates = env.list_template_files();
    let template_content = env.read_item(&templates[0]);
    let template_id = template_content
        .lines()
        .find(|l| l.starts_with("id:"))
        .and_then(|l| l.strip_prefix("id: "))
        .map(|s| s.trim_matches('\'')) // Remove YAML quotes
        .expect("Should have ID");

    // Create item from template
    let item_args = NewArgs {
        title: Some("Login Bug".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: Some(Some(template_id.to_string())),
    };
    commands::new(item_args).expect("create from template should succeed");

    // Verify item was created (not template)
    let items = env.list_stack_files();
    assert_eq!(items.len(), 1, "Should have one item");

    let item_content = env.read_item(&items[0]);
    assert!(
        item_content.contains("title: Login Bug"),
        "Should have new title"
    );
    assert!(item_content.contains("status: open"), "Should be open");
}

#[test]
fn test_new_from_template_inherits_labels() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    // Create template with labels
    let template_args = NewArgs {
        title: Some("Bug Template".to_string()),
        labels: vec!["bug".to_string(), "needs-triage".to_string()],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: true,
        from_template: None,
    };
    commands::new(template_args).expect("create template should succeed");

    let templates = env.list_template_files();
    let template_content = env.read_item(&templates[0]);
    let template_id = template_content
        .lines()
        .find(|l| l.starts_with("id:"))
        .and_then(|l| l.strip_prefix("id: "))
        .map(|s| s.trim_matches('\'')) // Remove YAML quotes
        .expect("Should have ID");

    // Create item from template with additional label
    let item_args = NewArgs {
        title: Some("Crash Bug".to_string()),
        labels: vec!["critical".to_string()],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: Some(Some(template_id.to_string())),
    };
    commands::new(item_args).expect("create from template should succeed");

    // Verify item has merged labels
    let items = env.list_stack_files();
    let item_content = env.read_item(&items[0]);
    assert!(item_content.contains("- bug"), "Should inherit bug label");
    assert!(
        item_content.contains("- needs-triage"),
        "Should inherit needs-triage label"
    );
    assert!(item_content.contains("- critical"), "Should have CLI label");
}

#[test]
fn test_new_from_template_inherits_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    // Create template in a category
    let template_args = NewArgs {
        title: Some("Bug Template".to_string()),
        labels: vec![],
        category: Some("bugs".to_string()),
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: true,
        from_template: None,
    };
    commands::new(template_args).expect("create template should succeed");

    let templates = env.list_template_files();
    let template_content = env.read_item(&templates[0]);
    let template_id = template_content
        .lines()
        .find(|l| l.starts_with("id:"))
        .and_then(|l| l.strip_prefix("id: "))
        .map(|s| s.trim_matches('\'')) // Remove YAML quotes
        .expect("Should have ID");

    // Create item from template without specifying category
    let item_args = NewArgs {
        title: Some("New Bug".to_string()),
        labels: vec![],
        category: None, // Should inherit from template
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: Some(Some(template_id.to_string())),
    };
    commands::new(item_args).expect("create from template should succeed");

    // Verify item is in the bugs category
    let items = env.list_category_files("bugs");
    assert_eq!(items.len(), 1, "Should have one item in bugs category");
}

// =============================================================================
// Template Exclusion
// =============================================================================

#[test]
fn test_templates_excluded_from_list() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    // Create a regular item
    let item_args = NewArgs {
        title: Some("Regular Item".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: None,
    };
    commands::new(item_args).expect("create item should succeed");

    // Create a template
    let template_args = NewArgs {
        title: Some("Bug Template".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: true,
        from_template: None,
    };
    commands::new(template_args).expect("create template should succeed");

    // List regular items - should only show 1
    let options = ListOptions {
        mode: ListMode::Items,
        status: StatusFilter::Open,
        labels: vec![],
        author: None,
        category: None,
        sort: commands::SortBy::Id,
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        id: None,
        file: None,
    };

    // Verify counts
    let items = env.list_stack_files();
    let templates = env.list_template_files();
    assert_eq!(items.len(), 1, "Should have one regular item");
    assert_eq!(templates.len(), 1, "Should have one template");

    // list command should work without error
    commands::list(&options).expect("list should succeed");
}

#[test]
fn test_find_template_by_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    // Create a template
    let template_args = NewArgs {
        title: Some("Bug Report".to_string()),
        labels: vec!["bug".to_string()],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: true,
        from_template: None,
    };
    commands::new(template_args).expect("create template should succeed");

    // Create item from template by title reference
    let item_args = NewArgs {
        title: Some("My Bug".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        },
        as_template: false,
        from_template: Some(Some("bug report".to_string())), // Case-insensitive title match
    };
    commands::new(item_args).expect("create from template by title should succeed");

    // Verify item was created with inherited label
    let items = env.list_stack_files();
    let item_content = env.read_item(&items[0]);
    assert!(item_content.contains("- bug"), "Should inherit bug label");
}
