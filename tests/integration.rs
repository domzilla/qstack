//! # Integration Tests
//!
//! Comprehensive integration tests for all qstack commands.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

mod harness;

use harness::{create_test_item, GlobalConfigBuilder, ProjectConfigBuilder, TestEnv};
use qstack::commands::{
    self, execute_close, execute_reopen, GetArgs, ListFilter, NewArgs, SearchArgs, SortBy,
    UpdateArgs,
};

// =============================================================================
// Init Command Tests
// =============================================================================

#[test]
fn test_init_creates_project_structure() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());

    // Run init
    commands::init().expect("init should succeed");

    // Verify structure
    assert!(
        env.project_config_path().exists(),
        "Project config should exist"
    );
    assert!(env.stack_path().exists(), "Stack directory should exist");
    assert!(
        env.archive_path().exists(),
        "Archive directory should exist"
    );
}

#[test]
fn test_init_creates_global_config_if_missing() {
    let env = TestEnv::new();
    assert!(
        !env.global_config_path().exists(),
        "Global config should not exist initially"
    );

    // Run init
    commands::init().expect("init should succeed");

    // Global config should be auto-created
    assert!(
        env.global_config_path().exists(),
        "Global config should be created"
    );
    let content = env.read_global_config();
    assert!(
        content.contains("auto_open"),
        "Config should contain auto_open setting"
    );
}

#[test]
fn test_init_fails_if_already_initialized() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());

    // First init
    commands::init().expect("first init should succeed");

    // Second init should fail
    let result = commands::init();
    assert!(result.is_err(), "Second init should fail");
}

// =============================================================================
// New Command Tests
// =============================================================================

#[test]
fn test_new_creates_item() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Test Item".to_string(),
        labels: vec![],
        category: None,
        no_open: true,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    assert_eq!(files.len(), 1, "Should have one item");

    let content = env.read_item(&files[0]);
    assert!(
        content.contains("title: Test Item"),
        "Should have correct title"
    );
    assert!(content.contains("author: Test User"), "Should have author");
    assert!(content.contains("status: open"), "Should be open");
}

#[test]
fn test_new_with_labels() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Bug Report".to_string(),
        labels: vec!["bug".to_string(), "urgent".to_string()],
        category: None,
        no_open: true,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    let content = env.read_item(&files[0]);
    assert!(content.contains("- bug"), "Should have bug label");
    assert!(content.contains("- urgent"), "Should have urgent label");
}

#[test]
fn test_new_with_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Bug in Login".to_string(),
        labels: vec![],
        category: Some("bugs".to_string()),
        no_open: true,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_category_files("bugs");
    assert_eq!(files.len(), 1, "Should have one item in bugs category");
}

#[test]
fn test_new_uses_custom_id_pattern() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().id_pattern("%y%j-%RR").build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Custom ID Item".to_string(),
        labels: vec![],
        category: None,
        no_open: true,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    let filename = files[0].file_name().unwrap().to_str().unwrap();
    // Pattern %y%j-%RR produces 8 characters: YYJJJ-RR
    assert!(filename.len() > 8, "Filename should include ID and slug");
}

#[test]
fn test_new_project_id_pattern_overrides_global() {
    let env = TestEnv::new();
    env.write_global_config(
        &GlobalConfigBuilder::new()
            .id_pattern("%y%m%d-%T%RRR")
            .build(),
    );
    env.write_project_config(&ProjectConfigBuilder::new().id_pattern("PROJ-%RRR").build());
    std::fs::create_dir_all(env.stack_path()).expect("create stack dir");
    std::fs::create_dir_all(env.archive_path()).expect("create archive dir");

    let args = NewArgs {
        title: "Project Pattern".to_string(),
        labels: vec![],
        category: None,
        no_open: true,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    let filename = files[0].file_name().unwrap().to_str().unwrap();
    assert!(
        filename.starts_with("PROJ-"),
        "Should use project ID pattern"
    );
}

// =============================================================================
// List Command Tests
// =============================================================================

#[test]
fn test_list_empty_project() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    let filter = ListFilter {
        open: false,
        closed: false,
        id: None,
        label: None,
        author: None,
        sort: SortBy::Id,
        no_open: true,
    };

    // Should not error even if empty
    let result = commands::list(&filter);
    assert!(result.is_ok(), "list should succeed even if empty");
}

