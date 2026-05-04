mod cli;

use crate::cli::run;

fn main() {
    if let Err(error) = run() {
        eprintln!("Failed to start OMNI-MESH: {error}");
        std::process::exit(1);
    }
}
