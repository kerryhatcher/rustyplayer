use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
    #[error("Decode error: {0}")]
    DecodeError(String),
    #[error("Invalid volume value: {0}")]
    InvalidVolume(f32),
}

/// Current state of the player
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerState {
    Stopped,
    Playing,
    Paused,
}

/// Status information about the current playback
#[derive(Debug, Clone)]
pub struct PlayerStatus {
    pub state: PlayerState,
    pub position: Option<Duration>,
    pub duration: Option<Duration>,
    pub current_file: Option<PathBuf>,
    pub volume: f32,
}

/// Player configuration and state
pub struct Player {
    #[cfg(feature = "audio")]
    inner: Arc<Mutex<PlayerInner>>,
    #[cfg(not(feature = "audio"))]
    state: PlayerState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_player_no_audio() {
        let player = Player::new().expect("Failed to create player");
        assert_eq!(player.state(), PlayerState::Stopped);
        
        let result = player.play(Path::new("nonexistent.mp3"));
        #[cfg(not(feature = "audio"))]
        assert!(matches!(result.unwrap_err(), PlayerError::AudioDisabled));
    }

    #[test]
    fn test_nonexistent_file() {
        let player = Player::new().expect("Failed to create player");
        let result = player.play(Path::new("nonexistent.mp3"));
        
        #[cfg(feature = "audio")]
        assert!(matches!(result.unwrap_err(), PlayerError::FileNotFound(_)));
        #[cfg(not(feature = "audio"))]
        assert!(matches!(result.unwrap_err(), PlayerError::AudioDisabled));
    }

    #[test]
    fn test_empty_file() {
        // Create an empty temporary file
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let result = Player::new()
            .expect("Failed to create player")
            .play(temp_file.path());

        #[cfg(feature = "audio")]
        assert!(matches!(result.unwrap_err(), PlayerError::UnsupportedFormat(_)));
        #[cfg(not(feature = "audio"))]
        assert!(matches!(result.unwrap_err(), PlayerError::AudioDisabled));
    }

    #[test]
    fn test_status_tracking() {
        let player = Player::new().expect("Failed to create player");
        let initial_status = player.status();
        assert_eq!(initial_status.state, PlayerState::Stopped);
        assert!(initial_status.position.is_none());
        assert!(initial_status.duration.is_none());
        assert!(initial_status.current_file.is_none());

        // Create a test file
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let _ = player.play(temp_file.path()); // This will fail but should update state

        let status = player.status();
        #[cfg(not(feature = "audio"))]
        assert_eq!(status.state, PlayerState::Stopped);
        assert!(status.position.is_none());
        assert!(status.duration.is_none());
        
        #[cfg(not(feature = "audio"))]
        assert!(status.current_file.is_none());
    }
}

#[cfg(feature = "audio")]
mod audio {
    use super::*;
    use rodio::{OutputStream, OutputStreamHandle, Sample, Sink, Source};
    use symphonia::core::audio::{AudioBufferRef, Signal};
    use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
    use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;
    use symphonia::core::units::Time;
    use std::fs::File;
    use std::io::BufReader;
    use std::time::Duration;

    /// Custom Source implementation that bridges Symphonia's decoder with Rodio
    #[derive(Clone)]
    struct SymphoniaDecoder {
        decoder: Arc<Mutex<Box<dyn symphonia::core::codecs::Decoder>>>,
        format: Arc<Mutex<Box<dyn symphonia::core::formats::FormatReader>>>,
        current_frame: Arc<Mutex<Option<AudioBufferRef<'static>>>>,
        frame_offset: Arc<Mutex<usize>>,
        sample_rate: u32,
        channels: u16,
        track_id: u32,
        duration: Option<Duration>,
    }

    impl SymphoniaDecoder {
        fn new(
            mut format: Box<dyn symphonia::core::formats::FormatReader>,
            decoder: Box<dyn symphonia::core::codecs::Decoder>,
            track_id: u32,
            sample_rate: u32,
            channels: u16,
        ) -> Self {
            // Try to get track duration if available
            let duration = format
                .metadata()
                .current()
                .and_then(|m| m.duration())
                .map(|time| Duration::from_secs_f64(time.seconds as f64));

            Self {
                decoder: Arc::new(Mutex::new(decoder)),
                format: Arc::new(Mutex::new(format)),
                current_frame: Arc::new(Mutex::new(None)),
                frame_offset: Arc::new(Mutex::new(0)),
                sample_rate,
                channels,
                track_id,
                duration,
            }
        }

