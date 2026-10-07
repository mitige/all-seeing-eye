//! Build script : les batteries embarquées (include_dir!) sont des
//! assets, pas du Rust — sans cette directive, cargo ne recompile pas
//! le crate quand une batterie est ajoutée/modifiée.

fn main() {
    println!("cargo:rerun-if-changed=batteries");
}
