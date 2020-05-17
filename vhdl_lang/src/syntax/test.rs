// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2020, Olof Kraigher olof.kraigher@gmail.com

use super::common::ParseResult;
use super::concurrent_statement::parse_labeled_concurrent_statement;
use super::context::{parse_library_clause, parse_use_clause};
use super::declarative_part::parse_declarative_part_leave_end_token;
use super::design_unit::parse_design_file;
use super::expression::{parse_aggregate, parse_choices, parse_expression};
use super::interface_declaration::{parse_generic, parse_parameter, parse_port};
use super::names::{parse_association_list, parse_designator, parse_name, parse_selected_name};
use super::range::{parse_discrete_range, parse_range};
use super::sequential_statement::parse_sequential_statement;
use super::subprogram::{parse_signature, parse_subprogram_declaration_no_semi};
use super::subtype_indication::parse_subtype_indication;
use super::tokens::{Comment, KeyWordToken, Kind, Symbols, Token, TokenStream, Tokenizer};
use super::waveform::parse_waveform;
use crate::ast;
use crate::ast::*;
use crate::data::Range;
use crate::data::*;
use std::collections::hash_map::DefaultHasher;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hasher;
use std::sync::Arc;

pub struct CodeBuilder {
    pub symbols: Arc<Symbols>,
}

impl CodeBuilder {
    pub fn new() -> CodeBuilder {
        CodeBuilder {
            symbols: Arc::new(Symbols::default()),
        }
    }

    pub fn code_from_source(&self, source: Source) -> Code {
        let contents = source.contents();

        let pos = SrcPos::new(source.clone(), contents.range());

        let code = Code {
            symbols: self.symbols.clone(),
            pos,
        };

        // Ensure symbol table is populated
        code.tokenize_result();
        code
    }

    pub fn code_with_file_name(&self, file_name: &Path, code: &str) -> Code {
        self.code_from_source(Source::inline(file_name, code))
    }

    pub fn code(&self, code: &str) -> Code {
        let mut hasher = DefaultHasher::new();
        hasher.write(code.as_bytes());
        let file_name: PathBuf = format!("<unknown file with hash {}>", hasher.finish()).into();
        self.code_with_file_name(&file_name, code)
    }

    pub fn symbol(&self, name: &str) -> Symbol {
        self.symbols.symtab().insert_utf8(name)
    }
}

#[derive(Clone)]
pub struct Code {
    pub symbols: Arc<Symbols>,
    pos: SrcPos,
}

impl Code {
    pub fn new(code: &str) -> Code {
        CodeBuilder::new().code(code)
    }

    pub fn new_with_file_name(file_name: &Path, code: &str) -> Code {
        CodeBuilder::new().code_with_file_name(file_name, code)
    }

    fn in_range(&self, range: Range) -> Code {
        Code {
            symbols: self.symbols.clone(),
            pos: SrcPos::new(self.pos.source.clone(), range),
        }
    }

    /// Create new Code from n:th occurence of substr
    pub fn s(&self, substr: &str, occurence: usize) -> Code {
        self.in_range(substr_range(
            &self.pos.source,
            self.pos.range(),
            substr,
            occurence,
        ))
    }

    /// Create new Code from first n:th occurence of substr
    pub fn s1(&self, substr: &str) -> Code {
        self.s(substr, 1)
    }

    pub fn pos(&self) -> SrcPos {
        self.pos.clone()
    }

    // Position after code
    pub fn eof_pos(&self) -> SrcPos {
        SrcPos::new(
            self.source().clone(),
            Range::new(self.end(), self.end().next_char()),
        )
    }

    pub fn start(&self) -> Position {
        self.pos.start()
    }

    pub fn end(&self) -> Position {
        self.pos.end()
    }

    pub fn source(&self) -> &Source {
        &self.pos.source
    }

    /// Helper method to test tokenization functions
    pub fn tokenize_result(&self) -> (Vec<Result<Token, Diagnostic>>, Vec<Comment>) {
        let mut tokens = Vec::new();
        let final_comments: Vec<Comment>;
        {
            let contents = self.pos.source.contents();
            let reader = ContentReader::new(&contents);
            let mut tokenizer = Tokenizer::new(&self.symbols, &self.pos.source, reader);
            loop {
                let token = tokenizer.pop();

                match token {
                    Ok(None) => break,
                    Ok(Some(token)) => tokens.push(Ok(token)),
                    Err(err) => tokens.push(Err(err)),
                }
            }
            match tokenizer.get_final_comments() {
                Some(comments) => final_comments = comments,
                None => panic!("Tokenizer failed to check for final comments."),
            }
        }
        (tokens, final_comments)
    }

