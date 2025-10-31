use anyhow::Result;

fn main() -> Result<()> {
    // Delegate to the CLI runner in the library crate.
    rustyplayer::cli::run()?;
    Ok(())
}
