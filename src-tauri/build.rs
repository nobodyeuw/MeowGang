fn main() {
    tauri_build::build();
    
    // Generate version info from tauri.conf.json
    let tauri_config = std::fs::read_to_string("tauri.conf.json")
        .expect("Failed to read tauri.conf.json");
    
    // Parse the JSON to extract version
    let config: serde_json::Value = serde_json::from_str(&tauri_config)
        .expect("Failed to parse tauri.conf.json");
    
    let version = config["version"].as_str()
        .expect("Version not found in tauri.conf.json");
    
    // Write version to a file that can be included at compile time
    std::fs::write("src/version.rs", format!(
        r#"/// Auto-generated version from tauri.conf.json
pub const APP_VERSION: &str = "{}";
"#, version
    )).expect("Failed to write version.rs");
}
