//! The `warcraft-survivors` launcher: the build stamp, handed to the survivors mode.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use benilla_app::BuildId;

fn main() -> benilla_app::AppExit {
    // A French pack stays separate from an existing installation and its settings.
    if std::env::var_os("WOW_DATA").is_none() {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let french = dir.join("Data-frFR");
                if french.join("patch-2.MPQ").is_file() {
                    std::env::set_var("WOW_DATA", french);
                }
            }
        }
    }
    benilla_app::run_survivors(BuildId {
        version: env!("CARGO_PKG_VERSION"),
        describe: env!("BENILLA_GIT_DESCRIBE"),
        sha: env!("BENILLA_GIT_SHA"),
        short: env!("BENILLA_GIT_SHORT"),
        date: env!("BENILLA_GIT_DATE"),
        profile: env!("BENILLA_PROFILE"),
        project_dir: env!("BENILLA_PROJECT_DIR"),
        ..Default::default()
    })
}
