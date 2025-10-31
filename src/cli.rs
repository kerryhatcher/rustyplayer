use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::player::{Player, PlayerError};

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
    /// Pause playback
    Pause,
    /// Resume playback
    Resume,
    /// Stop playback
    Stop,
    /// Seek to position (in seconds)
    Seek { seconds: u64 },
    /// Scan a directory (import into library)
    Scan { path: PathBuf },
}

pub fn run() -> Result<(), PlayerError> {
    let cli = Cli::parse();
    let player = Player::new()?;

    match cli.command {
        Commands::Play { path } => {
            player.play(&path)?;
            println!("Playing: {}", path.display());
        }
        Commands::Pause => {
            player.pause()?;
            println!("Paused playback");
        }
        Commands::Resume => {
            player.resume()?;
            println!("Resumed playback");
        }
        Commands::Stop => {
            player.stop()?;
            println!("Stopped playback");
        }
        Commands::Seek { seconds } => {
            player.seek(seconds)?;
            println!("Seeking to {}s", seconds);
        }
        Commands::Scan { path } => {
            println!("Scanning directory: {}", path.display());
            // TODO: Implement scanner
        }
    }

    Ok(())
}
