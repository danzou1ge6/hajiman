use clap::Parser;
use hajiman::cli::{Cli, run};

fn main() {
    match run(Cli::parse()) {
        Ok(()) => {}
        Err(e) => println!("{e}"),
    }
}
