// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2020, Olof Kraigher olof.kraigher@gmail.com

use super::contents::Contents;
use super::diagnostic::{Diagnostic, DiagnosticResult};
use parking_lot::{RwLock, RwLockReadGuard};
use std::cmp::{max, min};
use std::collections::hash_map::DefaultHasher;
use std::convert::AsRef;
use std::fmt;
use std::fmt::Write;
use std::hash::{Hash, Hasher};
use std::io;
pub use std::path::{Path, PathBuf};
use std::sync::Arc;

struct FileId {
    name: PathBuf,
    /// Hash value of `self.name`.
    hash: u64,
}

impl FileId {
    fn new(name: &Path) -> FileId {
        let hash = hash(name);
        Self {
            name: name.to_owned(),
            hash,
        }
    }
}

impl PartialEq for FileId {
    fn eq(&self, other: &Self) -> bool {
        // Use file name hash to speedup comparison
        if self.hash == other.hash {
            self.name == other.name
        } else {
            false
        }
    }
}

fn hash(value: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Represents a single source file and its contents.
struct UniqueSource {
    file_id: FileId,
    contents: RwLock<Contents>,
}

impl fmt::Debug for UniqueSource {
    /// Custom implementation to avoid large contents strings.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Source {{file_name: {:?}}}", self.file_name())
    }
}

impl UniqueSource {
    fn inline(file_name: &Path, contents: &str) -> Self {
        Self {
            file_id: FileId::new(file_name),
            contents: RwLock::new(Contents::from_str(contents)),
        }
    }

    fn from_latin1_file(file_name: &Path) -> io::Result<Self> {
        let contents = Contents::from_latin1_file(file_name)?;
        Ok(Self {
            file_id: FileId::new(file_name),
            contents: RwLock::new(contents),
        })
    }

    #[cfg(test)]
    pub fn from_contents(file_name: &Path, contents: Contents) -> UniqueSource {
        let file_name = file_name.into();
        Self {
            file_id: FileId::new(file_name),
            contents: RwLock::new(contents),
        }
    }

    fn contents(&self) -> RwLockReadGuard<Contents> {
        self.contents.read()
    }

    fn file_name(&self) -> &Path {
        self.file_id.name.as_ref()
    }
}

/// A thread-safe reference to a source file.
/// Multiple objects of this type can refer to the same source.
#[derive(Debug, Clone)]
pub struct Source {
    source: Arc<UniqueSource>,
}

impl PartialEq for Source {
    fn eq(&self, other: &Self) -> bool {
        self.source.file_id == other.source.file_id
    }
}

impl Eq for Source {}

impl Hash for Source {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        hasher.write_u64(self.source.file_id.hash)
    }
}

impl Source {
    /// Creates a source from a (virtual) name and in-memory contents.
    ///
    /// Note: For differing values of `contents`, the value of `file_name`
    /// *must* differ as well.
    pub fn inline(file_name: &Path, contents: &str) -> Source {
        Source {
            source: Arc::new(UniqueSource::inline(file_name, contents)),
        }
    }

    pub fn from_latin1_file(file_name: &Path) -> io::Result<Source> {
        Ok(Source {
            source: Arc::new(UniqueSource::from_latin1_file(file_name)?),
        })
    }

    #[cfg(test)]
    pub fn from_contents(file_name: &Path, contents: Contents) -> Source {
        Source {
            source: Arc::new(UniqueSource::from_contents(file_name, contents)),
        }
    }

    pub fn contents(&self) -> RwLockReadGuard<Contents> {
        self.source.contents()
    }

    pub fn file_name(&self) -> &Path {
        self.source.file_name()
    }

    pub fn pos(&self, start: Position, end: Position) -> SrcPos {
        SrcPos {
            source: self.clone(),
            range: Range { start, end },
        }
    }

    pub fn change(&self, range: Option<&Range>, content: &str) {
        let mut contents = self.source.contents.write();
        if let Some(range) = range {
            contents.change(range, content);
        } else {
            *contents = Contents::from_str(content);
        }
    }
}

/// A lexical position (line, column) in a source.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
pub struct Position {
    /// Line (zero-based).
    pub line: u32,
    /// Column (zero-based).
    pub character: u32,
}

impl Position {
    pub fn default() -> Position {
        Position {
            line: 0,
            character: 0,
        }
    }