    /// Tokenize and check that there are no errors, ignore final comments
    pub fn tokenize(&self) -> Vec<Token> {
        let tokens = self.tokenize_result().0;
        tokens.into_iter().map(|tok| tok.unwrap()).collect()
    }

    /// Helper method to run lower level parsing function at specific substring
    pub fn parse<F, R>(&self, parse_fun: F) -> R
    where
        F: FnOnce(&mut TokenStream) -> R,
    {
        let contents = self.pos.source.contents();
        let source = Source::from_contents(
            self.pos.file_name(),
            contents.crop(Range::new(Position::default(), self.pos.end())),
        );
        let contents = source.contents();
        let reader = ContentReader::new(&contents);
        let tokenizer = Tokenizer::new(&self.symbols, &source, reader);
        let mut stream = TokenStream::new(tokenizer);
        forward(&mut stream, self.pos.start());
        parse_fun(&mut stream)
    }

    /// Expect Ok() value
    pub fn parse_ok<F, R>(&self, parse_fun: F) -> R
    where
        F: FnOnce(&mut TokenStream) -> ParseResult<R>,
    {
        match self.parse(parse_fun) {
            Ok(res) => res,
            Err(diagnostic) => {
                panic!("{}", diagnostic.show());
            }
        }
    }

    pub fn with_partial_stream<F, R>(&self, parse_fun: F) -> R
    where
        F: FnOnce(&mut TokenStream) -> R,
    {
        let contents = self.pos.source.contents();
        let reader = ContentReader::new(&contents);
        let tokenizer = Tokenizer::new(&self.symbols, &self.pos.source, reader);
        let mut stream = TokenStream::new(tokenizer);
        parse_fun(&mut stream)
    }

    pub fn with_stream<F, R>(&self, parse_fun: F) -> R
    where
        R: Debug,
        F: FnOnce(&mut TokenStream) -> ParseResult<R>,
    {
        let parse_fun_eof = |stream: &mut TokenStream| {
            let result = parse_fun(stream);
            match result {
                Err(err) => {
                    println!("{:#?}", err);
                    println!("{}", err.show());
                    panic!("Got Err()");
                }
                Ok(result) => {
                    if let Some(token) = stream.peek().unwrap() {
                        println!("result = {:#?}", result);
                        panic!("Expected EOF got {:?}", token);
                    }
                    result
                }
            }
        };

        self.with_partial_stream(parse_fun_eof)
    }

    pub fn with_stream_err<F, R>(&self, parse_fun: F) -> Diagnostic
    where
        R: Debug,
        F: FnOnce(&mut TokenStream) -> ParseResult<R>,
    {
        let parse_fun_eof = |stream: &mut TokenStream| {
            let result = parse_fun(stream);
            match result {
                Err(err) => {
                    if let Some(token) = stream.peek().unwrap() {
                        println!("err = {:#?}", err);
                        panic!("Expected EOF got {:?}", token);
                    }
                    err
                }
                Ok(result) => {
                    panic!("Expected error got {:?}", result);
                }
            }
        };

        self.with_partial_stream(parse_fun_eof)
    }

    pub fn with_partial_stream_diagnostics<F, R>(&self, parse_fun: F) -> (R, Vec<Diagnostic>)
    where
        R: Debug,
        F: FnOnce(&mut TokenStream, &mut dyn DiagnosticHandler) -> R,
    {
        let mut diagnostics = Vec::new();
        let result = self
            .with_partial_stream(|stream: &mut TokenStream| parse_fun(stream, &mut diagnostics));
        (result, diagnostics)
    }

    pub fn with_stream_diagnostics<F, R>(&self, parse_fun: F) -> (R, Vec<Diagnostic>)
    where
        R: Debug,
        F: FnOnce(&mut TokenStream, &mut dyn DiagnosticHandler) -> ParseResult<R>,
    {
        let mut diagnostics = Vec::new();
        let result =
            self.with_stream(|stream: &mut TokenStream| parse_fun(stream, &mut diagnostics));
        (result, diagnostics)
    }

