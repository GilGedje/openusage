// The panel's styling comes from the shared design/theme.css; copy it next to the UI so Tauri
// bundles it (a symlink would break on Windows checkouts).
fn main() {
    let theme = "../../design/theme.css";
    println!("cargo:rerun-if-changed={theme}");
    std::fs::copy(theme, "ui/theme.css").expect("copy design/theme.css into the tray UI");
    tauri_build::build()
}
