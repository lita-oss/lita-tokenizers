use std::path::PathBuf;

use anyhow::Result;

use crate::{registry::CratesIo, release, workspace::Workspace};

#[derive(Debug, Default)]
pub struct Arg {
    pub manifest_path: Option<PathBuf>,
}

pub fn run(arg: Arg) -> Result<()> {
    let workspace = Workspace::load(arg.manifest_path)?;
    let plan = release::prepare(&workspace, &CratesIo::new()?)?;
    plan.print();
    Ok(())
}
