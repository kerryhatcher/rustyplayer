# rustyplayer — chosen crates & rationale

This document records the initial crate choices for the MVP and brief reasons.

- clap (v4) — CLI parsing with a good derive macro and subcommand support; small surface.
- symphonia (v0.6) — versatile audio decoding library supporting MP3/FLAC/WAV and many codecs; used for metadata extraction and decoding.
- rodio (v0.17) — high-level audio playback; integrates with cpal under the hood and is simple for MVP playback.
- rusqlite (v0.29) — lightweight SQLite wrapper; stable and suitable for local library storage.
- walkdir (v2) — recursive directory walking for the library scanner.
- directories (v4) — find platform-appropriate config/data directories for the DB file.
- anyhow + thiserror — ergonomic error handling and conversions for the app.

Notes
- These choices prioritize developer ergonomics and minimal native setup on Linux. If we find that `rodio` cannot meet advanced needs (e.g., low-latency seeking), we'll evaluate `cpal` directly or gstreamer bindings.
- Version numbers are initial estimates; lockfile (Cargo.lock) and CI will pin exact versions.