    pub fn with_stream_no_diagnostics<F, R>(&self, parse_fun: F) -> R
    where
        R: Debug,
        F: FnOnce(&mut TokenStream, &mut dyn DiagnosticHandler) -> ParseResult<R>,
    {
        let (result, diagnostics) = self.with_stream_diagnostics(parse_fun);
        check_no_diagnostics(&diagnostics);
        result
    }

    pub fn declarative_part(&self) -> Vec<Declaration> {
        let mut diagnostics = Vec::new();
        let res = self
            .parse_ok(|stream| parse_declarative_part_leave_end_token(stream, &mut diagnostics));
        check_no_diagnostics(&diagnostics);
        res
    }
    /// Helper to create a identifier at first occurence of name
    pub fn ident(&self) -> Ident {
        self.parse_ok(|stream: &mut TokenStream| stream.expect_ident())
    }

    pub fn designator(&self) -> WithPos<Designator> {
        self.parse_ok(parse_designator)
    }

    pub fn designator_ref(&self) -> WithPos<WithRef<Designator>> {
        self.parse_ok(parse_designator).into_ref()
    }

    pub fn character(&self) -> WithPos<u8> {
        self.parse_ok(|stream: &mut TokenStream| stream.expect()?.expect_character())
    }

    /// Helper method to create expression from first occurence of substr
    /// Can be used to test all but expression parsing
    pub fn expr(&self) -> WithPos<Expression> {
        self.parse_ok(parse_expression)
    }

    pub fn name(&self) -> WithPos<Name> {
        self.parse_ok(parse_name)
    }

    pub fn selected_name(&self) -> WithPos<SelectedName> {
        self.parse_ok(parse_selected_name)
    }

    pub fn signature(&self) -> WithPos<Signature> {
        self.parse_ok(parse_signature)
    }

    /// Return symbol from symbol table
    pub fn symbol(&self, name: &str) -> Symbol {
        self.symbols.symtab().insert_utf8(name)
    }

    pub fn subtype_indication(&self) -> SubtypeIndication {
        self.parse_ok(parse_subtype_indication)
    }

    pub fn port(&self) -> InterfaceDeclaration {
        self.parse_ok(parse_port)
    }

    pub fn generic(&self) -> InterfaceDeclaration {
        self.parse_ok(parse_generic)
    }

    pub fn parameter(&self) -> InterfaceDeclaration {
        self.parse_ok(parse_parameter)
    }

    pub fn function_call(&self) -> FunctionCall {
        let name = self.name();
        match name.item {
            Name::FunctionCall(call) => *call,
            _ => FunctionCall {
                name,
                parameters: vec![],
            },
        }
    }

    pub fn parse_ok_no_diagnostics<F, R>(&self, parse_fun: F) -> R
    where
        F: FnOnce(&mut TokenStream, &mut dyn DiagnosticHandler) -> ParseResult<R>,
    {
        let mut diagnostics = Vec::new();
        let res = self.parse_ok(|stream| parse_fun(stream, &mut diagnostics));
        check_no_diagnostics(&diagnostics);
        res
    }

    pub fn sequential_statement(&self) -> LabeledSequentialStatement {
        self.parse_ok_no_diagnostics(parse_sequential_statement)
    }

    pub fn concurrent_statement(&self) -> LabeledConcurrentStatement {
        self.parse_ok_no_diagnostics(parse_labeled_concurrent_statement)
    }

    pub fn association_list(&self) -> Vec<AssociationElement> {
        self.parse_ok(parse_association_list)
    }

    pub fn waveform(&self) -> Waveform {
        self.parse_ok(parse_waveform)
    }

    pub fn aggregate(&self) -> WithPos<Vec<ElementAssociation>> {
        self.parse_ok(|stream| parse_aggregate(stream))
    }

    pub fn range(&self) -> ast::Range {
        self.parse_ok(parse_range).item
    }

    pub fn discrete_range(&self) -> DiscreteRange {
        self.parse_ok(parse_discrete_range)
    }

    pub fn choices(&self) -> Vec<Choice> {
        self.parse_ok(parse_choices)
    }

