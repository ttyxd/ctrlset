use ctrlset_cli;

fn main() {
    if let Err(e) = ctrlset_cli::run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
