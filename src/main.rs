mod lint;

use lint::{lint_playlist, Finding, Severity};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut strict = false;
    let mut paths: Vec<String> = Vec::new();

    for arg in env::args().skip(1) {
        if arg == "--strict" {
            strict = true;
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
                had_error |= print_findings("<stdin>", &findings, strict);
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
                    had_error |= print_findings(path, &findings, strict);
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
fn print_findings(source_name: &str, findings: &[Finding], strict: bool) -> bool {
    let mut had_error = false;
    for finding in findings {
        println!(
            "{source_name}:{}: {}: {}",
            finding.line,
            finding.severity.as_str(),
            finding.message
        );
        had_error |= finding.severity == Severity::Error || strict;
    }
    had_error
}
