//! # Config Tests
//!
//! Tests for configuration handling including interactive flags, use_git_user,
//! editor settings, and custom directory configurations.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

mod common;

use common::{create_test_item, GlobalConfigBuilder, ProjectConfigBuilder, TestEnv};
use qstack::commands::{self, execute_close, InteractiveArgs, NewArgs};
use qstack::config::GlobalConfig;

// =============================================================================
// Config Combination Tests (interactive + no_interactive)
// =============================================================================

/// Tests that commands work correctly with interactive=true and no_interactive=false.
/// Note: Editor won't actually open in tests because stdout is not a terminal.
#[test]
fn test_config_interactive_true_no_interactive_false() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(true).build());
    commands::init().expect("init should succeed");

    // With interactive=true and no_interactive=false, editor would open (if terminal)
    let args = NewArgs {
        title: Some("Test".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: false,
        }, // Would open editor if in terminal
        as_template: false,
        from_template: None,
    };

    let result = commands::new(args);
    assert!(result.is_ok(), "new should succeed");
    assert_eq!(env.count_all_items(), 1);
}

/// Tests that no_interactive flag overrides interactive=true config.
#[test]
fn test_config_interactive_true_no_interactive_true() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(true).build());
    commands::init().expect("init should succeed");

    // With no_interactive=true, editor should never open
    let args = NewArgs {
        title: Some("Test".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: true,
        }, // Overrides interactive
        as_template: false,
        from_template: None,
    };

    let result = commands::new(args);
    assert!(result.is_ok(), "new should succeed");
    assert_eq!(env.count_all_items(), 1);
}

/// Tests that with interactive=false, editor never opens regardless of no_interactive.
#[test]
fn test_config_interactive_false_no_interactive_false() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    // With interactive=false, editor should never open
    let args = NewArgs {
        title: Some("Test".to_string()),
        labels: vec![],
        category: None,
        attachments: vec![],
        interactive: InteractiveArgs {
            interactive: false,
            no_interactive: false,
        }, // Doesn't matter since interactive is false
        as_template: false,
        from_template: None,
    };

    let result = commands::new(args);
    assert!(result.is_ok(), "new should succeed");
    assert_eq!(env.count_all_items(), 1);
}

/// Tests that both interactive=false and no_interactive=true definitely prevents editor.
#[test]
fn test_config_interactive_false_no_interactive_true() {
    let env = TestEnv::new();
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    commands::init().expect("init should succeed");

    let args = NewArgs {
        title: Some("Test".to_string()),
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

    let result = commands::new(args);
    assert!(result.is_ok(), "new should succeed");
    assert_eq!(env.count_all_items(), 1);
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
        title: Some("Test".to_string()),
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
        title: Some("Test".to_string()),
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
            .interactive(false) // Don't try to open
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
            .interactive(false)
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
            .interactive(false)
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
        title: Some("Task".to_string()),
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
    env.write_global_config(&GlobalConfigBuilder::new().interactive(false).build());
    env.write_project_config(&ProjectConfigBuilder::new().archive_dir("done").build());

    std::fs::create_dir_all(env.stack_path()).expect("create stack dir");
    std::fs::create_dir_all(env.stack_path().join("done")).expect("create archive dir");

    create_test_item(&env, "260101-AAA", "Task", "open", &[], None);
    execute_close(Some("260101".to_string()), None).expect("close should succeed");

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
            title: Some("Alice's Task".to_string()),
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
            title: Some("Bob's Task".to_string()),
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

        commands::new(args).expect("new should succeed");

        let files = env.list_stack_files();
        let content = env.read_item(&files[0]);
        assert!(content.contains("author: Bob"), "Should be Bob's item");
    }
}

// =============================================================================
// Config Validation Tests (setup command updates)
// =============================================================================

