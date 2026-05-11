// Hide the console window on Windows release builds. Has no effect elsewhere.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    spoke_widget_lib::run();
}
