fn main() {
    if std::env::var_os("DEP_TAURI_DEV").is_none() {
        // Plain Cargo builds do not get the Tauri CLI's dev instruction.
        unsafe {
            std::env::set_var("DEP_TAURI_DEV", "false");
        }
    }
    tauri_build::build();
}
