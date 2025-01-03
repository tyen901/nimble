#![allow(warnings)]          // Disables all warnings for the entire file
#![allow(dead_code)]         // For unused code
#![allow(unused_variables)]  // For unused variables
#![allow(unused_imports)]    // For unused imports

use clap::Parser;
use nimble::{Args, run};

fn main() {
    let args = Args::parse();
    if let Err(e) = run(args) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
