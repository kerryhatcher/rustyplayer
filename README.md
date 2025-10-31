# rustyplayer
yet another media player

## Features

- Play audio and video files
- Support for various media formats (MP3, FLAC, WAV)
- Simple and intuitive command-line interface
- Play tracking and history
- User ratings of media files
- Playlist management
- Random and repeat playback modes

## Quick Start

### Build from source

1. Install required system packages:

For Debian/Ubuntu:
```bash
# Minimal build (no audio playback):
sudo apt install -y build-essential pkg-config

# With audio support:
sudo apt install -y build-essential pkg-config libasound2-dev

# Optional: PulseAudio/PipeWire development files
sudo apt install -y libpulse-dev libpipewire-0.3-dev
```

For Fedora:
```bash
# Minimal build:
sudo dnf install -y gcc pkg-config

# With audio support:
sudo dnf install -y gcc pkg-config alsa-lib-devel

# Optional: PulseAudio/PipeWire development files
sudo dnf install -y pulseaudio-libs-devel pipewire-devel
```

For Arch Linux:
```bash
# Minimal build:
sudo pacman -S --needed base-devel pkgconf

# With audio support:
sudo pacman -S --needed base-devel pkgconf alsa-lib

# Optional: PulseAudio/PipeWire development files
sudo pacman -S --needed libpulse pipewire
```

2. Build the project:

```bash
# Build without audio support (CI, development):
cargo build

# Build with audio support (requires system audio dev packages):
cargo build --features audio
```

3. Run basic commands:

```bash
# Play a single file:
cargo run -- play path/to/music.mp3

# Scan a directory into library:
cargo run -- scan path/to/music/folder
```

## More info

Uses SQLite to store media metadata, play tracking, user ratings, and settings.
Built with Rust for performance and safety.

## Development Notes

- Audio playback requires system development packages (see build instructions).
- Use `--features audio` to enable playback support.
- The `audio` feature is optional to allow building/testing without system audio dependencies.