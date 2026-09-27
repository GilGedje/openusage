// Shares crates/tray/ui with the Tauri 2 tray; copy the theme next to it (see crates/tray/build.rs).
fn main() {
    let theme = "../../design/theme.css";
    println!("cargo:rerun-if-changed={theme}");
    std::fs::copy(theme, "../tray/ui/theme.css").expect("copy design/theme.css into the tray UI");
    tauri_build::build()
}
