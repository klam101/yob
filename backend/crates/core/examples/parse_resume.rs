//! Prints what the resume parser sees and makes of a file: the extracted
//! text, the links found in it, and the parsed draft profile.
//!
//! `./scripts/dev.sh parse-resume path/to/resume.pdf` (or `cargo run -p
//! yob-core --example parse_resume -- <file>` from `backend/`). Add `--raw`
//! to also print the text before cleanup.

use yob_core::profile::{clean, parse, resume};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let raw = args
        .iter()
        .position(|a| a == "--raw")
        .map(|i| args.remove(i))
        .is_some();
    let Some(path) = args.first() else {
        eprintln!("usage: parse_resume <file.pdf|file.docx|file.txt> [--raw]");
        std::process::exit(2);
    };
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("couldn't read {path}: {e}");
        std::process::exit(1);
    });
    let extracted = match resume::detect(&bytes) {
        Some(kind) => resume::extract(kind, &bytes).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        }),
        None => resume::Extracted {
            text: String::from_utf8_lossy(&bytes).into_owned(),
            links: Vec::new(),
        },
    };
    if raw {
        println!("==== raw text\n{}", extracted.text);
    }
    println!("==== clean text\n{}", clean::clean(&extracted.text));
    println!("==== links");
    for l in &extracted.links {
        println!("  {l}");
    }
    let parsed = parse::parse(&extracted.text, &extracted.links);
    println!(
        "==== profile\n{}",
        serde_json::to_string_pretty(&parsed.profile).unwrap()
    );
    println!("==== not placed");
    for u in &parsed.unsorted {
        println!("  {u}");
    }
}