    pub fn use_clause(&self) -> WithPos<UseClause> {
        self.parse_ok(parse_use_clause)
    }

    pub fn library_clause(&self) -> WithPos<LibraryClause> {
        self.parse_ok(parse_library_clause)
    }

    pub fn design_file(&self) -> DesignFile {
        self.parse_ok_no_diagnostics(parse_design_file)
    }

    pub fn subprogram_decl(&self) -> SubprogramDeclaration {
        self.parse_ok_no_diagnostics(parse_subprogram_declaration_no_semi)
    }

    pub fn attribute_name(&self) -> AttributeName {
        match self.parse_ok(parse_name).item {
            Name::Attribute(attr) => *attr,
            name => panic!("Expected attribute got {:?}", name),
        }
    }

    /// Get a keyword token from the n:th occurance of kind or last occurance if
    /// occurance = -1
    pub fn keyword_token(&self, kind: Kind, occurance: isize) -> KeyWordToken {
        let mut count = 0;
        let mut keyword_token = None;
        for token in self.tokenize().iter() {
            if token.kind == kind {
                count += 1;
                if occurance == -1 || count == occurance {
                    keyword_token = Some(KeyWordToken {
                        kind: token.kind,
                        pos: token.pos.clone(),
                        comments: token.comments.clone(),
                    });
                    if count == occurance {
                        break;
                    }
                }
            }
        }
        keyword_token.unwrap()
    }
}

fn substr_range(source: &Source, range: Range, substr: &str, occurence: usize) -> Range {
    let contents = source.contents();
    let mut reader = ContentReader::new(&contents);
    let mut count = occurence;

    reader.seek_pos(range.start);

    while reader.pos() < range.end {
        if reader.matches(&substr) {
            count -= 1;
            if count == 0 {
                let start = reader.pos();
                for _ in substr.chars() {
                    reader.skip();
                }
                if reader.pos() <= range.end {
                    return Range::new(start, reader.pos());
                }
            }
        }

        reader.skip();
    }

    panic!(
        "Could not find occurence {} of substring {:?}",
        occurence, substr
    );
}

/// Fast forward tokenstream until position
fn forward(stream: &mut TokenStream, start: Position) {
    loop {
        let token = stream.peek_expect().unwrap();
        if token.pos.start() >= start {
            break;
        }
        stream.move_after(&token);
    }
}

/// Check that no errors where found
pub fn check_no_diagnostics(diagnostics: &Vec<Diagnostic>) {
    for err in diagnostics.iter() {
        println!("{}", err.show());
    }
    if diagnostics.len() > 0 {
        panic!("Found errors");
    }
}

/// Create map from diagnostic -> count
fn diagnostics_to_map(diagnostics: Vec<Diagnostic>) -> HashMap<Diagnostic, usize> {
    let mut map = HashMap::new();
    for diagnostic in diagnostics {
        match map.entry(diagnostic) {
            Entry::Occupied(mut entry) => {
                let count = *entry.get() + 1;
                entry.insert(count);
            }
            Entry::Vacant(entry) => {
                entry.insert(1);
            }
        }
    }
    map
}

/// Check diagnostics are equal without considering order
pub fn check_diagnostics(got: Vec<Diagnostic>, expected: Vec<Diagnostic>) {
    let mut expected = diagnostics_to_map(expected);
    let mut got = diagnostics_to_map(got);

    let mut found_errors = false;

    for (diagnostic, count) in expected.drain() {
        match got.remove(&diagnostic) {
            Some(got_count) => {
                if count != got_count {
                    found_errors = true;
                    println!("-------------------------------------------------------");
                    println!(
                        "Got right diagnostic but wrong count {}, expected {}",
                        got_count, count
                    );
                    println!("-------------------------------------------------------");
                    println!("{}", diagnostic.show());
                }
            }
            None => {
                found_errors = true;
                println!("-------------------------------------------------------");
                println!("Got no diagnostic, expected {}", count);
                println!("-------------------------------------------------------");
                println!("{}", diagnostic.show());
            }
        }
    }

    for (diagnostic, _) in got.drain() {
        found_errors = true;
        println!("-------------------------------------------------------");
        println!("Got unexpected diagnostic");
        println!("-------------------------------------------------------");
        println!("{}", diagnostic.show());
    }

    if found_errors {
        panic!("Found diagnostic mismatch");
    }
}

