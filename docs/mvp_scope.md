# rustyplayer — MVP scope & contract

This document defines the Minimum Viable Product (MVP) for `rustyplayer`.

## Goals (what MVP must achieve)
- Play local audio files (play/pause/stop/seek).
- Maintain a small persistent library using SQLite with play tracking and simple user ratings.
- Provide a straightforward CLI for common flows (play single file, add to playlist, scan a folder).
- Support common audio formats on Linux (at minimum: MP3, FLAC, WAV). Cross-platform design is desired but out-of-scope for first pass.

## Contract (inputs / outputs / errors)
- Inputs:
  - Local filesystem paths to media files or directories.
  - CLI commands and flags.
- Outputs:
  - Audio sent to the system's default audio output device.
  - Console output for status, errors, and simple progress messages.
  - SQLite database file (e.g. `~/.local/share/rustyplayer/library.db`).
- Errors / error modes:
  - Unsupported format -> clear message and non-zero exit for CLI command.
  - Missing file/path -> clear message.
  - Audio device unavailable -> error message with suggestion to check device.
  - DB locked/unwritable -> message and fallback (read-only or abort depending on command).

## Minimum feature list (MVP scope)
1. Play audio file from CLI: `rustyplayer play <path>` — supports play/pause/stop.
2. Playlist: add/list/remove tracks in-memory and persisted to DB.
3. Library scanner: recursively scan a directory, extract basic metadata (title, artist, album, duration) and insert into DB.
4. Play tracking: increment play_count and record last_played timestamp each time a file finishes or is explicitly played.
5. Simple CLI built with `clap` exposing commands: `play`, `pause`, `stop`, `playlist`, `scan`, `library`.

## Non-goals (out of MVP scope)
- Video playback/rendering.
- Advanced UI (no full TUI/GUI in MVP; CLI only).
- Network streaming, podcasts, or remote libraries.
- Complex audio effect processing.

## Data shapes (DB tables, minimum)
- `tracks`:
  - id INTEGER PRIMARY KEY
  - path TEXT UNIQUE NOT NULL
  - title TEXT
  - artist TEXT
  - album TEXT
  - duration_seconds INTEGER
  - added_at TIMESTAMP

- `play_events` or fields on `tracks`:
  - play_count INTEGER DEFAULT 0
  - last_played TIMESTAMP NULLABLE

## API surface (internal)
- play(path: &Path) -> Result<()>
- pause() -> Result<()>
- stop() -> Result<()>
- seek(seconds: u64) -> Result<()>
- scan(dir: &Path) -> Result<ImportSummary>
- db_insert_track(metadata) -> Result<TrackId>

Implementations may wrap these in a `Player` struct that owns the playback thread and communicates via channels.

## Acceptance criteria (how we know MVP is done)
- Running `cargo run -- play /path/to/song.mp3` produces audible playback and returns 0 on success.
- `scan` imports files into the DB and shows counts of new/imported files.
- Playing a track increments `play_count` and updates `last_played` in the DB.
- CLI commands return meaningful, human-readable errors for common failure modes.

## Assumptions
- Primary development and verification is done on Linux. Audio device setup, card names, and permissions are platform-dependent.
- We'll use well-maintained crates (Symphonia for decoding, Rodio or cpal for output, Rusqlite for DB) unless a blocker appears.

## Edge cases to watch for
- Very large libraries (scanning performance and DB insertion batching).
- Unsupported container/codec combos — ensure graceful failure.
- Concurrent DB access if multiple commands run simultaneously.

## Estimated effort (rough)
- Scope & design doc: 0.5 day (this doc)
- Crate selection & scaffolding: 0.5 day
- Playback core + single-file CLI play: 2–3 days
- Playlist, DB, scanner: 2–3 days
- Tests, CI, docs: 1–2 days

## Next steps
1. Select crates and record choices in `docs/choices.md` (Task 2).
2. Scaffold project modules and update `Cargo.toml` (Task 3).
3. Implement playback core (Task 4).

---
Document created by the project planning step. Update as implementation details solidify.
