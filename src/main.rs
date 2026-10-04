//! Binaire `all-seeing-eye` — toute la logique vit dans
//! [`all_seeing_eye::cli`] (lib) ; ici : parsing fatal de clap, code
//! de sortie, erreur FR propre sur stderr.

use all_seeing_eye::cli::Cli;
use clap::Parser;

fn main() {
    match Cli::parse().execute() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("erreur : {e:#}");
            std::process::exit(1);
        }
    }
}
