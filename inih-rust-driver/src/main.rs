//! Differential parity driver for inih-core.
use inih_core::{
    ini_parse_file_path, ini_parse_string, ini_parse_string_with_alloc, HeapHooks, IniConfig,
    IniHandler,
};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

struct Dumper {
    user_slot: i32,
    last_user: i32,
    prev_section: String,
    handler_lineno: bool,
}

impl Dumper {
    fn new(handler_lineno: bool) -> Self {
        Self {
            user_slot: 100,
            last_user: 0,
            prev_section: String::new(),
            handler_lineno,
        }
    }
}

impl IniHandler for Dumper {
    fn handle(
        &mut self,
        section: &str,
        name: Option<&str>,
        value: Option<&str>,
        lineno: i32,
    ) -> bool {
        self.last_user = self.user_slot;
        if name.is_none() || section != self.prev_section {
            println!("... [{section}]");
            self.prev_section = section.to_string();
        }
        let Some(name) = name else {
            return true;
        };
        if self.handler_lineno {
            print!("... {name}");
            if let Some(v) = value {
                print!("={v}");
            }
            println!(";  line {lineno}");
        } else {
            print!("... {name}");
            if let Some(v) = value {
                print!("={v}");
            }
            println!(";");
        }
        match value {
            None => true,
            Some(v) => !(name == "user" && v == "parse_error"),
        }
    }
}

struct PrintAlloc;
impl HeapHooks for PrintAlloc {
    fn malloc(&mut self, size: usize) {
        println!("ini_malloc({size})");
    }
    fn free(&mut self) {
        println!("ini_free()");
    }
    fn realloc(&mut self, size: usize) {
        println!("ini_realloc({size})");
    }
}

fn string_display_name(stem: &str) -> &str {
    match stem {
        "empty_string" => "empty string",
        "long_line" => "long line",
        "long_continued" => "long continued",
        other => other,
    }
}

fn infer_profile(fixture: &Path) -> Option<String> {
    let parts: Vec<_> = fixture
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    for i in 0..parts.len().saturating_sub(1) {
        if parts[i] == "parity" {
            return Some(parts[i + 1].clone());
        }
    }
    None
}

fn is_string_profile(profile: &str) -> bool {
    matches!(profile, "string" | "heap_string" | "alloc")
}

fn main() {
    let mut args = env::args().skip(1);
    let mut fixture: Option<PathBuf> = None;
    let mut profile_override: Option<String> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--sections" => {
                let _ = args.next();
            }
            "--profile" => {
                profile_override = args.next();
            }
            s if s.starts_with('-') => {
                eprintln!("unknown flag: {s}");
                process::exit(2);
            }
            s => fixture = Some(PathBuf::from(s)),
        }
    }
    let Some(fixture) = fixture else {
        eprintln!("usage: inih-rust-driver [--sections parse] [--profile NAME] <fixture>");
        process::exit(2);
    };

    let profile = profile_override
        .or_else(|| infer_profile(&fixture))
        .unwrap_or_else(|| {
            eprintln!("cannot infer profile from {}", fixture.display());
            process::exit(2);
        });

    let cfg = match IniConfig::for_profile(&profile) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            process::exit(2);
        }
    };

    let base = fixture
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("fixture");

    // Ensure stdout is not line-buffered oddly; match C.
    let _ = io::stdout().flush();

    if is_string_profile(&profile) {
        let content = match fs::read(&fixture) {
            Ok(b) => String::from_utf8_lossy(&b).into_owned(),
            Err(e) => {
                eprintln!("failed to read {}: {e}", fixture.display());
                process::exit(1);
            }
        };
        let stem = base.strip_suffix(".ini").unwrap_or(base);
        let mut dumper = Dumper::new(cfg.handler_lineno);
        let e = if profile == "alloc" {
            let mut alloc = PrintAlloc;
            ini_parse_string_with_alloc(&cfg, &content, &mut dumper, &mut alloc)
        } else {
            ini_parse_string(&cfg, &content, &mut dumper)
        };
        if profile == "alloc" {
            println!("{}: e={e}", string_display_name(stem));
        } else {
            println!(
                "{}: e={e} user={}",
                string_display_name(stem),
                dumper.last_user
            );
        }
        return;
    }

    let mut dumper = Dumper::new(cfg.handler_lineno);
    let e = if base == "no_file.ini" {
        ini_parse_file_path(&cfg, Path::new("__inih_no_such_file__.ini"), &mut dumper)
    } else {
        ini_parse_file_path(&cfg, &fixture, &mut dumper)
    };
    println!("{base}: e={e} user={}", dumper.last_user);
}
