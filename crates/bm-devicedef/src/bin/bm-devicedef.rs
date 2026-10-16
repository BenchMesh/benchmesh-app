//! `bm-devicedef validate [DIR]` — validate all definitions (exit 1 on errors)
//! `bm-devicedef schema device|class` — print the JSON Schema

use std::path::PathBuf;
use std::process::ExitCode;

use bm_devicedef::{class_json_schema, device_json_schema, load_catalog, validate_catalog, Severity};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("validate") => {
            let dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("devices"));
            let cat = match load_catalog(&dir) {
                Ok(c) => c,
                Err(errs) => {
                    for e in errs {
                        eprintln!("error: {e}");
                    }
                    return ExitCode::FAILURE;
                }
            };
            let issues = validate_catalog(&cat);
            let errors = issues.iter().filter(|(_, i)| i.severity == Severity::Error).count();
            for (file, i) in &issues {
                println!("{file}: {i}");
            }
            println!(
                "{} classes, {} definitions: {} errors, {} warnings",
                cat.classes.len(),
                cat.devices.len(),
                errors,
                issues.len() - errors
            );
            if errors > 0 {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Some("schema") => {
            let schema = match args.get(1).map(String::as_str) {
                Some("device") => device_json_schema(),
                Some("class") => class_json_schema(),
                _ => {
                    eprintln!("usage: bm-devicedef schema device|class");
                    return ExitCode::FAILURE;
                }
            };
            println!("{}", serde_json::to_string_pretty(&schema).expect("json"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("usage: bm-devicedef validate [DIR] | schema device|class");
            ExitCode::FAILURE
        }
    }
}
