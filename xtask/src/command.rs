use std::process::{Command, Output};

use anyhow::{Context, Result, ensure};

pub(crate) fn output(command: &mut Command) -> Result<Output> {
    let description = format!("{command:?}");
    let output = command
        .output()
        .with_context(|| format!("could not run {description}"))?;
    ensure!(
        output.status.success(),
        "{description} failed ({}):\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output)
}