#[test]
fn test_list_shows_open_items() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    // Create test items
    create_test_item(&env, "260101-AAA", "First Task", "open", &[], None);
    create_test_item(&env, "260102-BBB", "Second Task", "open", &[], None);
    create_test_item(&env, "260103-CCC", "Closed Task", "closed", &[], None);

    // Move closed task to archive
    let archive = env.archive_path();
    std::fs::rename(
        env.stack_path().join("260103-CCC-closed-task.md"),
        archive.join("260103-CCC-closed-task.md"),
    )
    .expect("move to archive");

    let filter = ListFilter {
        open: true,
        closed: false,
        id: None,
        label: None,
        author: None,
        sort: SortBy::Id,
        no_open: true,
    };

    // Should succeed (output goes to stdout)
    let result = commands::list(&filter);
    assert!(result.is_ok(), "list should succeed");
}

#[test]
fn test_list_filter_by_label() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Bug Task", "open", &["bug"], None);
    create_test_item(
        &env,
        "260102-BBB",
        "Feature Task",
        "open",
        &["feature"],
        None,
    );

    let filter = ListFilter {
        open: false,
        closed: false,
        id: None,
        label: Some("bug".to_string()),
        author: None,
        sort: SortBy::Id,
        no_open: true,
    };

    let result = commands::list(&filter);
    assert!(result.is_ok(), "list with label filter should succeed");
}

#[test]
fn test_list_show_item_by_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(
        &env,
        "260101-AAA",
        "Target Task",
        "open",
        &["important"],
        None,
    );

    let filter = ListFilter {
        open: false,
        closed: false,
        id: Some("260101".to_string()),
        label: None,
        author: None,
        sort: SortBy::Id,
        no_open: true,
    };

    let result = commands::list(&filter);
    assert!(result.is_ok(), "list with id should succeed");
}

#[test]
fn test_list_sort_by_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Zebra Task", "open", &[], None);
    create_test_item(&env, "260102-BBB", "Alpha Task", "open", &[], None);

    let filter = ListFilter {
        open: false,
        closed: false,
        id: None,
        label: None,
        author: None,
        sort: SortBy::Title,
        no_open: true,
    };

    let result = commands::list(&filter);
    assert!(result.is_ok(), "list with sort should succeed");
}

// =============================================================================
// Get Command Tests
// =============================================================================

#[test]
fn test_get_first_item() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "First", "open", &[], None);
    create_test_item(&env, "260102-BBB", "Second", "open", &[], None);

    let args = GetArgs {
        label: None,
        author: None,
        sort: SortBy::Id,
        no_open: true,
        closed: false,
    };

    let result = commands::get(&args);
    assert!(result.is_ok(), "get should succeed");
}

#[test]
fn test_get_with_label_filter() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Regular", "open", &[], None);
    create_test_item(&env, "260102-BBB", "Important", "open", &["priority"], None);

    let args = GetArgs {
        label: Some("priority".to_string()),
        author: None,
        sort: SortBy::Id,
        no_open: true,
        closed: false,
    };

    let result = commands::get(&args);
    assert!(result.is_ok(), "get with filter should succeed");
}

#[test]
fn test_get_no_items_error() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    let args = GetArgs {
        label: None,
        author: None,
        sort: SortBy::Id,
        no_open: true,
        closed: false,
    };

    let result = commands::get(&args);
    assert!(result.is_err(), "get should fail when no items exist");
}

#[test]
fn test_get_sort_by_date() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Old Task", "open", &[], None);
    create_test_item(&env, "260102-BBB", "New Task", "open", &[], None);

    let args = GetArgs {
        label: None,
        author: None,
        sort: SortBy::Date,
        no_open: true,
        closed: false,
    };

    let result = commands::get(&args);
    assert!(result.is_ok(), "get with date sort should succeed");
}