    pub fn new(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    pub fn next_char(self) -> Position {
        Position {
            line: self.line,
            character: self.character + 1,
        }
    }

    pub fn move_after_char(&mut self, chr: char) {
        if chr == '\n' {
            self.line += 1;
            self.character = 0;
        } else {
            self.character += chr.len_utf16() as u32;
        }
    }

    pub fn after_char(mut self, chr: char) -> Position {
        self.move_after_char(chr);
        self
    }

    pub fn prev_char(self) -> Position {
        Position {
            line: self.line,
            character: self.character.saturating_sub(1),
        }
    }

    pub fn range_to(self, end: Position) -> Range {
        Range { start: self, end }
    }
}

/// A lexical range in a source.
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub struct Range {
    /// Start of the range (inclusive).
    pub start: Position,
    /// End of the range (exclusive).
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Range {
        Range { start, end }
    }
}

/// A lexical range within a specific source file.
#[derive(PartialEq, Clone, Debug, Eq, Hash)]
pub struct SrcPos {
    /// The referenced source file.
    pub source: Source,
    range: Range,
}

/// A generic object with an associated source file and lexical range.
#[derive(PartialEq, Clone, Debug)]
pub struct WithPos<T> {
    pub item: T,
    pub pos: SrcPos,
}

impl<T> WithPos<T> {
    pub fn new(item: T, pos: impl AsRef<SrcPos>) -> WithPos<T> {
        WithPos {
            item,
            pos: pos.as_ref().clone(),
        }
    }

    pub fn from(item: T, pos: impl Into<SrcPos>) -> WithPos<T> {
        WithPos {
            item,
            pos: pos.into(),
        }
    }

    pub fn map_into<F, U>(self, f: F) -> WithPos<U>
    where
        F: FnOnce(T) -> U,
    {
        WithPos {
            item: f(self.item),
            pos: self.pos,
        }
    }

    pub fn try_map_into<F, U>(self, f: F) -> DiagnosticResult<WithPos<U>>
    where
        F: FnOnce(T) -> Result<U, String>,
    {
        match f(self.item) {
            Ok(item) => Ok(WithPos {
                item,
                pos: self.pos,
            }),
            Err(msg) => Err(Diagnostic::error(&self.pos, msg)),
        }
    }

    pub fn combine_pos_with(self, other: &dyn AsRef<SrcPos>) -> Self {
        WithPos {
            item: self.item,
            pos: self.pos.combine_into(other.as_ref()),
        }
    }
}

impl<T> AsRef<SrcPos> for WithPos<T> {
    fn as_ref(&self) -> &SrcPos {
        &self.pos
    }
}

impl AsRef<SrcPos> for SrcPos {
    fn as_ref(&self) -> &SrcPos {
        self
    }
}

impl<T> Into<SrcPos> for WithPos<T> {
    fn into(self) -> SrcPos {
        self.pos
    }
}

impl SrcPos {
    const LINE_CONTEXT: u32 = 2;

    pub fn new(source: Source, range: Range) -> SrcPos {
        SrcPos { source, range }
    }

    fn get_line_context(&self, context_lines: u32, contents: &Contents) -> Vec<(u32, String)> {
        let mut lines = Vec::new();

        let start = self.range.start.line.saturating_sub(context_lines);
        let end = self.range.end.line + context_lines;

        for lineno in start..=end {
            if let Some(line) = contents.get_line(lineno as usize) {
                lines.push((lineno, line.to_owned()));
            }
        }

        if lines.is_empty() {
            lines.push((self.range.start.line, String::new()));
        }
        lines
    }

    fn push_replicate(line: &mut String, chr: char, times: usize) {
        for _ in 0..times {
            line.push(chr);
        }
    }

    fn visual_width(chr: char) -> usize {
        if chr == '\t' {
            4
        } else {
            1
        }
    }

    /// Write ~~~ to underline symbol
    fn underline(&self, lineno_len: usize, lineno: u32, line: &str, into: &mut String) {
        const NEWLINE_SIZE: usize = 1;
        into.reserve("  |  ".len() + lineno_len + line.len() + NEWLINE_SIZE);

        // Prefix
        for _ in 0..lineno_len {
            into.push(' ');
        }
        into.push_str("  |  ");

        let mut pos = Position {
            line: lineno,
            character: 0,
        };
        // Padding before underline
        for chr in line.chars() {
            if pos < self.range.start {
                Self::push_replicate(into, ' ', Self::visual_width(chr));
            } else if pos < self.range.end {
                Self::push_replicate(into, '~', Self::visual_width(chr));
            } else {
                break;
            }
            pos.character += chr.len_utf16() as u32;
        }

        if lineno == self.range.end.line {
            while pos < self.range.end {
                into.push('~');
                pos.character += 1;
            }
        }

        // Newline
        into.push_str("\n");
    }

