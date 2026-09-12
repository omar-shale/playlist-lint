#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub line: usize,
    pub severity: Severity,
    pub message: String,
}

/// Checks an M3U/M3U8 playlist for structural problems and returns
/// findings ordered by line number. `source` is the whole file, not a
/// single line, since some checks (dangling #EXTINF, duplicate tracks)
/// need to look across lines.
pub fn lint_playlist(source: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut seen_tracks: Vec<&str> = Vec::new();
    let mut pending_extinf: Option<usize> = None;
    let mut is_first_content_line = true;

    for (idx, raw_line) in source.lines().enumerate() {
        let line_no = idx + 1;
        // Tolerate CRLF line endings even though `lines()` already
        // splits on '\n'; a trailing '\r' would otherwise end up
        // baked into paths and comparisons.
        let trimmed = raw_line.trim_end_matches('\r').trim();

        if trimmed.is_empty() {
            continue;
        }

        if is_first_content_line {
            is_first_content_line = false;
            if trimmed == "#EXTM3U" {
                continue;
            }
            findings.push(Finding {
                line: line_no,
                severity: Severity::Warning,
                message: "playlist does not start with #EXTM3U".to_string(),
            });
            // Don't `continue` here: this line still needs to be
            // evaluated as a directive or track below.
        }

        if let Some(body) = trimmed.strip_prefix("#EXTINF:") {
            if let Some(pending_line) = pending_extinf {
                findings.push(Finding {
                    line: pending_line,
                    severity: Severity::Error,
                    message: "#EXTINF entry has no following track".to_string(),
                });
            }

            match body.split_once(',') {
                Some((duration, title)) => {
                    if duration.parse::<f64>().is_err() {
                        findings.push(Finding {
                            line: line_no,
                            severity: Severity::Error,
                            message: format!("#EXTINF duration '{duration}' is not a number"),
                        });
                    }
                    if title.trim().is_empty() {
                        findings.push(Finding {
                            line: line_no,
                            severity: Severity::Warning,
                            message: "#EXTINF entry has an empty title".to_string(),
                        });
                    }
                }
                None => {
                    findings.push(Finding {
                        line: line_no,
                        severity: Severity::Warning,
                        message: "#EXTINF entry is missing a title after the duration"
                            .to_string(),
                    });
                }
            }

            pending_extinf = Some(line_no);
            continue;
        }

        if trimmed.starts_with('#') {
            // Other directive (#EXTGRP, #PLAYLIST, ...) or a plain
            // comment. Neither closes a pending #EXTINF.
            continue;
        }

        // Anything else is a track reference: a file path or a URL.
        pending_extinf = None;

        if seen_tracks.contains(&trimmed) {
            findings.push(Finding {
                line: line_no,
                severity: Severity::Warning,
                message: format!("duplicate track entry '{trimmed}'"),
            });
        } else {
            seen_tracks.push(trimmed);
        }
    }

    if let Some(pending_line) = pending_extinf {
        findings.push(Finding {
            line: pending_line,
            severity: Severity::Error,
            message: "#EXTINF entry has no following track".to_string(),
        });
    }

    findings.sort_by_key(|f| f.line);
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_playlist_has_no_findings() {
        let src = "#EXTM3U\n#EXTINF:123,Artist - Title\nsong.mp3\n";
        assert!(lint_playlist(src).is_empty());
    }

    #[test]
    fn missing_header_is_a_warning() {
        let src = "song.mp3\n";
        let findings = lint_playlist(src);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(findings[0].line, 1);
    }

    #[test]
    fn extinf_without_track_is_an_error() {
        let src = "#EXTM3U\n#EXTINF:123,Artist - Title\n#EXTINF:45,Other Track\nsong.mp3\n";
        let findings = lint_playlist(src);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
        assert_eq!(findings[0].line, 2);
    }

    #[test]
    fn non_numeric_duration_is_an_error() {
        let src = "#EXTM3U\n#EXTINF:abc,Artist - Title\nsong.mp3\n";
        let findings = lint_playlist(src);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
        assert!(findings[0].message.contains("not a number"));
    }

    #[test]
    fn duplicate_track_is_a_warning() {
        let src = "#EXTM3U\nsong.mp3\nsong.mp3\n";
        let findings = lint_playlist(src);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 3);
    }

    #[test]
    fn live_stream_duration_of_negative_one_is_valid() {
        let src = "#EXTM3U\n#EXTINF:-1,Live Radio\nhttp://example.invalid/stream\n";
        assert!(lint_playlist(src).is_empty());
    }
}
