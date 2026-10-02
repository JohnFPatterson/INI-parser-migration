//! Safe Rust port of inih parser logic.
#![deny(unsafe_code)]

use std::fs::File;
use std::io::Read;
use std::path::Path;

pub mod internals;

/// Options mirroring `INI_*` compile-time macros.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IniConfig {
    pub allow_multiline: bool,
    pub allow_bom: bool,
    pub start_comment_prefixes: String,
    pub allow_inline_comments: bool,
    pub inline_comment_prefixes: String,
    pub use_stack: bool,
    pub max_line: usize,
    pub max_section: usize,
    pub max_name: usize,
    pub allow_realloc: bool,
    pub initial_alloc: usize,
    pub stop_on_first_error: bool,
    pub call_handler_on_new_section: bool,
    pub allow_no_value: bool,
    pub handler_lineno: bool,
}

impl Default for IniConfig {
    fn default() -> Self {
        Self {
            allow_multiline: true,
            allow_bom: true,
            start_comment_prefixes: ";#".to_string(),
            allow_inline_comments: true,
            inline_comment_prefixes: ";".to_string(),
            use_stack: true,
            max_line: 200,
            max_section: 50,
            max_name: 50,
            allow_realloc: false,
            initial_alloc: 200,
            stop_on_first_error: false,
            call_handler_on_new_section: false,
            allow_no_value: false,
            handler_lineno: false,
        }
    }
}

