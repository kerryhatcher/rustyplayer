use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlayerError {
    #[error("Audio feature not enabled")]
    AudioDisabled,
    #[error("No audio device found")]
    NoAudioDevice,
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("Audio error: {0}")]
    AudioError(String),
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

/// Current state of the player
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerState {
    Stopped,
    Playing,
    Paused,
}

/// Player configuration and state
pub struct Player {
    #[cfg(feature = "audio")]
    inner: Arc<Mutex<PlayerInner>>,
    #[cfg(not(feature = "audio"))]
    state: PlayerState,
}

#[cfg(feature = "audio")]
mod audio {
    use super::*;
    use rodio::{OutputStream, Sink};
    use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;
    use std::fs::File;
    use std::io::BufReader;
    use std::time::Duration;

    pub(crate) struct PlayerInner {
        _stream: OutputStream,
        sink: Option<Sink>,
        state: PlayerState,
    }

    impl PlayerInner {
        pub fn new() -> Result<Self, PlayerError> {
            let (_stream, stream_handle) = OutputStream::try_default()
                .map_err(|e| PlayerError::NoAudioDevice)?;
            
            Ok(Self {
                _stream,
                sink: None,
                state: PlayerState::Stopped,
            })
        }

        pub fn play(&mut self, path: &Path) -> Result<(), PlayerError> {
            // Stop any existing playback
            self.stop()?;

            // Open the media file
            let file = File::open(path)
                .map_err(|_| PlayerError::FileNotFound(path.display().to_string()))?;
            
            let mss = MediaSourceStream::new(
                Box::new(BufReader::new(file)),
                Default::default(),
            );

            // Create a hint to help the format registry guess what format reader is appropriate
            let mut hint = Hint::new();
            if let Some(extension) = path.extension() {
                if let Some(ext_str) = extension.to_str() {
                    hint.with_extension(ext_str);
                }
            }

            // Use the default options for metadata and format reading
            let format_opts: FormatOptions = Default::default();
            let metadata_opts: MetadataOptions = Default::default();

            // Probe the media format
            let probed = symphonia::default::get_probe()
                .format(&hint, mss, &format_opts, &metadata_opts)
                .map_err(|_| PlayerError::UnsupportedFormat(path.display().to_string()))?;

            // Get the format reader
            let mut format = probed.format;

            // Find the first audio track
            let track = format
                .tracks()
                .iter()
                .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
                .ok_or_else(|| PlayerError::UnsupportedFormat("No audio track found".into()))?;

            // Create a decoder for the track
            let decoder_opts: DecoderOptions = Default::default();
            let _decoder = symphonia::default::get_codecs()
                .make(&track.codec_params, &decoder_opts)
                .map_err(|_| PlayerError::UnsupportedFormat("Failed to create decoder".into()))?;

            // TODO: Wire up the decoder to rodio sink for actual playback
            // This is where we'll need to implement a custom Source that reads from the decoder
            
            self.state = PlayerState::Playing;
            Ok(())
        }

        pub fn pause(&mut self) -> Result<(), PlayerError> {
            if let Some(sink) = &self.sink {
                sink.pause();
                self.state = PlayerState::Paused;
                Ok(())
            } else {
                Err(PlayerError::InvalidState("No active playback".into()))
            }
        }

        pub fn resume(&mut self) -> Result<(), PlayerError> {
            if let Some(sink) = &self.sink {
                sink.play();
                self.state = PlayerState::Playing;
                Ok(())
            } else {
                Err(PlayerError::InvalidState("No active playback".into()))
            }
        }

        pub fn stop(&mut self) -> Result<(), PlayerError> {
            if let Some(sink) = &self.sink {
                sink.stop();
                self.state = PlayerState::Stopped;
            }
            self.sink = None;
            Ok(())
        }

        pub fn seek(&mut self, _seconds: u64) -> Result<(), PlayerError> {
            // TODO: Implement seeking using symphonia's seek API
            Err(PlayerError::AudioError("Seeking not yet implemented".into()))
        }

        pub fn state(&self) -> PlayerState {
            self.state
        }
    }
}

impl Player {
    pub fn new() -> Result<Self, PlayerError> {
        #[cfg(feature = "audio")]
        {
            let inner = audio::PlayerInner::new()?;
            Ok(Self {
                inner: Arc::new(Mutex::new(inner)),
            })
        }
        #[cfg(not(feature = "audio"))]
        {
            Ok(Self {
                state: PlayerState::Stopped,
            })
        }
    }

    pub fn play(&self, path: &Path) -> Result<(), PlayerError> {
        #[cfg(feature = "audio")]
        {
            self.inner.lock().unwrap().play(path)
        }
        #[cfg(not(feature = "audio"))]
        {
            Err(PlayerError::AudioDisabled)
        }
    }

    pub fn pause(&self) -> Result<(), PlayerError> {
        #[cfg(feature = "audio")]
        {
            self.inner.lock().unwrap().pause()
        }
        #[cfg(not(feature = "audio"))]
        {
            Err(PlayerError::AudioDisabled)
        }
    }

    pub fn resume(&self) -> Result<(), PlayerError> {
        #[cfg(feature = "audio")]
        {
            self.inner.lock().unwrap().resume()
        }
        #[cfg(not(feature = "audio"))]
        {
            Err(PlayerError::AudioDisabled)
        }
    }

    pub fn stop(&self) -> Result<(), PlayerError> {
        #[cfg(feature = "audio")]
        {
            self.inner.lock().unwrap().stop()
        }
        #[cfg(not(feature = "audio"))]
        {
            Err(PlayerError::AudioDisabled)
        }
    }

    pub fn seek(&self, seconds: u64) -> Result<(), PlayerError> {
        #[cfg(feature = "audio")]
        {
            self.inner.lock().unwrap().seek(seconds)
        }
        #[cfg(not(feature = "audio"))]
        {
            Err(PlayerError::AudioDisabled)
        }
    }

    pub fn state(&self) -> PlayerState {
        #[cfg(feature = "audio")]
        {
            self.inner.lock().unwrap().state()
        }
        #[cfg(not(feature = "audio"))]
        {
            self.state
        }
    }
}