fn compare_unordered<T: PartialEq + Debug>(got: &[T], expected: &[T]) -> bool {
    if got.len() != expected.len() {
        return false;
    }
    for exp in expected.iter() {
        if !got.contains(exp) {
            return false;
        }
    }
    true
}

pub fn assert_eq_unordered<T: PartialEq + Debug>(got: &[T], expected: &[T]) {
    if !compare_unordered(got, expected) {
        panic!(
            "\ngot(len={}): {:?}\nexp(len={}): {:?}",
            got.len(),
            got,
            expected.len(),
            expected
        );
    }
}

impl AsRef<SrcPos> for Code {
    fn as_ref(&self) -> &SrcPos {
        &self.pos
    }
}

// // Create a Range spanning from the first substring occurance of start to
// // to the first occurance of end after start (may overlap)
// pub fn source_range(code: &Code, start: &str, end: &str) -> SrcPos {
//     let start = code.s1(start).pos;
//     let mut end_occurance = 1;
//     loop {
//         let end = code.s(end, end_occurance).pos;
//         if end.range().start.line >= start.range().start.line
//             && end.range().end.line >= start.range().end.line
//         {
//             break start.combine_into(&end);
//         }
//         end_occurance += 1;
//         if end_occurance > 1000 {
//             panic!("Unable to find range");
//         }
//     }
// }

// pub fn source_range_between(code: &Code, start: &str, end: &str) -> SrcPos {
//     let start = code.s1(start).pos;
//     let mut end_occurance = 1;
//     loop {
//         let end = code.s(end, end_occurance).pos;
//         if end.range().start.line >= start.range().start.line
//             && end.range().end.line >= start.range().end.line
//         {
//             break SrcPos::new(
//                 code.source().clone(),
//                 Range {
//                     start: start.pos().end(),
//                     end: end.pos().start(),
//                 },
//             );
//         }
//         end_occurance += 1;
//         if end_occurance > 1000 {
//             panic!("Unable to find range");
//         }
//     }
// }

mod tests {
    use super::*;

    #[test]
    fn check_diagnostics_ok() {
        let code = Code::new("foo bar");
        check_diagnostics(
            vec![Diagnostic::error(code.s1("foo"), "hello")],
            vec![Diagnostic::error(code.s1("foo"), "hello")],
        )
    }

    #[test]
    fn check_diagnostics_ok_out_of_order() {
        let code = Code::new("foo bar");
        check_diagnostics(
            vec![
                Diagnostic::error(code.s1("foo"), "hello"),
                Diagnostic::error(code.s1("bar"), "msg"),
            ],
            vec![
                Diagnostic::error(code.s1("bar"), "msg"),
                Diagnostic::error(code.s1("foo"), "hello"),
            ],
        )
    }

    #[test]
    #[should_panic]
    fn check_diagnostics_not_ok_mismatch() {
        let code = Code::new("foo bar");
        check_diagnostics(
            vec![Diagnostic::error(code.s1("bar"), "msg")],
            vec![Diagnostic::error(code.s1("foo"), "hello")],
        )
    }

    #[test]
    #[should_panic]
    fn check_diagnostics_not_ok_count_mismatch() {
        let code = Code::new("foo bar");
        check_diagnostics(
            vec![
                Diagnostic::error(code.s1("bar"), "msg"),
                Diagnostic::error(code.s1("bar"), "msg"),
            ],
            vec![Diagnostic::error(code.s1("bar"), "msg")],
        )
    }

    #[test]
    #[should_panic]
    fn check_diagnostics_not_ok_missing() {
        let code = Code::new("foo bar");
        check_diagnostics(
            vec![Diagnostic::error(code.s1("bar"), "msg")],
            vec![
                Diagnostic::error(code.s1("bar"), "msg"),
                Diagnostic::error(code.s1("bar"), "missing"),
            ],
        )
    }

    #[test]
    #[should_panic]
    fn check_diagnostics_not_ok_unexpected() {
        let code = Code::new("foo bar");
        check_diagnostics(
            vec![
                Diagnostic::error(code.s1("bar"), "msg"),
                Diagnostic::error(code.s1("bar"), "unexpected"),
            ],
            vec![Diagnostic::error(code.s1("bar"), "msg")],
        )
    }
}