// =============================================================================
// Search Command Tests
// =============================================================================

#[test]
fn test_search_by_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Login Bug", "open", &[], None);
    create_test_item(&env, "260102-BBB", "Feature Request", "open", &[], None);

    let args = SearchArgs {
        query: "login".to_string(),
        full_text: false,
        no_open: true,
        closed: false,
    };

    let result = commands::search(&args);
    assert!(result.is_ok(), "search should succeed");
}

#[test]
fn test_search_by_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Some Task", "open", &[], None);

    let args = SearchArgs {
        query: "260101".to_string(),
        full_text: false,
        no_open: true,
        closed: false,
    };

    let result = commands::search(&args);
    assert!(result.is_ok(), "search by ID should succeed");
}

#[test]
fn test_search_case_insensitive() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Important Bug", "open", &[], None);

    let args = SearchArgs {
        query: "IMPORTANT".to_string(),
        full_text: false,
        no_open: true,
        closed: false,
    };

    let result = commands::search(&args);
    assert!(result.is_ok(), "search should be case insensitive");
}

#[test]
fn test_search_no_results() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Some Task", "open", &[], None);

    let args = SearchArgs {
        query: "nonexistent".to_string(),
        full_text: false,
        no_open: true,
        closed: false,
    };

    let result = commands::search(&args);
    assert!(result.is_err(), "search with no results should error");
}

// =============================================================================
// Update Command Tests
// =============================================================================

#[test]
fn test_update_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Old Title", "open", &[], None);

    let args = UpdateArgs {
        id: "260101".to_string(),
        title: Some("New Title".to_string()),
        labels: vec![],
        category: None,
        clear_category: false,
    };

    commands::update(args).expect("update should succeed");

    // Verify the file was renamed and content updated
    let item = env.find_item_by_id("260101").expect("item should exist");
    let content = env.read_item(&item);
    assert!(
        content.contains("title: New Title"),
        "Title should be updated"
    );
}

#[test]
fn test_update_add_labels() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &["existing"], None);

    let args = UpdateArgs {
        id: "260101".to_string(),
        title: None,
        labels: vec!["new-label".to_string()],
        category: None,
        clear_category: false,
    };

    commands::update(args).expect("update should succeed");

    let item = env.find_item_by_id("260101").expect("item should exist");
    let content = env.read_item(&item);
    assert!(
        content.contains("- existing"),
        "Original label should remain"
    );
    assert!(content.contains("- new-label"), "New label should be added");
}

#[test]
fn test_update_move_to_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

    let args = UpdateArgs {
        id: "260101".to_string(),
        title: None,
        labels: vec![],
        category: Some("bugs".to_string()),
        clear_category: false,
    };

    commands::update(args).expect("update should succeed");

    let files = env.list_category_files("bugs");
    assert_eq!(files.len(), 1, "Item should be in bugs category");
}

#[test]
fn test_update_clear_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], Some("bugs"));

    let args = UpdateArgs {
        id: "260101".to_string(),
        title: None,
        labels: vec![],
        category: None,
        clear_category: true,
    };

    commands::update(args).expect("update should succeed");

    let stack_files = env.list_stack_files();
    assert_eq!(stack_files.len(), 1, "Item should be in root stack");

    let category_files = env.list_category_files("bugs");
    assert!(category_files.is_empty(), "Category should be empty");
}

#[test]
fn test_update_nonexistent_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    let args = UpdateArgs {
        id: "999999".to_string(),
        title: Some("New Title".to_string()),
        labels: vec![],
        category: None,
        clear_category: false,
    };

    let result = commands::update(args);
    assert!(result.is_err(), "update with nonexistent ID should fail");
}

// =============================================================================
// Close/Reopen Command Tests
// =============================================================================