        fn seek(&mut self, time: u64) -> Result<(), PlayerError> {
            // Convert seconds to timestamp
            let ts = Time::new(time as u64, 1);
            
            // Attempt to seek in the format reader
            match self.format.lock().unwrap().seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: ts,
                    track_id: self.track_id,
                },
            ) {
                Ok(seeked_to) => {
                    // Clear current frame as it's no longer valid
                    *self.current_frame.lock().unwrap() = None;
                    *self.frame_offset.lock().unwrap() = 0;
                    
                    // Verify we seeked to approximately where we wanted
                    if (seeked_to.actual_ts.seconds as i64 - time as i64).abs() > 2 {
                        return Err(PlayerError::AudioError(
                            format!("Seek was not accurate: requested {}s, got {}s",
                                time, seeked_to.actual_ts.seconds)
                        ));
                    }
                    
                    Ok(())
                }
                Err(err) => Err(PlayerError::AudioError(
                    format!("Failed to seek: {}", err)
                )),
            }
        }

        fn next_frame(&mut self) -> Result<bool, PlayerError> {
            loop {
                let packet = match self.format.lock().unwrap().next_packet() {
                    Ok(packet) => packet,
                    Err(_) => return Ok(false),
                };

                let decoded = self.decoder.lock().unwrap().decode(&packet).map_err(|e| {
                    PlayerError::DecodeError(format!("Failed to decode audio frame: {}", e))
                })?;

                *self.current_frame.lock().unwrap() = Some(decoded);
                *self.frame_offset.lock().unwrap() = 0;
                return Ok(true);
            }
        }
    }

    impl Iterator for SymphoniaDecoder {
        type Item = f32;

        fn next(&mut self) -> Option<f32> {
            loop {
                // If we have a frame, try to get the next sample
                if let Some(frame) = self.current_frame.lock().unwrap().as_ref() {
                    let offset = *self.frame_offset.lock().unwrap();
                    if offset < frame.frames() * frame.spec().channels.count() {
                        let sample = match frame.planes().planes()[0].as_slice::<f32>() {
                            Ok(plane) => plane[offset],
                            Err(_) => return None,
                        };
                        *self.frame_offset.lock().unwrap() += 1;
                        return Some(sample);
                    }
                }

                // Need a new frame
                match self.next_frame() {
                    Ok(true) => continue,  // Got a new frame, try again
                    _ => return None,      // End of stream or error
                }
            }
        }
    }

    impl Source for SymphoniaDecoder {
        fn current_frame_len(&self) -> Option<usize> {
            self.current_frame.as_ref().map(|f| f.frames())
        }

        fn channels(&self) -> u16 {
            self.channels
        }

        fn sample_rate(&self) -> u32 {
            self.sample_rate
        }

        fn total_duration(&self) -> Option<Duration> {
            self.duration
        }
    }

    pub(crate) struct PlayerInner {
        _stream: OutputStream,
        stream_handle: OutputStreamHandle,
        sink: Option<Sink>,
        decoder: Arc<Mutex<Option<SymphoniaDecoder>>>,
        state: PlayerState,
        current_file: Option<PathBuf>,
        start_time: Option<std::time::Instant>,
        paused_position: Option<Duration>,
        volume: f32,
    }

    impl PlayerInner {
        pub fn new() -> Result<Self, PlayerError> {
            let (_stream, stream_handle) = OutputStream::try_default()
                .map_err(|e| PlayerError::NoAudioDevice)?;
            
            Ok(Self {
                _stream,
                stream_handle,
                sink: None,
                decoder: Arc::new(Mutex::new(None)),
                state: PlayerState::Stopped,
                current_file: None,
                start_time: None,
                paused_position: None,
                volume: 1.0,
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
            let format = probed.format;

            // Find the first audio track
            let track = format
                .tracks()
                .iter()
                .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
                .ok_or_else(|| PlayerError::UnsupportedFormat("No audio track found".into()))?;

            let track_id = track.id;
            
            // Get audio parameters
            let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
            let channels = track.codec_params.channels.unwrap_or(2) as u16;

            // Create a decoder for the track
            let decoder_opts: DecoderOptions = Default::default();
            let decoder = symphonia::default::get_codecs()
                .make(&track.codec_params, &decoder_opts)
                .map_err(|_| PlayerError::UnsupportedFormat("Failed to create decoder".into()))?;

            // Create our custom decoder that implements rodio::Source
            let source = SymphoniaDecoder::new(
                format,
                decoder,
                track_id,
                sample_rate,
                channels,
            );

            // Store the decoder for seeking
            *self.decoder.lock().unwrap() = Some(source.clone());

            // Create and configure the Rodio sink
            let sink = Sink::try_new(&self.stream_handle)
                .map_err(|e| PlayerError::AudioError(format!("Failed to create audio sink: {}", e)))?;

            sink.append(source);
            sink.play();

            self.sink = Some(sink);
            self.state = PlayerState::Playing;
            self.current_file = Some(path.to_owned());
            self.start_time = Some(std::time::Instant::now());
            self.paused_position = None;
            
            Ok(())
        }

        pub fn pause(&mut self) -> Result<(), PlayerError> {
            if let Some(sink) = &self.sink {
                sink.pause();
                self.state = PlayerState::Paused;
                
                // Store current position when pausing
                if let Some(start_time) = self.start_time {
                    let current_pos = start_time.elapsed();
                    self.paused_position = Some(current_pos);
                    self.start_time = None;
                }
                
                Ok(())
            } else {
                Err(PlayerError::InvalidState("No active playback".into()))
            }
        }

        pub fn resume(&mut self) -> Result<(), PlayerError> {
            if let Some(sink) = &self.sink {
                sink.play();
                self.state = PlayerState::Playing;
                
                // Resume timing from paused position
                self.start_time = Some(std::time::Instant::now()
                    .checked_sub(self.paused_position.unwrap_or_default())
                    .unwrap_or_else(std::time::Instant::now));
                self.paused_position = None;
                
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
            self.current_file = None;
            self.start_time = None;
            self.paused_position = None;
            Ok(())
        }

        pub fn seek(&mut self, seconds: u64) -> Result<(), PlayerError> {
            if let Some(decoder) = &mut self.decoder.lock().unwrap().as_mut() {
                // First seek in the decoder
                decoder.seek(seconds)?;
                
                // Create a new sink with the current decoder
                let new_sink = Sink::try_new(&self.stream_handle)
                    .map_err(|e| PlayerError::AudioError(format!("Failed to create audio sink: {}", e)))?;
                
                // Stop and replace the old sink
                if let Some(old_sink) = self.sink.take() {
                    old_sink.stop();
                }
                
                new_sink.play();
                self.sink = Some(new_sink);
                
                Ok(())
            } else {
                Err(PlayerError::InvalidState("No active playback".into()))
            }
        }

        pub fn state(&self) -> PlayerState {
            self.state
        }

        pub fn status(&self) -> PlayerStatus {
            let position = match (self.state, self.start_time, self.paused_position) {
                (PlayerState::Playing, Some(start_time), _) => Some(start_time.elapsed()),
                (PlayerState::Paused, _, Some(pos)) => Some(pos),
                _ => None,
            };

            let duration = self.decoder.lock().unwrap()
                .as_ref()
                .and_then(|decoder| decoder.duration);

            PlayerStatus {
                state: self.state,
                position,
                duration,
                current_file: self.current_file.clone(),
                volume: self.volume,
            }
        }

        pub fn set_volume(&mut self, volume: f32) -> Result<(), PlayerError> {
            // Validate volume is between 0.0 and 1.0
            if !(0.0..=1.0).contains(&volume) {
                return Err(PlayerError::InvalidVolume(volume));
            }

            self.volume = volume;
            if let Some(sink) = &self.sink {
                sink.set_volume(volume);
            }
            Ok(())
        }

        pub fn get_volume(&self) -> f32 {
            self.volume
        }

        pub fn increase_volume(&mut self) -> Result<(), PlayerError> {
            let new_volume = (self.volume + 0.1).min(1.0);
            self.set_volume(new_volume)
        }

        pub fn decrease_volume(&mut self) -> Result<(), PlayerError> {
            let new_volume = (self.volume - 0.1).max(0.0);
            self.set_volume(new_volume)
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
            // Forward the path parameter to inner implementation
            self.inner.lock().unwrap().play(path)
        }
        #[cfg(not(feature = "audio"))]
        {
            // Verify path exists but still return AudioDisabled
            if !path.exists() {
                return Err(PlayerError::FileNotFound(path.display().to_string()));
            }
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
        // Validate seek parameter
        if seconds > 24 * 60 * 60 {  // More than 24 hours
            return Err(PlayerError::InvalidState(format!("Invalid seek position: {}s", seconds)));
        }
        
        #[cfg(not(feature = "audio"))]
        {
            Err(PlayerError::AudioDisabled)
        }
        #[cfg(feature = "audio")]
        {
            self.inner.lock().unwrap().seek(seconds)
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

    pub fn status(&self) -> PlayerStatus {
        #[cfg(feature = "audio")]
        {
            self.inner.lock().unwrap().status()
        }
        #[cfg(not(feature = "audio"))]
        {
            PlayerStatus {
                state: self.state,
                position: None,
                duration: None,
                current_file: None,
            }
        }
    }
}
