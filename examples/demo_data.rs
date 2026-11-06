//! Generate the demo database on disk.
//!
//! ```text
//! cargo run --example demo_data -- <output folder>
//! ```
//!
//! The same data is available in the app via the start screen's "Try the demo" button.

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let out: std::path::PathBuf = std::env::args().nth(1).context("usage: demo_data <output folder>")?.into();
    let demo = fintrack::demo::generate(&out)?;
    println!("Created {}", demo.database.display());
    println!("Sample CSVs: {}", demo.sample_csvs.display());
    Ok(())
}
