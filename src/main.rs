mod lint;

use lint::{lint_playlist, Finding, Severity};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut strict = false;
    let mut json = false;
    let mut paths: Vec<String> = Vec::new();

    for arg in env::args().skip(1) {
        if arg == "--strict" {
            strict = true;
        } else if arg == "--json" {
            json = true;
        } else {
            paths.push(arg);
        }
    }

    let mut had_error = false;
    let mut had_io_error = false;

    if paths.is_empty() {
        match read_stdin() {
            Ok(contents) => {
                let findings = lint_playlist(&contents, None);
                had_error |= print_findings("<stdin>", &findings, strict, json);
            }
            Err(e) => {
                eprintln!("plint: error reading stdin: {e}");
                had_io_error = true;
            }
        }
    } else {
        for path in &paths {
            let contents = if path == "-" {
                read_stdin()
            } else {
                fs::read_to_string(path)
            };

            match contents {
                Ok(contents) => {
                    let base_dir = if path == "-" {
                        None
                    } else {
                        Path::new(path).parent()
                    };
                    let findings = lint_playlist(&contents, base_dir);
                    had_error |= print_findings(path, &findings, strict, json);
                }
                Err(e) => {
                    eprintln!("plint: error reading {path}: {e}");
                    had_io_error = true;
                }
            }
        }
    }

    if had_io_error {
        ExitCode::from(2)
    } else if had_error {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn read_stdin() -> io::Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

/// Prints every finding and reports whether the run should be treated as
/// a failure: always true if any finding is an error, or if `strict` is
/// set and there's a warning.
///
/// In JSON mode each finding is printed as its own line of JSON (not a
/// wrapping array), so an editor can parse diagnostics as they stream in
/// across multiple files instead of waiting for the whole run to finish.
fn print_findings(source_name: &str, findings: &[Finding], strict: bool, json: bool) -> bool {
    let mut had_error = false;
    for finding in findings {
        if json {
            println!(
                "{{\"file\":\"{}\",\"line\":{},\"severity\":\"{}\",\"message\":\"{}\"}}",
                json_escape(source_name),
                finding.line,
                finding.severity.as_str(),
                json_escape(&finding.message)
            );
        } else {
            println!(
                "{source_name}:{}: {}: {}",
                finding.line,
                finding.severity.as_str(),
                finding.message
            );
        }
        had_error |= finding.severity == Severity::Error || strict;
    }
    had_error
}

/// Escapes a string for embedding in a JSON string literal. Handles the
/// characters `serde_json` would insist on and nothing more, since this is
/// the one place in the codebase that produces JSON and pulling in a
/// dependency for it isn't worth it.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escape_passes_plain_text_through() {
        assert_eq!(json_escape("song.mp3"), "song.mp3");
    }

    #[test]
    fn json_escape_handles_quotes_and_backslashes() {
        assert_eq!(
            json_escape(r#"track "live" \ raw.mp3"#),
            r#"track \"live\" \\ raw.mp3"#
        );
    }

    #[test]
    fn json_escape_handles_control_characters() {
        assert_eq!(json_escape("a\nb\tc\r"), "a\\nb\\tc\\r");
        assert_eq!(json_escape("\u{1}"), "\\u0001");
    }
}
