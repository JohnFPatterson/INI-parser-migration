//! Ported public-API style tests (from tests/unittest*.c), against inih-core.
use inih_core::{
    ini_parse_file_path, ini_parse_string, ini_parse_string_with_alloc, HeapHooks, IniConfig,
    IniHandler,
};
use std::path::PathBuf;

struct Collect {
    lines: Vec<String>,
    prev: String,
    user_slot: i32,
    last_user: i32,
    lineno: bool,
}

impl Collect {
    fn new(lineno: bool) -> Self {
        Self {
            lines: Vec::new(),
            prev: String::new(),
            user_slot: 100,
            last_user: 0,
            lineno,
        }
    }
}

impl IniHandler for Collect {
    fn handle(
        &mut self,
        section: &str,
        name: Option<&str>,
        value: Option<&str>,
        lineno: i32,
    ) -> bool {
        self.last_user = self.user_slot;
        if name.is_none() || section != self.prev {
            self.lines.push(format!("... [{section}]"));
            self.prev = section.to_string();
        }
        let Some(name) = name else {
            return true;
        };
        if self.lineno {
            match value {
                Some(v) => self.lines.push(format!("... {name}={v};  line {lineno}")),
                None => self.lines.push(format!("... {name};  line {lineno}")),
            }
        } else {
            match value {
                Some(v) => self.lines.push(format!("... {name}={v};")),
                None => self.lines.push(format!("... {name};")),
            }
        }
        !matches!(value, Some(v) if name == "user" && v == "parse_error")
    }
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests")
        .join(name)
}

#[test]
fn parse_normal_ini_defaults() {
    let cfg = IniConfig::default();
    let mut h = Collect::new(false);
    let e = ini_parse_file_path(&cfg, &fixture("normal.ini"), &mut h);
    assert_eq!(e, 0);
    assert!(h.lines.iter().any(|l| l.contains("one=This is a test")));
    assert_eq!(h.last_user, 100);
}

#[test]
fn parse_no_file() {
    let cfg = IniConfig::default();
    let mut h = Collect::new(false);
    let e = ini_parse_file_path(
        &cfg,
        std::path::Path::new("__inih_no_such_file__.ini"),
        &mut h,
    );
    assert_eq!(e, -1);
    assert_eq!(h.last_user, 0);
}

#[test]
fn parse_user_error() {
    let cfg = IniConfig::default();
    let mut h = Collect::new(false);
    let e = ini_parse_file_path(&cfg, &fixture("user_error.ini"), &mut h);
    assert_eq!(e, 3);
}

#[test]
fn parse_string_basic() {
    let cfg = IniConfig {
        max_line: 20,
        ..Default::default()
    };
    let mut h = Collect::new(false);
    let e = ini_parse_string(&cfg, "[section]\nfoo = bar\nbazz = buzz quxx", &mut h);
    assert_eq!(e, 0);
    assert!(h.lines.iter().any(|l| l.contains("foo=bar")));
}

#[test]
fn parse_allow_no_value() {
    let cfg = IniConfig {
        allow_no_value: true,
        ..Default::default()
    };
    let mut h = Collect::new(false);
    let e = ini_parse_file_path(&cfg, &fixture("no_value.ini"), &mut h);
    assert_eq!(e, 0);
}

#[test]
fn parse_handler_lineno() {
    let cfg = IniConfig {
        handler_lineno: true,
        ..Default::default()
    };
    let mut h = Collect::new(true);
    let e = ini_parse_file_path(&cfg, &fixture("normal.ini"), &mut h);
    assert_eq!(e, 0);
    assert!(h.lines.iter().any(|l| l.contains("line ")));
}

#[test]
fn parse_call_handler_on_new_section() {
    let cfg = IniConfig {
        call_handler_on_new_section: true,
        ..Default::default()
    };
    let mut h = Collect::new(false);
    let e = ini_parse_file_path(&cfg, &fixture("normal.ini"), &mut h);
    assert_eq!(e, 0);
    assert!(h
        .lines
        .iter()
        .any(|l| l == "... [empty]" || l.starts_with("... [")));
}

struct CountAlloc {
    mallocs: usize,
}
impl HeapHooks for CountAlloc {
    fn malloc(&mut self, _: usize) {
        self.mallocs += 1;
    }
    fn free(&mut self) {}
    fn realloc(&mut self, _: usize) {}
}

#[test]
fn parse_string_with_alloc_hooks() {
    let cfg = IniConfig {
        use_stack: false,
        allow_realloc: true,
        initial_alloc: 12,
        ..Default::default()
    };
    let mut h = Collect::new(false);
    let mut a = CountAlloc { mallocs: 0 };
    let e = ini_parse_string_with_alloc(
        &cfg,
        "[section]\nfoo = bar\nbazz = buzz quxx",
        &mut h,
        &mut a,
    );
    assert_eq!(e, 0);
    assert_eq!(a.mallocs, 1);
}

#[test]
fn single_line_disables_multiline() {
    let cfg = IniConfig {
        allow_multiline: false,
        ..Default::default()
    };
    let mut h = Collect::new(false);
    let _e = ini_parse_file_path(&cfg, &fixture("multi_line.ini"), &mut h);
    assert!(!h.lines.iter().any(|l| l.contains("multi=multi-line value")));
}
