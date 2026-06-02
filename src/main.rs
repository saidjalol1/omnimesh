use clap::Parser;
use omnimesh::config::loader::ConfigFile;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "OMNI-MESH V7 Runtime Daemon", long_about = None)]
struct Args {
    /// Path to omni-mesh.toml configuration file
    #[arg(short, long, default_value = "omni-mesh.toml")]
    config: PathBuf,
}

fn main() -> Result<(), String> {
    let args = Args::parse();

    println!("Starting OMNI-MESH Daemon...");

    let config_file =
        ConfigFile::load(&args.config).map_err(|e| format!("Failed to load config: {}", e))?;

    let runtime_config = config_file.to_runtime_config()?;

    println!("Node ID: {:?}", runtime_config.node_id);
    println!("Operating Mode: {:?}", runtime_config.mode);

    // Pass config to runtime (will be fully async later)
    if let Err(e) = omnimesh::runtime::run(runtime_config.mode, runtime_config.node_id) {
        eprintln!("Daemon crashed: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
