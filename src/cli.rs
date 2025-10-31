use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::PathBuf;

use crate::player::Player;

#[derive(Parser, Debug)]
#[command(name = "rustyplayer", version, about = "A small Rust media player MVP")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Play a file
    Play { path: PathBuf },
    /// Scan a directory (import into library)
    Scan { path: PathBuf },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let player = Player::new();

    match cli.command {
        Commands::Play { path } => player.play(&path)?,
        Commands::Scan { path } => println!("scan: {}", path.display()),
    }

    Ok(())
}
