use omnimesh::run as runtime_run;
use omnimesh::config::modes::OmniMeshConfig;
use omnimesh::config::OmnimeshMode;

pub fn run() -> Result<(), String> {
    // Try loading a TOML config file first
    let config_path = std::env::var("OMNIMESH_CONFIG").unwrap_or_else(|_| "omni-mesh.toml".to_string());
    if std::path::Path::new(&config_path).exists() {
        use omnimesh::config::loader::ConfigFile;
        let config_file = ConfigFile::load(std::path::Path::new(&config_path))
            .map_err(|e| format!("Failed to load config '{}': {}", config_path, e))?;
        return runtime_run(config_file.to_runtime_config()?);
    }

    // No config file — fall back to mode selected by env var
    let selected_mode = std::env::var("OMNIMESH_MODE").unwrap_or_else(|_| "development".to_string());
    let mode = match selected_mode.to_lowercase().as_str() {
        "development" => OmnimeshMode::development(),
        "lightweight" => OmnimeshMode::lightweight(),
        "production" => OmnimeshMode::production(),
        _ => {
            eprintln!("Unknown OMNIMESH_MODE='{}'. Falling back to development.", selected_mode);
            OmnimeshMode::development()
        }
    };

    runtime_run(OmniMeshConfig::from_mode(mode))
}