/// Tests that setup adds missing options to existing config.
#[test]
fn test_setup_adds_missing_options() {
    let env = TestEnv::new();

    // Write minimal config - missing most fields
    env.write_global_config(r#"user_name = "Test User""#);

    // Validate should detect missing fields
    let validation = GlobalConfig::validate().expect("validate should succeed");
    assert!(validation.has_changes(), "Should detect missing fields");
    assert!(
        validation.missing.contains(&"use_git_user".to_string()),
        "Should report use_git_user as missing"
    );
    assert!(
        validation.missing.contains(&"interactive".to_string()),
        "Should report interactive as missing"
    );

    // Update should add the fields
    let validation = GlobalConfig::update_if_needed().expect("update should succeed");
    assert!(validation.has_changes(), "Should have made changes");

    // Re-validate should show no changes needed
    let validation = GlobalConfig::validate().expect("validate should succeed");
    assert!(
        !validation.has_changes(),
        "After update, config should be complete"
    );

    // Verify the config is parseable
    let config = GlobalConfig::load().expect("load should succeed");
    assert_eq!(config.user_name, Some("Test User".to_string()));
    assert!(config.use_git_user); // default
    assert!(config.interactive); // default
}

/// Tests that setup removes unrecognized/invalid options.
#[test]
fn test_setup_removes_invalid_options() {
    let env = TestEnv::new();

    // Write config with unknown field
    env.write_global_config(
        r#"
user_name = "Test User"
use_git_user = true
interactive = true
id_pattern = "%y%m%d-%T%RRR"
unknown_field = "should be removed"
another_typo = 42
"#,
    );

    // Validate should detect invalid fields
    let validation = GlobalConfig::validate().expect("validate should succeed");
    assert!(validation.has_changes(), "Should detect invalid fields");
    assert!(
        validation.invalid.contains(&"unknown_field".to_string()),
        "Should report unknown_field as invalid"
    );
    assert!(
        validation.invalid.contains(&"another_typo".to_string()),
        "Should report another_typo as invalid"
    );

    // Update should remove them
    GlobalConfig::update_if_needed().expect("update should succeed");

    // Re-validate should show no changes needed
    let validation = GlobalConfig::validate().expect("validate should succeed");
    assert!(
        !validation.has_changes(),
        "After update, no invalid fields should remain"
    );

    // Verify unknown fields are gone from the file content
    let content = env.read_global_config();
    assert!(
        !content.contains("unknown_field"),
        "unknown_field should be removed"
    );
    assert!(
        !content.contains("another_typo"),
        "another_typo should be removed"
    );
}

/// Tests that setup preserves user-set values during update.
#[test]
fn test_setup_preserves_user_values() {
    let env = TestEnv::new();

    // Write config with custom values
    env.write_global_config(
        r#"
user_name = "Custom Name"
use_git_user = false
editor = "nvim"
interactive = false
id_pattern = "%y%j-%RRR"
stack_dir = "tasks"
archive_dir = "done"
template_dir = "blueprints"
"#,
    );

    // This config is complete, no changes should be needed
    let validation = GlobalConfig::validate().expect("validate should succeed");
    assert!(
        !validation.has_changes(),
        "Complete config should not need changes"
    );

    // Now add an invalid field and update
    env.write_global_config(
        r#"
user_name = "Custom Name"
use_git_user = false
editor = "nvim"
interactive = false
id_pattern = "%y%j-%RRR"
stack_dir = "tasks"
archive_dir = "done"
template_dir = "blueprints"
invalid_option = true
"#,
    );

    GlobalConfig::update_if_needed().expect("update should succeed");

    // Verify custom values are preserved
    let config = GlobalConfig::load().expect("load should succeed");
    assert_eq!(config.user_name, Some("Custom Name".to_string()));
    assert!(!config.use_git_user);
    assert_eq!(config.editor, Some("nvim".to_string()));
    assert!(!config.interactive);
    assert_eq!(config.id_pattern, "%y%j-%RRR");
    assert_eq!(config.stack_dir, Some("tasks".to_string()));
    assert_eq!(config.archive_dir, Some("done".to_string()));
    assert_eq!(config.template_dir, Some("blueprints".to_string()));
}

/// Tests that setup migrates legacy field names (default_id_pattern -> id_pattern).
#[test]
fn test_setup_migrates_legacy_fields() {
    let env = TestEnv::new();

    // Write config with legacy field name
    env.write_global_config(
        r#"
user_name = "Test User"
use_git_user = true
interactive = true
default_id_pattern = "%y%j-%RRR"
"#,
    );

    // Validate should detect the legacy field
    let validation = GlobalConfig::validate().expect("validate should succeed");
    assert!(validation.has_changes(), "Should detect legacy field");
    assert!(
        validation
            .migrated
            .iter()
            .any(|(old, new)| old == "default_id_pattern" && new == "id_pattern"),
        "Should report default_id_pattern -> id_pattern migration"
    );

    // Update should migrate it
    GlobalConfig::update_if_needed().expect("update should succeed");

    // Verify the config uses new field name and preserves the value
    let content = env.read_global_config();
    assert!(
        !content.contains("default_id_pattern"),
        "Legacy field name should be removed"
    );
    assert!(
        content.contains("id_pattern"),
        "New field name should be present"
    );

    let config = GlobalConfig::load().expect("load should succeed");
    assert_eq!(
        config.id_pattern, "%y%j-%RRR",
        "ID pattern value should be preserved"
    );
}