#[test]
fn test_close_moves_to_archive() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Task to Close", "open", &[], None);

    execute_close("260101").expect("close should succeed");

    let stack_files = env.list_stack_files();
    assert!(stack_files.is_empty(), "Stack should be empty");

    let archive_files = env.list_archive_files();
    assert_eq!(archive_files.len(), 1, "Archive should have one item");

    // Check status was updated
    let content = env.read_item(&archive_files[0]);
    assert!(
        content.contains("status: closed"),
        "Status should be closed"
    );
}

#[test]
fn test_close_item_with_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAA", "Bug Task", "open", &[], Some("bugs"));

    execute_close("260101").expect("close should succeed");

    let category_files = env.list_category_files("bugs");
    assert!(category_files.is_empty(), "Category should be empty");

    let archive_files = env.list_archive_files();
    assert_eq!(archive_files.len(), 1, "Archive should have one item");
}

#[test]
fn test_reopen_moves_from_archive() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    // Create item and close it
    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);
    execute_close("260101").expect("close should succeed");

    // Now reopen
    execute_reopen("260101").expect("reopen should succeed");

    let stack_files = env.list_stack_files();
    assert_eq!(stack_files.len(), 1, "Stack should have one item");

    let archive_files = env.list_archive_files();
    assert!(archive_files.is_empty(), "Archive should be empty");

    // Check status was updated
    let content = env.read_item(&stack_files[0]);
    assert!(content.contains("status: open"), "Status should be open");
}

#[test]
fn test_reopen_restores_category() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    // Create item with category and close it
    create_test_item(&env, "260101-AAA", "Bug Task", "open", &[], Some("bugs"));
    execute_close("260101").expect("close should succeed");

    // Reopen - should restore to category
    execute_reopen("260101").expect("reopen should succeed");

    let category_files = env.list_category_files("bugs");
    assert_eq!(category_files.len(), 1, "Item should be back in category");
}

#[test]
fn test_close_nonexistent_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    let result = execute_close("999999");
    assert!(result.is_err(), "close with nonexistent ID should fail");
}

// =============================================================================
// Config Combination Tests (auto_open + no_open)
// =============================================================================

/// Tests that commands work correctly with auto_open=true and no_open=false.
/// Note: Editor won't actually open in tests because stdout is not a terminal.
#[test]
fn test_config_auto_open_true_no_open_false() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(true).build());
    commands::init().expect("init should succeed");

    // With auto_open=true and no_open=false, editor would open (if terminal)
    let args = NewArgs {
        title: "Test".to_string(),
        labels: vec![],
        category: None,
        no_open: false, // Would open editor if in terminal
    };

    let result = commands::new(args);
    assert!(result.is_ok(), "new should succeed");
    assert_eq!(env.count_all_items(), 1);
}

/// Tests that no_open flag overrides auto_open=true config.
#[test]
fn test_config_auto_open_true_no_open_true() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(true).build());
    commands::init().expect("init should succeed");

    // With no_open=true, editor should never open
    let args = NewArgs {
        title: "Test".to_string(),
        labels: vec![],
        category: None,
        no_open: true, // Overrides auto_open
    };

    let result = commands::new(args);
    assert!(result.is_ok(), "new should succeed");
    assert_eq!(env.count_all_items(), 1);
}

/// Tests that with auto_open=false, editor never opens regardless of no_open.
#[test]
fn test_config_auto_open_false_no_open_false() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    // With auto_open=false, editor should never open
    let args = NewArgs {
        title: "Test".to_string(),
        labels: vec![],
        category: None,
        no_open: false, // Doesn't matter since auto_open is false
    };

    let result = commands::new(args);
    assert!(result.is_ok(), "new should succeed");
    assert_eq!(env.count_all_items(), 1);
}

/// Tests that both auto_open=false and no_open=true definitely prevents editor.
#[test]
fn test_config_auto_open_false_no_open_true() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Test".to_string(),
        labels: vec![],
        category: None,
        no_open: true,
    };

    let result = commands::new(args);
    assert!(result.is_ok(), "new should succeed");
    assert_eq!(env.count_all_items(), 1);
}

