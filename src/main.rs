mod lint;

use lint::{lint_playlist, Finding};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut had_findings = false;
    let mut had_io_error = false;

    if args.is_empty() {
        match read_stdin() {
            Ok(contents) => {
                let findings = lint_playlist(&contents);
                had_findings |= print_findings("<stdin>", &findings);
            }
            Err(e) => {
                eprintln!("plint: error reading stdin: {e}");
                had_io_error = true;
            }
        }
    } else {
        for path in &args {
            let contents = if path == "-" {
                read_stdin()
            } else {
                fs::read_to_string(path)
            };

            match contents {
                Ok(contents) => {
                    let findings = lint_playlist(&contents);
                    had_findings |= print_findings(path, &findings);
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
    } else if had_findings {
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

fn print_findings(source_name: &str, findings: &[Finding]) -> bool {
    for finding in findings {
        println!(
            "{source_name}:{}: {}: {}",
            finding.line,
            finding.severity.as_str(),
            finding.message
        );
    }
    !findings.is_empty()
}
