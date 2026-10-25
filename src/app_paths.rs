//! Application data directory resolution.
//!
//! Native builds: `~/.Pandabox` (the historical location — unchanged so
//! existing vaults keep working).
//!
//! Flatpak: `$XDG_DATA_HOME/pandabox` (default `~/.local/share/pandabox`).
//! Inside the sandbox `$HOME` points at tmpfs (`$XDG_RUNTIME_DIR/doc`),
//! so anything written there vanishes on reboot; the manifest's
//! `--persist=.local/share/pandabox` makes the `XDG_DATA_HOME` location
//! survive reboots under `~/.var/app/<app-id>/`.

use std::env;
use std::path::{Path, PathBuf};

/// The directory holding the vault database and `sync-config.json`.
pub fn app_data_dir() -> PathBuf {
    if env::var("FLATPAK_ID").is_ok() {
        flatpak_data_dir(env::var("XDG_DATA_HOME").ok().as_deref(), &home_base())
    } else {
        home_base().join("Pandabox")
    }
}

/// Data dir inside a Flatpak: `$XDG_DATA_HOME/pandabox`, or
/// `~/.local/share/pandabox` when `XDG_DATA_HOME` is unset/empty.
fn flatpak_data_dir(xdg_data_home: Option<&str>, home: &Path) -> PathBuf {
    let base = xdg_data_home
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local").join("share"));
    base.join("pandabox")
}

fn home_base() -> PathBuf {
    env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn flatpak_uses_xdg_data_home() {
        let dir = flatpak_data_dir(Some("/run/user/1000/doc/.local/share"), Path::new("/home/u"));
        assert_eq!(dir, PathBuf::from("/run/user/1000/doc/.local/share/pandabox"));
    }

    #[test]
    fn flatpak_defaults_to_home_local_share() {
        let dir = flatpak_data_dir(None, Path::new("/home/u"));
        assert_eq!(dir, PathBuf::from("/home/u/.local/share/pandabox"));
    }

    #[test]
    fn flatpak_ignores_empty_xdg_data_home() {
        let dir = flatpak_data_dir(Some(""), Path::new("/home/u"));
        assert_eq!(dir, PathBuf::from("/home/u/.local/share/pandabox"));
    }
}
