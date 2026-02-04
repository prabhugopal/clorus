// Optional standalone REPL binary for development and testing
// This is not required for normal usage - the REPL is integrated into the `clorus` CLI

fn main() {
    if let Err(e) = clorus_repl::run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
