use omnimesh::config::OmnimeshMode;
use omnimesh::run as runtime_run;

pub fn run() -> Result<(), String> {
    let selected_mode = std::env::var("OMNIMESH_MODE").unwrap_or_else(|_| "development".to_string());

    let config = match selected_mode.to_lowercase().as_str() {
        "development" => OmnimeshMode::development(),
        "lightweight" => OmnimeshMode::lightweight(),
        "production" => OmnimeshMode::production(),
        _ => {
            eprintln!("Unknown OMNIMESH_MODE='{}'. Falling back to development.", selected_mode);
            OmnimeshMode::development()
        }
    };

    runtime_run(config)
}
