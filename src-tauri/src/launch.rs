use crate::installer::{edition_dir, find_binary, make_exec, version_dir};
use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;
use tauri::{AppHandle, Emitter};

pub fn launch(app: &AppHandle, base: &Path, edition: &str, version: &str) -> Result<()> {
    let vdir = version_dir(base, edition, version);
    let bin = find_binary(&vdir).ok_or_else(|| anyhow!("No game binary found for {version}"))?;
    make_exec(&bin)?;

    let cwd = edition_dir(base, edition);
    let mut cmd = Command::new(&bin);
    cmd.current_dir(&cwd);
    let mut child = cmd.spawn()?;

    let _ = app.emit("game-running", true);
    let handle = app.clone();
    std::thread::spawn(move || {
        let _ = child.wait();
        let _ = handle.emit("game-running", false);
    });

    Ok(())
}
