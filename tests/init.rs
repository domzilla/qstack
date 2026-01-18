//! # Init Command Tests
//!
//! Tests for the `qs init` command.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

mod common;

use common::{GlobalConfigBuilder, TestEnv};
use queuestack::commands;

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
fn test_init_fails_without_global_config() {
    let env = TestEnv::new();
    assert!(
        !env.global_config_path().exists(),
        "Global config should not exist initially"
    );

    // Run init - should fail because global config doesn't exist
    let result = commands::init();
    assert!(result.is_err(), "init should fail without global config");

    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("qs setup"),
        "Error should mention running 'qs setup': {err}"
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
