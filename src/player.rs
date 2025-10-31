use std::path::Path;
use anyhow::Result;

/// Minimal Player API stub for MVP scaffolding.
pub struct Player;

impl Player {
    pub fn new() -> Self {
        Self
    }

    pub fn play(&self, path: &Path) -> Result<()> {
        println!("[player] play: {}", path.display());
        // TODO: implement decoding (Symphonia) and output (Rodio)
        Ok(())
    }

    pub fn pause(&self) -> Result<()> {
        println!("[player] pause");
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        println!("[player] stop");
        Ok(())
    }

    pub fn seek(&self, seconds: u64) -> Result<()> {
        println!("[player] seek: {}s", seconds);
        Ok(())
    }
}