impl IniConfig {
    pub fn for_profile(profile: &str) -> Result<Self, Error> {
        Ok(match profile {
            "multi" => Self::default(),
            "multi_max_line" => Self {
                max_line: 20,
                ..Default::default()
            },
            "single" => Self {
                allow_multiline: false,
                ..Default::default()
            },
            "disallow_inline_comments" => Self {
                allow_inline_comments: false,
                ..Default::default()
            },
            "stop_on_first_error" => Self {
                stop_on_first_error: true,
                ..Default::default()
            },
            "handler_lineno" => Self {
                handler_lineno: true,
                ..Default::default()
            },
            "heap" => Self {
                use_stack: false,
                ..Default::default()
            },
            "heap_max_line" => Self {
                use_stack: false,
                max_line: 20,
                initial_alloc: 20,
                ..Default::default()
            },
            "heap_realloc" => Self {
                use_stack: false,
                allow_realloc: true,
                initial_alloc: 5,
                ..Default::default()
            },
            "heap_realloc_max_line" => Self {
                use_stack: false,
                max_line: 20,
                allow_realloc: true,
                initial_alloc: 5,
                ..Default::default()
            },
            "call_handler_on_new_section" => Self {
                call_handler_on_new_section: true,
                ..Default::default()
            },
            "allow_no_value" => Self {
                allow_no_value: true,
                ..Default::default()
            },
            "string" => Self {
                max_line: 20,
                ..Default::default()
            },
            "heap_string" => Self {
                use_stack: false,
                max_line: 20,
                initial_alloc: 20,
                ..Default::default()
            },
            "alloc" => Self {
                use_stack: false,
                allow_realloc: true,
                initial_alloc: 12,
                ..Default::default()
            },
            other => return Err(Error::UnknownProfile(other.to_string())),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    UnknownProfile(String),
    Io(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnknownProfile(p) => write!(f, "unknown profile: {p}"),
            Error::Io(m) => write!(f, "io error: {m}"),
        }
    }
}

impl std::error::Error for Error {}

/// Optional hooks matching C `ini_malloc` / `ini_free` / `ini_realloc` observability.
pub trait HeapHooks {
    fn malloc(&mut self, size: usize);
    fn free(&mut self);
    fn realloc(&mut self, size: usize);
}

pub struct NoHeap;
impl HeapHooks for NoHeap {
    fn malloc(&mut self, _: usize) {}
    fn free(&mut self) {}
    fn realloc(&mut self, _: usize) {}
}

/// Return `true` on success (C nonzero), `false` on error (C zero).
pub trait IniHandler {
    fn handle(
        &mut self,
        section: &str,
        name: Option<&str>,
        value: Option<&str>,
        lineno: i32,
    ) -> bool;
}

pub type ParseResult = i32;

pub(crate) fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

fn rstrip_len(buf: &mut [u8]) -> usize {
    let mut end = buf.len();
    while end > 0 && is_space(buf[end - 1]) {
        end -= 1;
        buf[end] = 0;
    }
    end
}

fn lskip(buf: &[u8]) -> usize {
    let mut i = 0;
    while i < buf.len() && is_space(buf[i]) {
        i += 1;
    }
    i
}

pub(crate) fn find_chars_or_comment(cfg: &IniConfig, s: &[u8], chars: Option<&[u8]>) -> usize {
    let mut i = 0;
    if cfg.allow_inline_comments {
        let mut was_space = false;
        while i < s.len() {
            let c = s[i];
            if let Some(ch) = chars {
                if ch.contains(&c) {
                    break;
                }
            }
            if was_space && cfg.inline_comment_prefixes.as_bytes().contains(&c) {
                break;
            }
            was_space = is_space(c);
            i += 1;
        }
    } else {
        while i < s.len() {
            let c = s[i];
            if let Some(ch) = chars {
                if ch.contains(&c) {
                    break;
                }
            }
            i += 1;
        }
    }
    i
}

fn strncpy0(dest: &mut [u8], src: &[u8]) {
    if dest.is_empty() {
        return;
    }
    let copy = src.len().min(dest.len() - 1);
    dest[..copy].copy_from_slice(&src[..copy]);
    dest[copy] = 0;
}

fn bytes_to_str(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn cstr(buf: &[u8]) -> String {
    bytes_to_str(&buf[..strlen(buf)])
}

struct StringReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl StringReader<'_> {
    fn read_line(&mut self, buf: &mut [u8], num: usize) -> bool {
        if self.pos >= self.data.len() || num < 2 {
            return false;
        }
        let mut n = 0usize;
        let max = num - 1;
        while n < max && self.pos < self.data.len() {
            let c = self.data[self.pos];
            self.pos += 1;
            buf[n] = c;
            n += 1;
            if c == b'\n' {
                break;
            }
        }
        buf[n] = 0;
        true
    }
}

struct FileLineReader {
    file: File,
    pending: Vec<u8>,
    eof: bool,
}

impl FileLineReader {
    fn open(path: &Path) -> Result<Self, Error> {
        Ok(Self {
            file: File::open(path).map_err(|e| Error::Io(e.to_string()))?,
            pending: Vec::new(),
            eof: false,
        })
    }

    fn read_line(&mut self, buf: &mut [u8], num: usize) -> bool {
        if num < 2 || self.eof {
            return false;
        }
        let max = num - 1;
        let mut n = 0usize;
        while n < max {
            if self.pending.is_empty() {
                let mut chunk = [0u8; 1024];
                match self.file.read(&mut chunk) {
                    Ok(0) => {
                        self.eof = true;
                        break;
                    }
                    Ok(k) => self.pending.extend_from_slice(&chunk[..k]),
                    Err(_) => {
                        self.eof = true;
                        break;
                    }
                }
            }
            if self.pending.is_empty() {
                break;
            }
            let c = self.pending.remove(0);
            buf[n] = c;
            n += 1;
            if c == b'\n' {
                break;
            }
        }
        if n == 0 {
            return false;
        }
        buf[n] = 0;
        true
    }
}

enum Reader<'a> {
    String(StringReader<'a>),
    File(FileLineReader),
}

impl Reader<'_> {
    fn read_line(&mut self, buf: &mut [u8], num: usize) -> bool {
        match self {
            Reader::String(r) => r.read_line(buf, num),
            Reader::File(r) => r.read_line(buf, num),
        }
    }
}

fn parse_stream<H: IniHandler, A: HeapHooks>(
    cfg: &IniConfig,
    reader: &mut Reader<'_>,
    handler: &mut H,
    alloc: Option<&mut A>,
) -> ParseResult {
    let mut max_line = if cfg.use_stack {
        cfg.max_line
    } else {
        cfg.initial_alloc
    };
    let mut line = vec![0u8; max_line.max(1)];
    let mut alloc = alloc;
    if !cfg.use_stack {
        if let Some(a) = alloc.as_mut() {
            a.malloc(cfg.initial_alloc);
        }
    }

    let mut section = vec![0u8; cfg.max_section.max(1)];
    section[0] = 0;
    let mut prev_name = vec![0u8; cfg.max_name.max(1)];
    prev_name[0] = 0;
    let mut lineno = 0i32;
    let mut error = 0i32;
    let mut abyss = [0u8; 16];

    loop {
        if line.len() < max_line {
            line.resize(max_line, 0);
        }
        if !reader.read_line(&mut line, max_line) {
            break;
        }
        let mut offset = strlen(&line);

        if cfg.allow_realloc && !cfg.use_stack {
            while max_line < cfg.max_line
                && offset == max_line - 1
                && offset > 0
                && line[offset - 1] != b'\n'
            {
                max_line *= 2;
                if max_line > cfg.max_line {
                    max_line = cfg.max_line;
                }
                line.resize(max_line, 0);
                if let Some(a) = alloc.as_mut() {
                    a.realloc(max_line);
                }
                let mut more = vec![0u8; (max_line - offset).max(1)];
                if !reader.read_line(&mut more, max_line - offset) {
                    break;
                }
                let add = strlen(&more);
                line[offset..offset + add].copy_from_slice(&more[..add]);
                line[offset + add] = 0;
                offset += add;
            }
        }

        lineno += 1;

        if offset == max_line - 1 && offset > 0 && line[offset - 1] != b'\n' {
            let abyss_cap = abyss.len();
            while reader.read_line(&mut abyss, abyss_cap) {
                if error == 0 {
                    error = lineno;
                }
                let abyss_len = strlen(&abyss);
                if abyss_len > 0 && abyss[abyss_len - 1] == b'\n' {
                    break;
                }
            }
        }

        let mut bom_off = 0usize;
        if cfg.allow_bom
            && lineno == 1
            && offset >= 3
            && line[0] == 0xEF
            && line[1] == 0xBB
            && line[2] == 0xBF
        {
            bom_off = 3;
        }

        let after_lskip = bom_off + lskip(&line[bom_off..offset]);
        let mut kept = line[after_lskip..offset].to_vec();
        let n = rstrip_len(&mut kept);
        kept.truncate(n);
        // C: start > line  (pointer after lskip past &line[0])
        let had_leading_ws = after_lskip > 0;

        if kept.is_empty() {
            // blank
        } else if cfg.start_comment_prefixes.as_bytes().contains(&kept[0]) {
            // comment
        } else if cfg.allow_multiline && prev_name[0] != 0 && !kept.is_empty() && had_leading_ws {
            let mut cont = kept;
            if cfg.allow_inline_comments {
                let end = find_chars_or_comment(cfg, &cont, None);
                cont.truncate(end);
                let n = rstrip_len(&mut cont);
                cont.truncate(n);
            }
            let prev = cstr(&prev_name);
            let section_s = cstr(&section);
            let val = bytes_to_str(&cont);
            if !handler.handle(&section_s, Some(&prev), Some(&val), lineno) && error == 0 {
                error = lineno;
            }
        } else if kept[0] == b'[' {
            let end = find_chars_or_comment(cfg, &kept[1..], Some(b"]"));
            if 1 + end < kept.len() && kept[1 + end] == b']' {
                strncpy0(&mut section, &kept[1..1 + end]);
                if cfg.allow_multiline {
                    prev_name[0] = 0;
                }
                if cfg.call_handler_on_new_section {
                    let section_s = cstr(&section);
                    if !handler.handle(&section_s, None, None, lineno) && error == 0 {
                        error = lineno;
                    }
                }
            } else if error == 0 {
                error = lineno;
            }
        } else {
            let buf = kept;
            let end = find_chars_or_comment(cfg, &buf, Some(b"=:"));
            if end < buf.len() && (buf[end] == b'=' || buf[end] == b':') {
                let mut name = buf[..end].to_vec();
                let nlen = rstrip_len(&mut name);
                name.truncate(nlen);

                let mut value = buf[end + 1..].to_vec();
                if cfg.allow_inline_comments {
                    let ie = find_chars_or_comment(cfg, &value, None);
                    value.truncate(ie);
                }
                let vskip = lskip(&value);
                let mut value = value[vskip..].to_vec();
                let vlen = rstrip_len(&mut value);
                value.truncate(vlen);

                if cfg.allow_multiline {
                    strncpy0(&mut prev_name, &name);
                }
                let section_s = cstr(&section);
                let name_s = bytes_to_str(&name);
                let value_s = bytes_to_str(&value);
                if !handler.handle(&section_s, Some(&name_s), Some(&value_s), lineno) && error == 0
                {
                    error = lineno;
                }
            } else if cfg.allow_no_value {
                let mut name = buf[..end.min(buf.len())].to_vec();
                let nlen = rstrip_len(&mut name);
                name.truncate(nlen);
                let section_s = cstr(&section);
                let name_s = bytes_to_str(&name);
                if !handler.handle(&section_s, Some(&name_s), None, lineno) && error == 0 {
                    error = lineno;
                }
            } else if error == 0 {
                error = lineno;
            }
        }

        if cfg.stop_on_first_error && error != 0 {
            break;
        }
    }

    if !cfg.use_stack {
        if let Some(a) = alloc.as_mut() {
            a.free();
        }
    }
    error
}

pub fn ini_parse_string_with_alloc<H: IniHandler, A: HeapHooks>(
    cfg: &IniConfig,
    data: &str,
    handler: &mut H,
    alloc: &mut A,
) -> ParseResult {
    let mut reader = Reader::String(StringReader {
        data: data.as_bytes(),
        pos: 0,
    });
    parse_stream(cfg, &mut reader, handler, Some(alloc))
}

pub fn ini_parse_string<H: IniHandler>(
    cfg: &IniConfig,
    data: &str,
    handler: &mut H,
) -> ParseResult {
    let mut reader = Reader::String(StringReader {
        data: data.as_bytes(),
        pos: 0,
    });
    let mut no = NoHeap;
    let hooks = if cfg.use_stack { None } else { Some(&mut no) };
    parse_stream(cfg, &mut reader, handler, hooks)
}

pub fn ini_parse_string_length<H: IniHandler>(
    cfg: &IniConfig,
    data: &[u8],
    handler: &mut H,
) -> ParseResult {
    let mut reader = Reader::String(StringReader { data, pos: 0 });
    let mut no = NoHeap;
    let hooks = if cfg.use_stack { None } else { Some(&mut no) };
    parse_stream(cfg, &mut reader, handler, hooks)
}

pub fn ini_parse_file_path<H: IniHandler>(
    cfg: &IniConfig,
    path: &Path,
    handler: &mut H,
) -> ParseResult {
    let file = match FileLineReader::open(path) {
        Ok(f) => f,
        Err(_) => return -1,
    };
    let mut reader = Reader::File(file);
    let mut no = NoHeap;
    let hooks = if cfg.use_stack { None } else { Some(&mut no) };
    parse_stream(cfg, &mut reader, handler, hooks)
}