/// Tests get command with auto_open configurations.
#[test]
fn test_get_auto_open_combinations() {
    // Test with auto_open=true
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().auto_open(true).build());
        commands::init().expect("init should succeed");
        create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

        let args = GetArgs {
            no_open: true, // Override auto_open
            ..Default::default()
        };

        commands::get(&args).expect("get should succeed");
    }

    // Test with auto_open=false
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
        commands::init().expect("init should succeed");
        create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

        let args = GetArgs {
            no_open: false,
            ..Default::default()
        };

        commands::get(&args).expect("get should succeed");
    }
}

/// Tests list command with auto_open configurations.
#[test]
fn test_list_auto_open_combinations() {
    // Test with auto_open=true, no_open=true (override)
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().auto_open(true).build());
        commands::init().expect("init should succeed");
        create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

        let filter = ListFilter {
            open: false,
            closed: false,
            id: None,
            label: None,
            author: None,
            sort: SortBy::Id,
            no_open: true, // Override auto_open
        };

        commands::list(&filter).expect("list should succeed");
    }

    // Test with auto_open=true, no_open=false (would show interactive selector if terminal)
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().auto_open(true).build());
        commands::init().expect("init should succeed");
        create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

        let filter = ListFilter {
            open: false,
            closed: false,
            id: None,
            label: None,
            author: None,
            sort: SortBy::Id,
            no_open: false, // Would show selector if in terminal
        };

        // Works because we're not in a terminal, so interactive selection is skipped
        commands::list(&filter).expect("list should succeed");
    }

    // Test with auto_open=false, no_open=false (never shows selector)
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
        commands::init().expect("init should succeed");
        create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

        let filter = ListFilter {
            open: false,
            closed: false,
            id: None,
            label: None,
            author: None,
            sort: SortBy::Id,
            no_open: false,
        };

        commands::list(&filter).expect("list should succeed");
    }

    // Test with auto_open=false, no_open=true (definitely no selector)
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
        commands::init().expect("init should succeed");
        create_test_item(&env, "260101-AAA", "Task", "open", &[], None);

        let filter = ListFilter {
            open: false,
            closed: false,
            id: None,
            label: None,
            author: None,
            sort: SortBy::Id,
            no_open: true,
        };

        commands::list(&filter).expect("list should succeed");
    }
}

/// Tests search command with auto_open configurations.
#[test]
fn test_search_auto_open_combinations() {
    // Test with auto_open=true, no_open=true
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().auto_open(true).build());
        commands::init().expect("init should succeed");
        create_test_item(&env, "260101-AAA", "Login Bug", "open", &[], None);

        let args = SearchArgs {
            query: "login".to_string(),
            full_text: false,
            no_open: true,
            closed: false,
        };

        commands::search(&args).expect("search should succeed");
    }

    // Test with auto_open=false
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
        commands::init().expect("init should succeed");
        create_test_item(&env, "260101-AAA", "Login Bug", "open", &[], None);

        let args = SearchArgs {
            query: "login".to_string(),
            full_text: false,
            no_open: false,
            closed: false,
        };

        commands::search(&args).expect("search should succeed");
    }
}

// =============================================================================
// use_git_user Config Tests
// =============================================================================

/// Tests that use_git_user=false prevents using git user.name even if available.
#[test]
fn test_use_git_user_disabled() {
    let env = TestEnv::new();
    // Explicit user_name set, use_git_user=false should not matter
    env.write_global_config(
        &GlobalConfigBuilder::new()
            .user_name("Explicit User")
            .use_git_user(false)
            .build(),
    );
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Test".to_string(),
        labels: vec![],
        category: None,
        no_open: true,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    let content = env.read_item(&files[0]);
    assert!(
        content.contains("author: Explicit User"),
        "Should use explicit user_name"
    );
}