    fn code_context_from_contents(
        &self,
        contents: &Contents,
        context_lines: u32,
    ) -> (usize, String) {
        let lines = self.get_line_context(context_lines, contents);
        use pad::{Alignment, PadStr};
        // +1 since lines are shown with 1-index
        let lineno_len = (self.range.start.line + context_lines + 1)
            .to_string()
            .len();

        let mut result = String::new();

        for (lineno, line) in lines.iter() {
            let line = line.to_string();
            let line = line.trim_matches('\n');
            let lineno_str = (lineno + 1)
                .to_string()
                .pad_to_width_with_alignment(lineno_len, Alignment::Right);
            let overlaps = self.range.start.line <= *lineno && *lineno <= self.range.end.line;

            if overlaps {
                write!(result, "{} --> ", lineno_str).unwrap();
            } else {
                write!(result, "{}  |  ", lineno_str).unwrap();
            }

            for chr in line.trim_end().chars() {
                if chr == '\t' {
                    Self::push_replicate(&mut result, ' ', Self::visual_width(chr));
                } else {
                    result.push(chr);
                }
            }
            result.push('\n');

            if overlaps {
                self.underline(lineno_len, *lineno, line, &mut result);
            }
        }

        (lineno_len, result)
    }

    /// Create a string for pretty printing.
    pub fn code_context(&self) -> String {
        self.lineno_len_and_code_context().1
    }

    fn lineno_len_and_code_context(&self) -> (usize, String) {
        let contents = self.source.contents();
        self.code_context_from_contents(&contents, Self::LINE_CONTEXT)
    }

    pub fn show(&self, message: &str) -> String {
        let (lineno_len, pretty_str) = self.lineno_len_and_code_context();
        let file_name = self.source.file_name();
        let mut result = String::new();

        let lineno = self.range.start.line;
        writeln!(result, "{}", &message).unwrap();
        for _ in 0..lineno_len {
            result.push(' ');
        }
        writeln!(
            result,
            " --> {}:{}",
            file_name.to_string_lossy(),
            lineno + 1
        )
        .unwrap();
        for _ in 0..lineno_len {
            result.push(' ');
        }
        writeln!(result, "  |").unwrap();
        result.push_str(&pretty_str);
        result
    }

    /// Combines two lexical positions into a larger lexical position overlapping both.
    /// The file name is assumed to be the same.
    pub fn combine_into(self, other: &dyn AsRef<Self>) -> Self {
        let other = other.as_ref();
        debug_assert!(self.source == other.source, "Assumes sources are equal");

        let start = min(self.range.start, other.range.start);
        let end = max(self.range.end, other.range.end);

        SrcPos {
            source: self.source,
            range: Range { start, end },
        }
    }

    pub fn start(&self) -> Position {
        self.range.start
    }

    pub fn end(&self) -> Position {
        self.range.end
    }

    pub fn range(&self) -> Range {
        self.range
    }

    pub fn file_name(&self) -> &Path {
        self.source.file_name()
    }

