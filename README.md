# playlist-lint

A small linter for M3U / M3U8 audio playlists. It reads a playlist and
reports structural problems with a file and line number, the way a
compiler warning would, instead of leaving you to notice at playback
time that track 40 of 200 silently got skipped.

## Why

M3U is a plain text format with almost no validation anywhere in the
tooling that produces or consumes it. Playlists exported from one app
and edited by hand or merged with another one tend to accumulate the
same handful of problems:

- an `#EXTINF:` line with no track path after it (the previous entry
  and the new metadata get merged into whatever plays next, or the
  metadata is just dropped)
- an `#EXTINF:` duration that isn't a number, because someone hand
  edited the file
- the same track listed twice
- a playlist missing the `#EXTM3U` header, which some players will
  refuse to treat as extended M3U at all, silently dropping titles and
  durations
- a local track reference that points at a file that isn't there,
  usually because the playlist was moved without the media, or a track
  was renamed or deleted after the playlist was built

None of these cause an error until playback, and the symptom (wrong
title showing, a track missing, playback stalling) rarely points back
to the actual line in the file.

## Usage

Lint a file:

```
$ plint road_trip.m3u
road_trip.m3u:14: error: #EXTINF entry has no following track
road_trip.m3u:22: warning: duplicate track entry 'songs/interlude.mp3'
```

Lint multiple files in one run:

```
$ plint library/*.m3u
```

Read from stdin, e.g. as part of a pipeline that generates a playlist
on the fly:

```
$ generate-playlist.sh | plint
```

`-` also means stdin, so it can sit in a list of files:

```
$ cat extra.m3u | plint main.m3u -
```

Local track paths (anything that isn't a URL) are checked for
existence relative to the playlist file's own directory. Reading a
playlist from stdin skips that check for relative paths, since there's
no directory to resolve them against; absolute paths are still
checked either way.

Exit codes: `0` means no findings (or only warnings, without `--strict`),
`1` means an error-level finding was reported, `2` means a file or
stdin could not be read.

Pass `--strict` to also exit `1` on warnings, which is useful in CI
where you want any finding to fail the build:

```
$ plint --strict road_trip.m3u
```

## Building

Standard library only, no dependencies:

```
$ cargo build --release
$ ./target/release/plint some.m3u
```

## Status

Early. The checks so far are the ones listed above. See the issues /
roadmap for what's planned next; contributions and bug reports on real
playlists that trip it up (or that it misses) are useful.