/// Tests that use_git_user=true allows falling back to git config.
/// Note: This test verifies the config is parsed correctly; actual git fallback
/// depends on git being configured on the test machine.
#[test]
fn test_use_git_user_enabled_with_explicit_name() {
    let env = TestEnv::new();
    // When both user_name and use_git_user are set, user_name takes precedence
    env.write_global_config(
        &GlobalConfigBuilder::new()
            .user_name("Config User")
            .use_git_user(true)
            .build(),
    );
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Test".to_string(),
        labels: vec![],
        category: None,
        no_open: true,
    };

    commands::new(args).expect("new should succeed");

    let files = env.list_stack_files();
    let content = env.read_item(&files[0]);
    assert!(
        content.contains("author: Config User"),
        "Explicit user_name should take precedence over git"
    );
}

// =============================================================================
// editor Config Tests
// =============================================================================

/// Tests that custom editor config is parsed correctly.
/// Note: Editor won't actually open in tests (not a terminal), but we verify
/// the config value is stored and retrievable.
#[test]
fn test_custom_editor_config() {
    let env = TestEnv::new();
    env.write_global_config(
        &GlobalConfigBuilder::new()
            .editor("nvim")
            .auto_open(false) // Don't try to open
            .build(),
    );
    commands::init().expect("init should succeed");

    // Verify config was written correctly
    let content = env.read_global_config();
    assert!(
        content.contains("editor = \"nvim\""),
        "Editor should be set in config"
    );
}

/// Tests editor with arguments (like "code --wait").
#[test]
fn test_editor_with_arguments() {
    let env = TestEnv::new();
    env.write_global_config(
        &GlobalConfigBuilder::new()
            .editor("code --wait")
            .auto_open(false)
            .build(),
    );
    commands::init().expect("init should succeed");

    let content = env.read_global_config();
    assert!(
        content.contains("editor = \"code --wait\""),
        "Editor with args should be set"
    );
}

/// Tests that Config::editor() returns the configured value.
#[test]
fn test_config_editor_resolution() {
    use qstack::Config;

    let env = TestEnv::new();
    env.write_global_config(
        &GlobalConfigBuilder::new()
            .editor("custom-editor")
            .auto_open(false)
            .build(),
    );
    commands::init().expect("init should succeed");

    let config = Config::load().expect("load config");
    assert_eq!(
        config.editor(),
        Some("custom-editor".to_string()),
        "Config should return custom editor"
    );
}

// Note: Editor env var fallback (VISUAL/EDITOR) is tested at the unit level
// in src/config/mod.rs. We don't test it here to avoid modifying shell env vars.

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

#[test]
fn test_special_characters_in_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Bug: 100% failure rate (critical!)".to_string(),
        labels: vec![],
        category: None,
        no_open: true,
    };

    commands::new(args).expect("new should succeed with special characters");
    assert_eq!(env.count_all_items(), 1);
}

#[test]
fn test_unicode_in_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: "Support für Umlaute (日本語テスト)".to_string(),
        labels: vec![],
        category: None,
        no_open: true,
    };

    commands::new(args).expect("new should succeed with unicode");
    assert_eq!(env.count_all_items(), 1);
}

#[test]
fn test_empty_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    // Empty title should still work (will create file with just ID)
    let args = NewArgs {
        title: "".to_string(),
        labels: vec![],
        category: None,
        no_open: true,
    };

    commands::new(args).expect("new should succeed with empty title");
    assert_eq!(env.count_all_items(), 1);
}

#[test]
fn test_very_long_title() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    commands::init().expect("init should succeed");

    let long_title = "A".repeat(500);
    let args = NewArgs {
        title: long_title,
        labels: vec![],
        category: None,
        no_open: true,
    };

    commands::new(args).expect("new should succeed with long title");
    assert_eq!(env.count_all_items(), 1);

    // Filename should be truncated
    let files = env.list_stack_files();
    let filename = files[0].file_name().unwrap().to_str().unwrap();
    assert!(filename.len() < 200, "Filename should be truncated");
}