    pub fn combine(&self, other: &dyn AsRef<Self>) -> Self {
        self.clone().combine_into(other)
    }
}

/// Denotes an item with an associated source file.
///
/// Most types that implement this trait do so through the blanket implementation
/// on [`HasSrcPos`](trait.HasSrcPos.html).
pub trait HasSource {
    fn source(&self) -> &Source;
}

impl HasSource for Source {
    fn source(&self) -> &Source {
        &self
    }
}

/// Denotes an item with an associated lexical range in a source file.
pub trait HasSrcPos {
    fn pos(&self) -> &SrcPos;
}

impl HasSrcPos for SrcPos {
    fn pos(&self) -> &SrcPos {
        &self
    }
}

impl<T: HasSrcPos> HasSource for T {
    fn source(&self) -> &Source {
        &self.pos().source
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Latin1String;
    use crate::syntax::test::{Code, CodeBuilder};
    use pretty_assertions::assert_eq;
    use tempfile;

    #[test]
    fn srcpos_combine() {
        let code = Code::new("hello world");

        assert_eq!(
            code.s1("hello").pos().combine(&code.s1("world").pos()),
            code.pos()
        );

        assert_eq!(code.s1("h").pos().combine(&code.s1("d").pos()), code.pos());

        assert_eq!(code.s1("d").pos().combine(&code.s1("h").pos()), code.pos());
    }

    fn with_code_from_file<F, R>(contents: &str, fun: F) -> R
    where
        F: Fn(Code) -> R,
    {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        let file_name = file.path().to_owned();
        file.write(&Latin1String::from_utf8_unchecked(contents).bytes)
            .unwrap();
        fun(CodeBuilder::new().code_from_source(Source::from_latin1_file(&file_name).unwrap()))
    }

    #[test]
    fn code_context_pos_from_filename() {
        with_code_from_file("hello\nworld\n", |code: Code| {
            assert_eq!(
                code.s1("hello").pos().code_context(),
                "\
1 --> hello
   |  ~~~~~
2  |  world
"
            )
        });
    }

    #[test]
    fn code_context_pos_last_line_without_newline() {
        let code = Code::new("hello world");
        let pos = code.s1("hello").pos();
        assert_eq!(
            pos.code_context(),
            "\
1 --> hello world
   |  ~~~~~
"
        );
    }

    #[test]
    fn code_context_pos_with_indent() {
        let code = Code::new("    hello world");
        let pos = code.s1("hello").pos();
        assert_eq!(
            pos.code_context(),
            "\
1 -->     hello world
   |      ~~~~~
"
        );
    }

    #[test]
    fn code_context_eof() {
        let code = Code::new("h");
        assert_eq!(
            code.eof_pos().code_context(),
            "\
1 --> h
   |   ~
",
        );
    }

    #[test]
    fn code_context_eof_empty() {
        let code = Code::new("");
        assert_eq!(code.eof_pos().code_context(), "1 --> \n   |  ~\n",);
    }

    #[test]
    fn code_context_with_context() {
        let code = Code::new("hello\nworld");
        let pos = code.s1("hello").pos();
        assert_eq!(
            pos.code_context(),
            "\
1 --> hello
   |  ~~~~~
2  |  world
",
        );
    }

    #[test]
    fn code_context_with_tabs() {
        let code = Code::new("\thello\t");
        let pos = code.s1("hello\t").pos();
        assert_eq!(
            pos.code_context(),
            "\
1 -->     hello
   |      ~~~~~~~~~
",
        );
    }

    #[test]
    fn code_context_non_ascii() {
        let code = Code::new("åäö\nåäö\n__å_ä_ö__");
        let substr = code.s1("å_ä_ö");
        assert_eq!(substr.end().character - substr.start().character, 5);
        assert_eq!(
            substr.pos().code_context(),
            "\
1  |  åäö
2  |  åäö
3 --> __å_ä_ö__
   |    ~~~~~
",
        );
    }

    #[test]
    fn code_context_double_utf16() {
        // Bomb emojii requires 2 utf-16 codes
        let code = Code::new("\u{1F4A3}");
        assert_eq!(code.end().character - code.start().character, 2);
        assert_eq!(
            code.pos().code_context(),
            "\
1 --> \u{1F4A3}
   |  ~
",
        );
    }

    #[test]
    fn code_context_non_ascii_from_file() {
        with_code_from_file("åäö\nåäö\n__å_ä_ö__", |code: Code| {
            let substr = code.s1("å_ä_ö");
            assert_eq!(substr.end().character - substr.start().character, 5);
            assert_eq!(
                substr.pos().code_context(),
                "\
1  |  åäö
2  |  åäö
3 --> __å_ä_ö__
   |    ~~~~~
",
            );
        });
    }

    #[test]
    fn code_context_with_full_context() {
        let code = Code::new(
            "\
line1
line2
line3
line4
line5
line6
line7
line8
line9
line10
line11
line12
line13",
        );
        let pos = code.s1("line10").pos();
        assert_eq!(
            pos.code_context(),
            " \
 8  |  line8
 9  |  line9
10 --> line10
    |  ~~~~~~
11  |  line11
12  |  line12
",
        );
    }

    #[test]
    fn show_from_filename() {
        with_code_from_file("hello\nworld\nline\n", |code: Code| {
            assert_eq!(
                code.s1("world").pos().show("Greetings"),
                format!(
                    "\
Greetings
  --> {}:2
   |
1  |  hello
2 --> world
   |  ~~~~~
3  |  line
",
                    code.source().file_name().to_string_lossy()
                )
            )
        });
    }

    #[test]
    fn show_contents() {
        let code = Code::new("hello\nworld\nline\n");
        assert_eq!(
            code.s1("world").pos().show("Greetings"),
            format!(
                "\
Greetings
  --> {}:2
   |
1  |  hello
2 --> world
   |  ~~~~~
3  |  line
",
                code.source().file_name().to_string_lossy()
            )
        );
    }
}
