//! # Editor Integration
//!
//! Launches the user's preferred editor for editing items.
//!
//! Copyright (c) 2025 Dominic Rodemer. All rights reserved.
//! Licensed under the MIT License.

use std::{io::IsTerminal, path::Path, process::Command};

use anyhow::{Context, Result};

use crate::config::Config;

/// Opens a file in the user's configured editor.
///
/// The editor is determined by (in order of priority):
/// 1. `editor` setting in config
/// 2. `$VISUAL` environment variable
/// 3. `$EDITOR` environment variable
/// 4. Fallback to `vi`
///
/// The editor is only launched if stdout is a terminal.
pub fn open(path: &Path, config: &Config) -> Result<()> {
    // Skip if not running in a terminal
    if !std::io::stdout().is_terminal() {
        return Ok(());
    }

    let editor = config.editor().unwrap_or_else(|| "vi".to_string());

    // Parse editor command with proper shell quoting (e.g., `nvim -c ":normal G"`)
    let parts = shlex::split(&editor).context("Invalid editor command syntax")?;
    let (program, args) = parts.split_first().context("Empty editor command")?;

    let mut cmd = Command::new(program);
    cmd.args(args).arg(path);

    let status = cmd
        .status()
        .with_context(|| format!("Failed to launch editor: {editor}"))?;

    if !status.success() {
        anyhow::bail!("Editor exited with error: {status}");
    }

    Ok(())
}
