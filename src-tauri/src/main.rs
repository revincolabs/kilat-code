// Hide the console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Claude Code runs this exe as its hook / statusline command.
    if let Some(code) = kilat_lib::helper_cli() {
        std::process::exit(code);
    }
    kilat_lib::run()
}
