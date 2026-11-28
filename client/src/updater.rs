//! Background self-updater that checks GitHub Releases on launch and silently
//! installs + relaunches newer Wireline builds on Windows.

use crate::events::UiEvent;
use serde::Deserialize;
use std::sync::mpsc;
use std::time::{Duration, UNIX_EPOCH};

const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/skibidiboydawg/wireline/releases/latest";

#[derive(Deserialize)]
struct GithubRelease {
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    updated_at: String,
    browser_download_url: String,
}

/// Parses an ISO-8601 UTC timestamp (`YYYY-MM-DDTHH:MM:SSZ`) into Unix seconds.
pub fn parse_rfc3339_utc(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.len() < 19 {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: i64 = s.get(5..7)?.parse().ok()?;
    let day: i64 = s.get(8..10)?.parse().ok()?;
    let hour: i64 = s.get(11..13)?.parse().ok()?;
    let min: i64 = s.get(14..16)?.parse().ok()?;
    let sec: i64 = s.get(17..19)?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    // Civil date to days since 1970-01-01 (Howard Hinnant algorithm)
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m_adj = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * m_adj + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hour * 3_600 + min * 60 + sec)
}

fn current_exe_info() -> Option<(i64, bool)> {
    let exe = std::env::current_exe().ok()?;
    let exe_str = exe.to_string_lossy().to_ascii_lowercase();
    let is_dev_build =
        exe_str.contains("\\target\\debug\\") || exe_str.contains("\\target\\release\\");
    let mtime = std::fs::metadata(&exe)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    Some((mtime, is_dev_build))
}

/// Checks GitHub Releases for a newer Windows installer; if `auto_install` is true,
/// downloads it to `%TEMP%\Wireline-AutoUpdate.exe` and signals `UiEvent::UpdateReady`.
pub async fn check_for_update(
    auto_install: bool,
    last_installed_ts: Option<String>,
    manual: bool,
    tx: mpsc::Sender<UiEvent>,
) {
    let Some((exe_mtime_secs, is_dev_build)) = current_exe_info() else {
        return;
    };
    if is_dev_build && !manual {
        return;
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("Wireline-Updater/0.1")
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    let release = match client
        .get(LATEST_RELEASE_API)
        .send()
        .await
        .and_then(|r| r.error_for_status())
    {
        Ok(resp) => match resp.json::<GithubRelease>().await {
            Ok(rel) => rel,
            Err(_) => return,
        },
        Err(_) => {
            if manual {
                let _ = tx.send(UiEvent::Status("Could not reach GitHub Releases.".into()));
            }
            return;
        }
    };

    #[cfg(target_os = "windows")]
    let target_name = "Wireline-Setup.exe";
    #[cfg(not(target_os = "windows"))]
    let target_name = "wireline-linux-x86_64.tar.gz";

    let Some(asset) = release.assets.into_iter().find(|a| a.name == target_name) else {
        if manual {
            let _ = tx.send(UiEvent::Status("You are on the latest version.".into()));
        }
        return;
    };

    let Some(asset_secs) = parse_rfc3339_utc(&asset.updated_at) else {
        return;
    };

    // Already on this release asset?
    if last_installed_ts.as_deref() == Some(asset.updated_at.as_str())
        || asset_secs <= exe_mtime_secs + 90
    {
        let _ = tx.send(UiEvent::UpdateSynced(asset.updated_at));
        if manual {
            let _ = tx.send(UiEvent::Status("Wireline is already up to date.".into()));
        }
        return;
    }

    #[cfg(target_os = "windows")]
    {
        if !auto_install && !manual {
            let _ = tx.send(UiEvent::Status(
                "A new Wireline update is available (enable Auto-Update in Settings).".into(),
            ));
            return;
        }
        let _ = tx.send(UiEvent::Status("Downloading Wireline update...".into()));
        let dl_client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .user_agent("Wireline-Updater/0.1")
            .build()
        {
            Ok(c) => c,
            Err(_) => return,
        };
        let bytes = match dl_client
            .get(&asset.browser_download_url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
        {
            Ok(resp) => match resp.bytes().await {
                Ok(b) => b,
                Err(_) => return,
            },
            Err(_) => return,
        };

        // Sanity-check PE executable header (`MZ`) and minimum size (> 100 KB)
        if bytes.len() < 100_000 || &bytes[..2] != b"MZ" {
            return;
        }

        let dest = std::env::temp_dir().join("Wireline-AutoUpdate.exe");
        if std::fs::write(&dest, &bytes).is_err() {
            return;
        }
        let _ = tx.send(UiEvent::UpdateReady {
            asset_updated_at: asset.updated_at,
            installer_path: dest.to_string_lossy().to_string(),
        });
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (auto_install, asset);
        let _ = tx.send(UiEvent::Status(
            "A new Wireline update is available on GitHub Releases.".into(),
        ));
    }
}

/// Spawns a detached silent installer + relaunch helper on Windows.
#[cfg(target_os = "windows")]
pub fn launch_silent_install_and_relaunch(installer_path: &str) -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let escaped = installer_path.replace('\'', "''");
    let script = format!(
        "Start-Sleep -Milliseconds 700; \
         Start-Process -FilePath '{escaped}' -ArgumentList '/S' -Wait; \
         Remove-Item -LiteralPath '{escaped}' -Force -ErrorAction SilentlyContinue; \
         $exe = Join-Path $env:LOCALAPPDATA 'Programs\\Wireline\\Wireline.exe'; \
         if (Test-Path $exe) {{ Start-Process -FilePath $exe }}"
    );
    std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .is_ok()
}

#[cfg(not(target_os = "windows"))]
pub fn launch_silent_install_and_relaunch(_installer_path: &str) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_github_rfc3339_timestamps() {
        assert_eq!(parse_rfc3339_utc("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_rfc3339_utc("2026-11-28T10:32:10Z"), Some(1_795_861_930));
        assert!(parse_rfc3339_utc("bad").is_none());
    }
}
