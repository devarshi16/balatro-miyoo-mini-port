// Derived from balatro-port-tui (Apache-2.0). Modified by Producdevity; see NOTICE.

use mlua::prelude::*;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use crate::state::SharedState;

const MIYOO_CLOCK_MIN_MHZ: u32 = 1200;
const MIYOO_CLOCK_MAX_MHZ: u32 = 1600;
const DEFAULT_SYSTEM_DIR: &str = "/mnt/SDCARD/.tmp_update";

fn apply_cpu_clock(
    system_dir: &Path,
    marker_path: Option<&Path>,
    mhz: u32,
) -> Result<bool, String> {
    if !(MIYOO_CLOCK_MIN_MHZ..=MIYOO_CLOCK_MAX_MHZ).contains(&mhz) {
        return Err(format!(
            "CPU clock must be between {MIYOO_CLOCK_MIN_MHZ} and {MIYOO_CLOCK_MAX_MHZ} MHz (got {mhz})"
        ));
    }

    let clock_bin = system_dir.join("bin").join("cpuclock");
    if !clock_bin.is_file() {
        eprintln!(
            "[clock] Onion clock helper unavailable at {}",
            clock_bin.display()
        );
        return Ok(false);
    }

    if let Some(marker) = marker_path {
        if let Err(error) = std::fs::write(marker, b"1\n") {
            eprintln!(
                "[clock] could not write clock marker {}: {error}",
                marker.display()
            );
        }
    }

    let output = match Command::new(&clock_bin).arg(mhz.to_string()).output() {
        Ok(output) => output,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::NotFound | ErrorKind::PermissionDenied
            ) =>
        {
            eprintln!(
                "[clock] Onion clock helper unavailable at {}: {error}",
                clock_bin.display()
            );
            return Ok(false);
        }
        Err(error) => {
            eprintln!(
                "[clock] could not run {} {mhz}: {error}",
                clock_bin.display()
            );
            return Ok(false);
        }
    };

    if output.status.success() {
        let reported = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if reported.is_empty() {
            eprintln!("[clock] CPU clock set to {mhz} MHz");
        } else {
            eprintln!("[clock] CPU clock set to {mhz} MHz (reported: {reported})");
        }
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if stderr.is_empty() {
            eprintln!("[clock] cpuclock {mhz} exited with {}", output.status);
        } else {
            eprintln!(
                "[clock] cpuclock {mhz} failed ({}): {stderr}",
                output.status
            );
        }
        Ok(false)
    }
}

pub fn register(lua: &Lua, love: &LuaTable, _state: Arc<SharedState>) -> LuaResult<()> {
    let sys = lua.create_table()?;

    // love.system.getOS() -> "Linux" to skip Steam initialization in Balatro
    sys.set("getOS", lua.create_function(|_, ()| Ok("Linux"))?)?;

    // love.system.getClipboardText()
    sys.set("getClipboardText", lua.create_function(|_, ()| Ok(""))?)?;

    // love.system.setClipboardText(text)
    sys.set(
        "setClipboardText",
        lua.create_function(|_, _text: String| Ok(()))?,
    )?;

    // love.system.openURL(url)
    sys.set("openURL", lua.create_function(|_, _url: String| Ok(false))?)?;

    // Port extension: switch OnionOS CPU clock at runtime when available.
    sys.set(
        "_setCpuClock",
        lua.create_function(|_, mhz: u32| {
            let system_dir = std::env::var_os("BALATRO_SYSTEM_DIR")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(DEFAULT_SYSTEM_DIR));
            let marker = std::env::var_os("BALATRO_CLOCK_MARKER")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from);
            apply_cpu_clock(&system_dir, marker.as_deref(), mhz).map_err(LuaError::runtime)
        })?,
    )?;

    love.set("system", sys)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::apply_cpu_clock;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(prefix: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("{prefix}-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn rejects_out_of_range_frequencies() {
        let dir = TempDir::new("balatro-clock-range");
        let marker = dir.0.join("marker");
        assert!(apply_cpu_clock(&dir.0, Some(&marker), 1199).is_err());
        assert!(apply_cpu_clock(&dir.0, Some(&marker), 1601).is_err());
        assert!(!marker.exists());
    }

    #[test]
    fn missing_helper_returns_false_without_failing() {
        let dir = TempDir::new("balatro-clock-missing");
        let marker = dir.0.join("marker");
        assert_eq!(apply_cpu_clock(&dir.0, Some(&marker), 1500).unwrap(), false);
        assert!(!marker.exists());
    }

    #[cfg(unix)]
    #[test]
    fn runs_helper_and_handles_exit_status() {
        let dir = TempDir::new("balatro-clock-helper");
        let bin_dir = dir.0.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let log_path = dir.0.join("clock.log");
        let marker = dir.0.join("marker");
        let helper = bin_dir.join("cpuclock");
        fs::write(
            &helper,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$1\" >> \"{}\"\nif [ \"$1\" = 1500 ]; then echo 1500; exit 0; fi\nexit 1\n",
                log_path.display()
            ),
        )
        .unwrap();
        let mut perms = fs::metadata(&helper).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&helper, perms).unwrap();

        assert_eq!(apply_cpu_clock(&dir.0, Some(&marker), 1500).unwrap(), true);
        assert!(marker.exists());
        assert_eq!(apply_cpu_clock(&dir.0, Some(&marker), 1200).unwrap(), false);
        assert_eq!(fs::read_to_string(&log_path).unwrap(), "1500\n1200\n");
    }
}