#[test]
fn test_partial_id_matching() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-ABCD", "Task One", "open", &[], None);
    create_test_item(&env, "260201-EFGH", "Task Two", "open", &[], None);

    // Update with partial ID
    let args = UpdateArgs {
        id: "2601".to_string(), // Should match "260101-ABCD"
        title: Some("Updated".to_string()),
        labels: vec![],
        category: None,
        clear_category: false,
    };

    commands::update(args).expect("update with partial ID should succeed");
}

#[test]
fn test_ambiguous_partial_id() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    commands::init().expect("init should succeed");

    create_test_item(&env, "260101-AAAA", "Task One", "open", &[], None);
    create_test_item(&env, "260101-BBBB", "Task Two", "open", &[], None);

    // Ambiguous ID should fail
    let args = UpdateArgs {
        id: "2601".to_string(), // Matches both items
        title: Some("Updated".to_string()),
        labels: vec![],
        category: None,
        clear_category: false,
    };

    let result = commands::update(args);
    assert!(result.is_err(), "update with ambiguous ID should fail");
}

// =============================================================================
// Custom Config Directory Tests
// =============================================================================

#[test]
fn test_custom_stack_directory() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().build());
    env.write_project_config(&ProjectConfigBuilder::new().stack_dir("tasks").build());

    let tasks_dir = env.project_path().join("tasks");
    let archive_dir = tasks_dir.join("archive");
    std::fs::create_dir_all(&tasks_dir).expect("create tasks dir");
    std::fs::create_dir_all(&archive_dir).expect("create archive dir");

    let args = NewArgs {
        title: "Task".to_string(),
        labels: vec![],
        category: None,
        no_open: true,
    };

    commands::new(args).expect("new should succeed");

    // Check item was created in custom directory
    let files: Vec<_> = std::fs::read_dir(&tasks_dir)
        .expect("read tasks dir")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();

    assert_eq!(files.len(), 1, "Item should be in custom stack dir");
}

#[test]
fn test_custom_archive_directory() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().auto_open(false).build());
    env.write_project_config(&ProjectConfigBuilder::new().archive_dir("done").build());

    std::fs::create_dir_all(env.stack_path()).expect("create stack dir");
    std::fs::create_dir_all(env.stack_path().join("done")).expect("create archive dir");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);
    execute_close("260101").expect("close should succeed");

    let done_dir = env.stack_path().join("done");
    let files: Vec<_> = std::fs::read_dir(&done_dir)
        .expect("read done dir")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();

    assert_eq!(files.len(), 1, "Item should be in custom archive dir");
}

// =============================================================================
// Global Config Isolation Tests
// =============================================================================

#[test]
fn test_global_config_isolation() {
    // Verify that tests don't affect each other's global config
    let env1 = TestEnv::new();
    env1.write_global_config(&GlobalConfigBuilder::new().user_name("User One").build());
    drop(env1);

    let env2 = TestEnv::new();
    assert!(
        !env2.global_config_path().exists(),
        "New env should not have previous env's config"
    );
}

#[test]
fn test_different_users_in_parallel() {
    // Note: These run sequentially due to ENV_LOCK, but test the isolation
    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().user_name("Alice").build());
        commands::init().expect("init should succeed");

        let args = NewArgs {
            title: "Alice's Task".to_string(),
            labels: vec![],
            category: None,
            no_open: true,
        };

        commands::new(args).expect("new should succeed");

        let files = env.list_stack_files();
        let content = env.read_item(&files[0]);
        assert!(content.contains("author: Alice"), "Should be Alice's item");
    }

    {
        let env = TestEnv::new();
        env.write_global_config(&GlobalConfigBuilder::new().user_name("Bob").build());
        commands::init().expect("init should succeed");

        let args = NewArgs {
            title: "Bob's Task".to_string(),
            labels: vec![],
            category: None,
            no_open: true,
        };

        commands::new(args).expect("new should succeed");

        let files = env.list_stack_files();
        let content = env.read_item(&files[0]);
        assert!(content.contains("author: Bob"), "Should be Bob's item");
    }
}
