// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2018, Olof Kraigher olof.kraigher@gmail.com

use crate::ast;
use crate::ast::{BaseSpecifier, Ident};
use crate::data::*;

/// The kind of a Token
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Kind {
    // Keywords
    Architecture,
    Entity,
    Configuration,
    Package,
    Block,
    Process,
    Generate,
    Postponed,
    Library,
    Label,
    Use,
    Context,
    Body,
    Component,
    Is,
    Return,
    Null,
    Of,
    On,
    Generic,
    Map,
    Default,
    Port,
    Attribute,
    Begin,
    If,
    Loop,
    While,
    Case,
    Else,
    Elsif,
    Then,
    When,
    With,
    Select,
    Next,
    Exit,
    For,
    Force,
    Release,
    Assert,
    Report,
    Severity,
    Wait,
    After,
    Transport,
    Inertial,
    Reject,
    Unaffected,
    Until,
    End,
    All,
    Range,
    Downto,
    To,
    In,
    Out,
    InOut,
    Buffer,
    Linkage,
    Signal,
    Constant,
    Variable,
    File,
    Open,
    Alias,
    Shared,
    Others,
    Record,
    Type,
    Subtype,
    Access,
    Units,
    New,
    Array,
    Protected,
    Impure,
    Function,
    Procedure,
    Vunit,

    // Unary operators
    Abs,
    Not,

    // Unary and binary operators
    Plus,
    Minus,
    QueQue,

    // Binary operators
    And,
    Or,
    Nand,
    Nor,
    Xor,
    Xnor,
    SLL,
    SRL,
    SLA,
    SRA,
    ROL,
    ROR,

    Mod,
    Rem,

    EQ,
    NE,
    LT,
    LTE,
    GT,
    GTE,

    QueEQ,
    QueNE,
    QueLT,
    QueLTE,
    QueGT,
    QueGTE,
    Que,

    Times,
    Pow,
    Div,

    Identifier,
    AbstractLiteral,
    StringLiteral,
    BitString,
    Character,
    Tick,
    LeftPar,
    RightPar,
    LeftSquare,
    RightSquare,
    SemiColon,
    Colon,
    Bar,
    Dot,
    BOX,
    LtLt,
    GtGt,
    Circ,
    CommAt,
    Concat,
    Comma,
    ColonEq,
    RightArrow,
}
use self::Kind::*;

/// Expect any number of token kind patterns, returns Ok variant when
/// match else Err variant
#[macro_export]
macro_rules! match_token_kind {
    ($token:expr, $($($kind:ident)|+ => $result:expr),*) => {
        {
            match $token.kind {
                $(
                    $($kind)|+ => $result
                ),*,
                _ => {
                    let mut kinds = Vec::new();
                    $(
                        $(
                            kinds.push($kind);
                        )*
                    )*

                    Err($token.kinds_error(&kinds))
                }
            }
        }
    }
}

/// Expect any number of token kind patterns, return on no match with
/// error diagnostic based on expected kinds
#[macro_export]
macro_rules! try_token_kind {
    ($token:expr, $($($kind:ident)|+ => $result:expr),*) => {
        match $token.kind {
            $(
                $($kind)|+ => $result
            ),*,
            _ => {
                let mut kinds = Vec::new();
                $(
                    $(
                        kinds.push($kind);
                    )*
                )*

                return Err($token.kinds_error(&kinds));
            }
        }
    }
}

pub fn kind_str(kind: Kind) -> &'static str {
    match kind {
        // Keywords
        Architecture => &"architecture",
        Entity => &"entity",
        Configuration => &"configuration",
        Package => &"package",
        Block => &"block",
        Process => &"process",
        Generate => &"generate",
        Postponed => &"postponed",
        Library => &"library",
        Label => &"label",
        Use => &"use",
        Context => &"context",
        Body => &"body",
        Component => &"component",
        Is => &"is",
        Return => &"return",
        Null => &"null",
        Of => &"of",
        On => &"on",
        Generic => &"generic",
        Map => &"map",
        Default => &"default",
        Port => &"port",
        Attribute => &"attribute",
        Begin => &"begin",
        If => &"if",
        Loop => &"loop",
        While => &"while",
        Case => &"case",
        Else => &"else",
        Elsif => &"elsif",
        Then => &"then",
        When => &"when",
        With => &"with",
        Select => &"select",
        Next => &"next",
        Exit => &"exit",
        For => &"for",
        Force => &"force",
        Release => &"release",
        Assert => &"assert",
        Report => &"report",
        Severity => &"severity",
        Wait => &"wait",
        After => &"after",
        Transport => &"transport",
        Inertial => &"inertial",
        Reject => &"reject",
        Unaffected => &"unaffected",
        Until => &"until",
        End => &"end",
        All => &"all",
        Range => &"range",
        Downto => &"downto",
        To => &"to",
        In => &"in",
        Out => &"out",
        InOut => &"inout",
        Buffer => &"buffer",
        Linkage => &"linkage",
        Signal => &"signal",
        Constant => &"constant",
        Variable => &"variable",
        File => &"file",
        Open => &"open",
        Alias => &"alias",
        Shared => &"shared",
        Others => &"others",
        Record => &"record",
        Type => &"type",
        Subtype => &"subtype",
        Access => &"access",
        Units => &"units",
        New => &"new",
        Array => &"array",
        Protected => &"protected",
        Impure => &"impure",
        Function => &"function",
        Procedure => &"procedure",
        Vunit => &"vunit",

        // Unary operators
        Abs => &"abs",
        Not => &"not",

        // Unary and binary operators
        Plus => &"plus",
        Minus => &"minus",
        QueQue => &"??",

        // Binary operators
        And => &"and",
        Or => &"or",
        Nand => &"nand",
        Nor => &"nor",
        Xor => &"xor",
        Xnor => &"xnor",
        SLL => &"sll",
        SRL => &"srl",
        SLA => &"sla",
        SRA => &"sra",
        ROL => &"rol",
        ROR => &"ror",

        Mod => &"mod",
        Rem => &"rem",

        EQ => &"=",
        NE => &"/=",
        LT => &"<",
        LTE => &"<=",
        GT => &">",
        GTE => &">=",

        QueEQ => &"?=",
        QueNE => &"?/=",
        QueLT => &"?<",
        QueLTE => &"?<=",
        QueGT => &"?>",
        QueGTE => &"?>=",
        Que => &"?",

        Times => &"*",
        Pow => &"**",
        Div => &"/",

        Identifier => &"{identifier}",
        AbstractLiteral => &"{abstract_literal}",
        StringLiteral => &"{string}",
        BitString => &"{bit_string}",
        Character => &"{character}",
        Tick => &"'",
        LeftPar => &"(",
        RightPar => &")",
        LeftSquare => &"[",
        RightSquare => &"]",
        SemiColon => &";",
        Colon => &":",
        Bar => &"|",
        Dot => &".",
        BOX => &"<>",
        LtLt => &"<<",
        GtGt => &">>",
        Circ => &"^",
        CommAt => &"@",
        Concat => &"&",
        Comma => &",",
        ColonEq => &":=",
        RightArrow => &"=>",
    }
}

/// Create s string representation of the kinds separated by a separator
pub fn kinds_str(kinds: &[Kind]) -> String {
    let mut result = String::new();
    for (i, kind) in kinds.iter().enumerate() {
        result.push('\'');
        result.push_str(kind_str(*kind));
        result.push('\'');

        if i == kinds.len() - 1 {
        } else if i == kinds.len() - 2 {
            result.push_str(" or ");
        } else {
            result.push_str(", ");
        }
    }
    result
}

/// The value of a Token
#[derive(PartialEq, Clone, Debug)]
pub enum Value {
    Identifier(Symbol),
    String(Latin1String),
    BitString(ast::BitString),
    AbstractLiteral(ast::AbstractLiteral),
    Character(u8),
    NoValue,
}

/// A Token
#[derive(PartialEq, Clone, Debug)]
pub struct Token {
    pub kind: Kind,
    pub value: Value,
    pub pos: SrcPos,
    pub next_state: ReaderState,
    pub comments: Option<Box<TokenComments>>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct KeyWordToken {
    pub kind: Kind,
    pub pos: SrcPos,
    pub comments: Option<Box<TokenComments>>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct TokenComments {
    pub leading: Vec<Comment>,
    pub trailing: Option<Comment>,
}

#[derive(PartialEq, Clone, Debug)]
pub struct Comment {
    pub value: String,
    pub range: crate::data::Range,
    pub multi_line: bool,
}

use std::convert::AsRef;
impl AsRef<SrcPos> for Token {
    fn as_ref(&self) -> &SrcPos {
        &self.pos
    }
}

use std::convert::Into;
impl Into<SrcPos> for Token {
    fn into(self) -> SrcPos {
        self.pos
    }
}

impl From<Token> for KeyWordToken {
    fn from(token: Token) -> KeyWordToken {
        KeyWordToken {
            kind: token.kind,
            pos: token.pos,
            comments: token.comments,
        }
    }
}

pub fn kinds_error<T: AsRef<SrcPos>>(pos: T, kinds: &[Kind]) -> Diagnostic {
    Diagnostic::error(
        pos.as_ref(),
        format!("Expected {}", kinds_str(&kinds)).as_str(),
    )
}

impl Token {
    pub fn kinds_error(&self, kinds: &[Kind]) -> Diagnostic {
        kinds_error(self, kinds)
    }

    pub fn expect_ident(self) -> DiagnosticResult<Ident> {
        if let Token {
            kind: Identifier,
            value: Value::Identifier(value),
            pos,
            ..
        } = self
        {
            Ok(WithPos::from(value, pos))
        } else {
            Err(self.kinds_error(&[Identifier]))
        }
    }

    pub fn expect_character(self) -> DiagnosticResult<WithPos<u8>> {
        if let Token {
            kind: Character,
            value: Value::Character(value),
            pos,
            ..
        } = self
        {
            Ok(WithPos::from(value, pos))
        } else {
            Err(self.kinds_error(&[Character]))
        }
    }

    pub fn expect_kind(self, kind: Kind) -> DiagnosticResult<Token> {
        if self.kind == kind {
            Ok(self)
        } else {
            Err(self.kinds_error(&[kind]))
        }
    }

    pub fn expect_bit_string(self) -> DiagnosticResult<WithPos<ast::BitString>> {
        if let Token {
            kind: BitString,
            value: Value::BitString(value),
            pos,
            ..
        } = self
        {
            Ok(WithPos::from(value, pos))
        } else {
            Err(self.kinds_error(&[BitString]))
        }
    }

    pub fn expect_abstract_literal(self) -> DiagnosticResult<WithPos<ast::AbstractLiteral>> {
        if let Token {
            kind: AbstractLiteral,
            value: Value::AbstractLiteral(value),
            pos,
            ..
        } = self
        {
            Ok(WithPos::from(value, pos))
        } else {
            Err(self.kinds_error(&[AbstractLiteral]))
        }
    }

    pub fn expect_string(self) -> DiagnosticResult<WithPos<Latin1String>> {
        if let Token {
            kind: StringLiteral,
            value: Value::String(value),
            pos,
            ..
        } = self
        {
            Ok(WithPos::from(value, pos))
        } else {
            Err(self.kinds_error(&[StringLiteral]))
        }
    }
}

struct TokenError {
    range: crate::data::Range,
    message: String,
}

impl TokenError {
    fn range(start: Position, end: Position, message: impl Into<String>) -> TokenError {
        TokenError {
            range: crate::data::Range::new(start, end),
            message: message.into(),
        }
    }

    fn pos(pos: Position, message: impl Into<String>) -> TokenError {
        Self::range(pos, pos.next_char(), message)
    }
}

impl From<Utf8ToLatin1Error> for TokenError {
    fn from(err: Utf8ToLatin1Error) -> TokenError {
        TokenError::range(err.pos, err.pos.after_char(err.value), err.message())
    }
}

/// Resolves ir1045
/// http://www.eda-stds.org/isac/IRs-VHDL-93/IR1045.txt
/// char may not come after ], ), all, or identifier
fn can_be_char(last_token_kind: Option<Kind>) -> bool {
    if let Some(kind) = last_token_kind {
        match kind {
            RightSquare | RightPar | All | Identifier => false,
            _ => true,
        }
    } else {
        true
    }
}

fn parse_integer(
    reader: &mut ContentReader,
    base: u64,
    stop_on_suffix: bool,
) -> Result<u64, TokenError> {
    let mut result = Some(0 as u64);
    let mut too_large_digit = None;
    let mut invalid_character = None;

    let start = reader.pos();
    while let Some(b) = reader.peek()? {
        let digit = u64::from(match b {
            // Bit string literal or exponent
            // Lower case
            b's' | b'u' | b'b' | b'o' | b'x' | b'd' | b'e' if stop_on_suffix => {
                break;
            }
            // Upper case
            b'S' | b'U' | b'B' | b'O' | b'X' | b'D' | b'E' if stop_on_suffix => {
                break;
            }

            b'0'..=b'9' => (b - b'0'),
            b'a'..=b'f' => (10 + b - b'a'),
            b'A'..=b'F' => (10 + b - b'A'),
            b'_' => {
                reader.skip();
                continue;
            }
            b'g'..=b'z' | b'G'..=b'Z' => {
                invalid_character = Some((b, reader.pos()));
                reader.skip();
                continue;
            }
            _ => {
                break;
            }
        });

        if digit >= base {
            too_large_digit = Some((b, reader.pos()));
        }

        reader.skip();

        result = result
            .and_then(|x| base.checked_mul(x))
            .and_then(|x| x.checked_add(digit));
    }

    if let Some((b, pos)) = invalid_character {
        Err(TokenError::pos(
            pos,
            format!("Invalid integer character '{}'", Latin1String::new(&[b])),
        ))
    } else if let Some((b, pos)) = too_large_digit {
        Err(TokenError::pos(
            pos,
            format!(
                "Illegal digit '{}' for base {}",
                Latin1String::new(&[b]),
                base
            ),
        ))
    } else if let Some(result) = result {
        Ok(result)
    } else {
        Err(TokenError::range(
            start,
            reader.pos(),
            "Integer too large for 64-bit unsigned",
        ))
    }
}

fn parse_exponent(reader: &mut ContentReader) -> Result<i32, TokenError> {
    let start = reader.pos();
    let negative = {
        if reader.peek()? == Some(b'-') {
            reader.skip();
            true
        } else {
            reader.skip_if(b'+')?;
            false
        }
    };

    let exp = parse_integer(reader, 10, false)?;
    if negative {
        if exp <= (-(i32::min_value() as i64)) as u64 {
            return Ok((-(exp as i64)) as i32);
        }
    } else if exp <= i32::max_value() as u64 {
        return Ok(exp as i32);
    }

    Err(TokenError::range(
        start,
        reader.pos(),
        "Exponent too large for 32-bits signed",
    ))
}

#[derive(Clone, Copy)]
pub struct TokenState {
    last_token_kind: Option<Kind>,
    start: ReaderState,
}

impl TokenState {
    pub fn new(start: ReaderState) -> TokenState {
        TokenState {
            last_token_kind: None,
            start,
        }
    }

    /// Set state to after token
    pub fn set_after(&mut self, token: &Token) {
        self.last_token_kind = Some(token.kind);
        self.start = token.next_state;
    }
}

// Assumes first quote is already consumed
fn parse_quoted(
    buffer: &mut Latin1String,
    reader: &mut ContentReader,
    quote: u8,
    include_quote: bool,
) -> Result<Latin1String, TokenError> {
    let start = reader.pos();

    buffer.bytes.clear();
    let mut is_multiline = false;
    let mut found_end = false;

    if include_quote {
        buffer.bytes.push(quote)
    }

    while let Some(chr) = reader.pop()? {
        is_multiline |= chr == b'\n';
        if chr == quote {
            if reader.peek()? == Some(quote) {
                reader.skip();
            } else {
                found_end = true;
                break;
            }
        }
        buffer.bytes.push(chr);
    }

    if include_quote {
        buffer.bytes.push(quote)
    }

    if !found_end {
        Err(TokenError::range(
            start.prev_char(),
            reader.pos(),
            "Reached EOF before end quote",
        ))
    } else if is_multiline {
        Err(TokenError::range(
            start.prev_char(),
            reader.pos(),
            "Multi line string",
        ))
    } else {
        Ok(buffer.clone())
    }
}

// Assumes first quote is already consumed
fn parse_string(
    buffer: &mut Latin1String,
    reader: &mut ContentReader,
) -> Result<Latin1String, TokenError> {
    parse_quoted(buffer, reader, b'"', false)
}

/// Assume -- has already been consumed
fn parse_comment(reader: &mut ContentReader) -> Comment {
    let start_pos = reader.pos().prev_char().prev_char();
    let mut value = String::new();
    while let Some(chr) = reader.peek_char() {
        if chr == '\n' {
            break;
        } else {
            reader.skip();
            value.push(chr);
        }
    }
    let end_pos = reader.pos();
    Comment {
        value,
        range: start_pos.range_to(end_pos),
        multi_line: false,
    }
}

/// Assume /* has been consumed
fn parse_multi_line_comment(reader: &mut ContentReader) -> Result<Comment, TokenError> {
    let start_pos = reader.pos().prev_char().prev_char();
    let mut value = String::new();
    while let Some(chr) = reader.pop_char() {
        if chr == '*' {
            if reader.peek_char() == Some('/') {
                reader.skip();
                // Comment ended
                let end_pos = reader.pos();
                return Ok(Comment {
                    value,
                    range: start_pos.range_to(end_pos),
                    multi_line: true,
                });
            } else {
                value.push(chr);
            }
        } else {
            value.push(chr);
        }
    }

    let end_pos = reader.pos();
    Err(TokenError::range(
        start_pos,
        end_pos,
        "Incomplete multi-line comment",
    ))
}

fn parse_real_literal(
    buffer: &mut Latin1String,
    reader: &mut ContentReader,
) -> Result<f64, TokenError> {
    buffer.bytes.clear();
    let start = reader.pos();
    while let Some(b) = reader.peek_lowercase()? {
        match b {
            b'e' => {
                break;
            }
            b'E' => {
                break;
            }
            b'0'..=b'9' | b'a'..=b'd' | b'f' | b'A'..=b'F' | b'.' => {
                reader.skip();
                buffer.bytes.push(b);
            }
            b'_' => {
                reader.skip();
                continue;
            }
            _ => {
                break;
            }
        };
    }

    let string = unsafe { std::str::from_utf8_unchecked(&buffer.bytes) };

    let result: Result<f64, TokenError> =
        string.parse().map_err(|err: std::num::ParseFloatError| {
            TokenError::range(start, reader.pos(), err.to_string())
        });
    result
}

fn exponentiate(value: u64, exp: u32) -> Option<u64> {
    (10 as u64)
        .checked_pow(exp)
        .and_then(|x| x.checked_mul(value))
}

/// LRM 15.5 Abstract literals
fn parse_abstract_literal(
    buffer: &mut Latin1String,
    reader: &mut ContentReader,
) -> Result<(Kind, Value), TokenError> {
    let state = reader.state();
    let initial = parse_integer(reader, 10, true);
    let pos_after_initial = reader.pos();

    match reader.peek_lowercase()? {
        // Real
        Some(b'.') => {
            reader.set_state(state);
            let real = parse_real_literal(buffer, reader)?;

            match reader.peek()? {
                // Exponent
                Some(b'e') | Some(b'E') => {
                    reader.skip();
                    let exp = parse_exponent(reader)?;
                    Ok((
                        AbstractLiteral,
                        Value::AbstractLiteral(ast::AbstractLiteral::Real(
                            real * (10.0 as f64).powi(exp),
                        )),
                    ))
                }
                _ => Ok((
                    AbstractLiteral,
                    Value::AbstractLiteral(ast::AbstractLiteral::Real(real)),
                )),
            }
        }

        // Integer exponent
        Some(b'e') => {
            let integer = initial?;
            reader.skip();
            let exp = parse_exponent(reader)?;
            if exp >= 0 {
                if let Some(value) = exponentiate(integer, exp as u32) {
                    Ok((
                        AbstractLiteral,
                        Value::AbstractLiteral(ast::AbstractLiteral::Integer(value)),
                    ))
                } else {
                    Err(TokenError::range(
                        state.pos(),
                        reader.pos(),
                        "Integer too large for 64-bit unsigned",
                    ))
                }
            } else {
                Err(TokenError::range(
                    state.pos(),
                    reader.pos(),
                    "Integer literals may not have negative exponent",
                ))
            }
        }

        // Based integer
        Some(b'#') => {
            let base = initial?;
            reader.skip();
            let base_result = parse_integer(reader, base, false);

            if let Some(b'#') = reader.peek()? {
                reader.skip();
                let integer = base_result?;
                if base >= 2 && base <= 16 {
                    Ok((
                        AbstractLiteral,
                        Value::AbstractLiteral(ast::AbstractLiteral::Integer(integer)),
                    ))
                } else {
                    Err(TokenError::range(
                        state.pos(),
                        pos_after_initial,
                        format!("Base must be at least 2 and at most 16, got {}", base),
                    ))
                }
            } else {
                Err(TokenError::range(
                    state.pos(),
                    reader.pos(),
                    "Based integer did not end with #",
                ))
            }
        }

        // Bit string literal
        Some(b's') | Some(b'u') | Some(b'b') | Some(b'o') | Some(b'x') | Some(b'd') => {
            let integer = initial?;

            if let Some(base_spec) = parse_base_specifier(reader)? {
                // @TODO check overflow
                parse_bit_string(buffer, reader, base_spec, Some(integer as u32))
            } else {
                Err(TokenError::range(
                    state.pos(),
                    reader.pos(),
                    "Invalid bit string literal",
                ))
            }
        }
        _ => {
            // Plain integer
            Ok((
                AbstractLiteral,
                Value::AbstractLiteral(ast::AbstractLiteral::Integer(initial?)),
            ))
        }
    }
}

/// LRM 15.8 Bit string literals
/// Parse the base specifier such as ub, sx, b etc
/// Also requires and consumes the trailing quoute "
fn parse_base_specifier(reader: &mut ContentReader) -> Result<Option<BaseSpecifier>, TokenError> {
    let base_specifier = match reader.pop_lowercase()? {
        Some(b'u') => match reader.pop_lowercase()? {
            Some(b'b') => BaseSpecifier::UB,
            Some(b'o') => BaseSpecifier::UO,
            Some(b'x') => BaseSpecifier::UX,
            _ => return Ok(None),
        },
        Some(b's') => match reader.pop_lowercase()? {
            Some(b'b') => BaseSpecifier::SB,
            Some(b'o') => BaseSpecifier::SO,
            Some(b'x') => BaseSpecifier::SX,
            _ => return Ok(None),
        },
        Some(b'b') => BaseSpecifier::B,
        Some(b'o') => BaseSpecifier::O,
        Some(b'x') => BaseSpecifier::X,
        Some(b'd') => BaseSpecifier::D,
        _ => return Ok(None),
    };

    Ok(if reader.pop()? == Some(b'"') {
        Some(base_specifier)
    } else {
        None
    })
}

// Only consume reader if it is a base specifier
fn maybe_base_specifier(reader: &mut ContentReader) -> Result<Option<BaseSpecifier>, TokenError> {
    let mut lookahead = reader.clone();
    if let Some(value) = parse_base_specifier(&mut lookahead)? {
        reader.set_to(&lookahead);
        Ok(Some(value))
    } else {
        Ok(None)
    }
}

fn parse_bit_string(
    buffer: &mut Latin1String,
    reader: &mut ContentReader,
    base_specifier: BaseSpecifier,
    bit_string_length: Option<u32>,
) -> Result<(Kind, Value), TokenError> {
    let value = match parse_string(buffer, reader) {
        Ok(value) => value,
        Err(mut err) => {
            err.message = "Invalid bit string literal".to_string();
            return Err(err);
        }
    };

    Ok((
        BitString,
        Value::BitString(ast::BitString {
            length: bit_string_length,
            base: base_specifier,
            value,
        }),
    ))
}

/// LRM 15.4 Identifiers
fn parse_basic_identifier_or_keyword(
    buffer: &mut Latin1String,
    reader: &mut ContentReader,
    symbols: &Symbols,
) -> Result<(Kind, Value), TokenError> {
    buffer.bytes.clear();
    while let Some(b) = reader.peek()? {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' => {
                buffer.bytes.push(b);
                reader.skip();
            }
            _ => {
                break;
            }
        }
    }

    Ok(symbols.insert_or_keyword(&buffer))
}

/// Assumes leading ' has already been consumed
/// Only consumes from reader if Some is returned
fn parse_character_literal(
    reader: &mut ContentReader,
) -> Result<Option<(Kind, Value)>, TokenError> {
    let mut lookahead = reader.clone();

    if let Some(chr) = lookahead.pop()? {
        if lookahead.skip_if(b'\'')? {
            reader.set_to(&lookahead);
            Ok(Some((Character, Value::Character(chr))))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

fn get_leading_comments(reader: &mut ContentReader) -> Result<Vec<Comment>, TokenError> {
    let mut comments: Vec<Comment> = Vec::new();

    loop {
        skip_whitespace(reader);
        let state = reader.state();

        let byte = if let Some(byte) = reader.pop()? {
            byte
        } else {
            break;
        };

        match byte {
            b'/' => {
                if reader.pop()? == Some(b'*') {
                    comments.push(parse_multi_line_comment(reader)?);
                } else {
                    reader.set_state(state);
                    break;
                }
            }
            b'-' => {
                if reader.pop()? == Some(b'-') {
                    comments.push(parse_comment(reader));
                } else {
                    reader.set_state(state);
                    break;
                }
            }
            _ => {
                reader.set_state(state);
                break;
            }
        }
    }

    Ok(comments)
}

/// Skip whitespace but not newline
fn skip_whitespace_in_line(reader: &mut ContentReader) {
    while let Ok(Some(byte)) = reader.peek() {
        match byte {
            b' ' | b'\t' => {
                reader.skip();
            }
            _ => {
                break;
            }
        }
    }
}

/// Skip all whitespace
fn skip_whitespace(reader: &mut ContentReader) {
    while let Ok(Some(byte)) = reader.peek() {
        match byte {
            b' ' | b'\t' | b'\n' => {
                reader.skip();
            }
            _ => {
                break;
            }
        }
    }
}

fn get_trailing_comment(reader: &mut ContentReader) -> Result<Option<Comment>, TokenError> {
    skip_whitespace_in_line(reader);

    let state = reader.state();
    match reader.pop()? {
        Some(b'-') => {
            if reader.pop()? == Some(b'-') {
                Ok(Some(parse_comment(reader)))
            } else {
                reader.set_state(state);
                Ok(None)
            }
        }
        _ => {
            reader.set_state(state);
            Ok(None)
        }
    }
}

/// Static tokenizer data
pub struct Symbols {
    symtab: SymbolTable,
    keywords: Vec<Kind>,
}

impl Symbols {
    pub fn symtab(&self) -> &SymbolTable {
        &self.symtab
    }

    fn insert_or_keyword(&self, name: &Latin1String) -> (Kind, Value) {
        let symbol = self.symtab.insert(name);
        if let Some(kind) = self.keywords.get(symbol.id) {
            (*kind, Value::NoValue)
        } else {
            (Identifier, Value::Identifier(symbol))
        }
    }
}

impl std::default::Default for Symbols {
    fn default() -> Symbols {
        let keywords_init = [
            ("architecture", Architecture),
            ("entity", Entity),
            ("configuration", Configuration),
            ("package", Package),
            ("block", Block),
            ("process", Process),
            ("generate", Generate),
            ("postponed", Postponed),
            ("library", Library),
            ("label", Label),
            ("use", Use),
            ("context", Context),
            ("body", Body),
            ("component", Component),
            ("is", Is),
            ("return", Return),
            ("null", Null),
            ("of", Of),
            ("on", On),
            ("generic", Generic),
            ("map", Map),
            ("default", Default),
            ("port", Port),
            ("attribute", Attribute),
            ("begin", Begin),
            ("end", End),
            ("if", If),
            ("loop", Loop),
            ("while", While),
            ("case", Case),
            ("else", Else),
            ("elsif", Elsif),
            ("then", Then),
            ("when", When),
            ("with", With),
            ("select", Select),
            ("next", Next),
            ("exit", Exit),
            ("for", For),
            ("force", Force),
            ("release", Release),
            ("assert", Assert),
            ("report", Report),
            ("severity", Severity),
            ("wait", Wait),
            ("after", After),
            ("transport", Transport),
            ("inertial", Inertial),
            ("reject", Reject),
            ("unaffected", Unaffected),
            ("until", Until),
            ("all", All),
            ("range", Range),
            ("downto", Downto),
            ("to", To),
            ("in", In),
            ("out", Out),
            ("inout", InOut),
            ("buffer", Buffer),
            ("linkage", Linkage),
            ("signal", Signal),
            ("constant", Constant),
            ("variable", Variable),
            ("file", File),
            ("open", Open),
            ("alias", Alias),
            ("shared", Shared),
            ("others", Others),
            ("record", Record),
            ("type", Type),
            ("subtype", Subtype),
            ("access", Access),
            ("units", Units),
            ("new", New),
            ("array", Array),
            ("protected", Protected),
            ("impure", Impure),
            ("function", Function),
            ("procedure", Procedure),
            ("abs", Abs),
            ("not", Not),
            ("and", And),
            ("or", Or),
            ("nand", Nand),
            ("nor", Nor),
            ("xor", Xor),
            ("xnor", Xnor),
            ("sll", SLL),
            ("srl", SRL),
            ("sla", SLA),
            ("sra", SRA),
            ("rol", ROL),
            ("ror", ROR),
            ("mod", Mod),
            ("rem", Rem),
            ("vunit", Vunit),
        ];

        let symtab = SymbolTable::default();
        let mut keywords = Vec::with_capacity(keywords_init.len());

        let mut latin1 = Latin1String::empty();
        for (keyword, kind) in keywords_init.iter() {
            latin1.bytes.clear();
            latin1.bytes.extend_from_slice(keyword.as_bytes());
            let symbol = symtab.insert(&latin1);
            assert_eq!(symbol.id, keywords.len());
            keywords.push(*kind);
        }

        Symbols { symtab, keywords }
    }
}

pub struct Tokenizer<'a> {
    symbols: &'a Symbols,
    buffer: Latin1String,
    state: TokenState,
    source: &'a Source,
    reader: ContentReader<'a>,
    final_comments: Option<Vec<Comment>>,
    range_sym: Symbol,
    reverse_range_sym: Symbol,
}

impl<'a> Tokenizer<'a> {
    pub fn new(
        symbols: &'a Symbols,
        source: &'a Source,
        reader: ContentReader<'a>,
    ) -> Tokenizer<'a> {
        let range_sym = symbols.symtab().insert(&Latin1String::new(b"range"));
        let reverse_range_sym = symbols
            .symtab()
            .insert(&Latin1String::new(b"reverse_range"));

        Tokenizer {
            symbols,
            state: TokenState::new(reader.state()),
            buffer: Latin1String::empty(),
            source,
            reader,
            final_comments: None,
            range_sym,
            reverse_range_sym,
        }
    }

    pub fn state(&self) -> TokenState {
        self.state
    }

    pub fn eof_error(&self) -> Diagnostic {
        Diagnostic::error(
            self.source
                .pos(self.state.start.pos(), self.state.start.pos().next_char()),
            "Unexpected EOF",
        )
    }

    pub fn set_state(&mut self, state: TokenState) {
        self.state = state;
        self.reader.set_state(state.start);
    }

    pub fn move_after(&mut self, token: &Token) {
        self.state.set_after(token);
        self.reader.set_state(self.state.start);
    }

    pub fn range_sym(&self) -> &Symbol {
        &self.range_sym
    }

    pub fn reverse_range_sym(&self) -> &Symbol {
        &self.reverse_range_sym
    }

    fn parse_token(&mut self) -> Result<Option<(Kind, Value)>, TokenError> {
        macro_rules! illegal_token {
            () => {
                return Err(TokenError::range(
                    self.state.start.pos(),
                    self.reader.pos(),
                    "Illegal token",
                ));
            };
        }

        let byte = if let Some(byte) = self.reader.peek()? {
            byte
        } else {
            // End of file
            return Ok(None);
        };

        let (kind, value) = match byte {
            b'a'..=b'z' | b'A'..=b'Z' => {
                if let Some(base_spec) = maybe_base_specifier(&mut self.reader)? {
                    parse_bit_string(&mut self.buffer, &mut self.reader, base_spec, None)?
                } else {
                    parse_basic_identifier_or_keyword(
                        &mut self.buffer,
                        &mut self.reader,
                        self.symbols,
                    )?
                }
            }
            b'0'..=b'9' => parse_abstract_literal(&mut self.buffer, &mut self.reader)?,
            b':' => {
                self.reader.skip();
                if self.reader.skip_if(b'=')? {
                    (ColonEq, Value::NoValue)
                } else {
                    (Colon, Value::NoValue)
                }
            }
            b'\'' => {
                self.reader.skip();
                if can_be_char(self.state.last_token_kind) {
                    if let Some(chr_lit) = parse_character_literal(&mut self.reader)? {
                        chr_lit
                    } else {
                        (Tick, Value::NoValue)
                    }
                } else {
                    (Tick, Value::NoValue)
                }
            }
            b'-' => {
                self.reader.skip();
                (Minus, Value::NoValue)
            }
            b'"' => {
                self.reader.skip();
                let result = parse_string(&mut self.buffer, &mut self.reader)?;
                (StringLiteral, Value::String(result))
            }
            b';' => {
                self.reader.skip();
                (SemiColon, Value::NoValue)
            }
            b'(' => {
                self.reader.skip();
                (LeftPar, Value::NoValue)
            }
            b')' => {
                self.reader.skip();
                (RightPar, Value::NoValue)
            }
            b'+' => {
                self.reader.skip();
                (Plus, Value::NoValue)
            }
            b'.' => {
                self.reader.skip();
                (Dot, Value::NoValue)
            }
            b'&' => {
                self.reader.skip();
                (Concat, Value::NoValue)
            }
            b',' => {
                self.reader.skip();
                (Comma, Value::NoValue)
            }
            b'=' => {
                self.reader.skip();
                if self.reader.skip_if(b'>')? {
                    (RightArrow, Value::NoValue)
                } else {
                    (EQ, Value::NoValue)
                }
            }
            b'<' => {
                self.reader.skip();
                match self.reader.peek()? {
                    Some(b'=') => {
                        self.reader.skip();
                        (LTE, Value::NoValue)
                    }
                    Some(b'>') => {
                        self.reader.skip();
                        (BOX, Value::NoValue)
                    }
                    Some(b'<') => {
                        self.reader.skip();
                        (LtLt, Value::NoValue)
                    }
                    _ => (LT, Value::NoValue),
                }
            }
            b'>' => {
                self.reader.skip();
                match self.reader.peek()? {
                    Some(b'=') => {
                        self.reader.skip();
                        (GTE, Value::NoValue)
                    }
                    Some(b'>') => {
                        self.reader.skip();
                        (GtGt, Value::NoValue)
                    }
                    _ => (GT, Value::NoValue),
                }
            }
            b'/' => {
                self.reader.skip();

                if self.reader.skip_if(b'=')? {
                    (NE, Value::NoValue)
                } else {
                    (Div, Value::NoValue)
                }
            }
            b'*' => {
                self.reader.skip();

                if self.reader.skip_if(b'*')? {
                    (Pow, Value::NoValue)
                } else {
                    (Times, Value::NoValue)
                }
            }
            b'?' => {
                self.reader.skip();
                match self.reader.peek()? {
                    Some(b'?') => {
                        self.reader.skip();
                        (QueQue, Value::NoValue)
                    }
                    Some(b'=') => {
                        self.reader.skip();
                        (QueEQ, Value::NoValue)
                    }
                    Some(b'/') => {
                        self.reader.skip();
                        if self.reader.skip_if(b'=')? {
                            (QueNE, Value::NoValue)
                        } else {
                            illegal_token!();
                        }
                    }
                    Some(b'<') => {
                        self.reader.skip();
                        if self.reader.skip_if(b'=')? {
                            (QueLTE, Value::NoValue)
                        } else {
                            (QueLT, Value::NoValue)
                        }
                    }
                    Some(b'>') => {
                        self.reader.skip();
                        if self.reader.skip_if(b'=')? {
                            (QueGTE, Value::NoValue)
                        } else {
                            (QueGT, Value::NoValue)
                        }
                    }
                    _ => (Que, Value::NoValue),
                }
            }
            b'^' => {
                self.reader.skip();
                (Circ, Value::NoValue)
            }
            b'@' => {
                self.reader.skip();
                (CommAt, Value::NoValue)
            }
            b'|' => {
                self.reader.skip();
                (Bar, Value::NoValue)
            }
            b'[' => {
                self.reader.skip();
                (LeftSquare, Value::NoValue)
            }
            b']' => {
                self.reader.skip();
                (RightSquare, Value::NoValue)
            }
            b'\\' => {
                self.reader.skip();
                // LRM 15.4.3 Extended identifers
                let result = parse_quoted(&mut self.buffer, &mut self.reader, b'\\', true)?;
                let result = Value::Identifier(self.symbols.symtab().insert_extended(&result));
                (Identifier, result)
            }
            _ => {
                self.reader.skip();
                illegal_token!();
            }
        };
        Ok(Some((kind, value)))
    }

    fn pop_raw(&mut self) -> Result<Option<Token>, TokenError> {
        let leading_comments = get_leading_comments(&mut self.reader)?;
        self.state.start = self.reader.state();

        match self.parse_token()? {
            Some((kind, value)) => {
                // Parsed a token.
                let pos_start = self.state.start.pos();
                let pos_end = self.reader.pos();
                let trailing_comment = get_trailing_comment(&mut self.reader)?;
                let token_comments = if (!leading_comments.is_empty()) | trailing_comment.is_some()
                {
                    Some(Box::new(TokenComments {
                        leading: leading_comments,
                        trailing: trailing_comment,
                    }))
                } else {
                    None
                };
                let token = Token {
                    kind,
                    value,
                    pos: self.source.pos(pos_start, pos_end),
                    next_state: self.reader.state(),
                    comments: token_comments,
                };
                self.move_after(&token);
                Ok(Some(token))
            }
            None => {
                // End of file.
                self.final_comments = Some(leading_comments);
                Ok(None)
            }
        }
    }

    pub fn pop(&mut self) -> DiagnosticResult<Option<Token>> {
        match self.pop_raw() {
            Ok(token) => Ok(token),
            Err(err) => {
                self.state.start = self.reader.state();
                Err(Diagnostic::error(
                    &self.source.pos(err.range.start, err.range.end),
                    err.message,
                ))
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_final_comments(&self) -> Option<Vec<Comment>> {
        self.final_comments.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::test::Code;
    use pretty_assertions::assert_eq;

    fn state_at_end(code: &Code) -> ReaderState {
        let contents = code.source().contents();
        let mut reader = ContentReader::new(&contents);
        reader.seek_pos(code.end());
        reader.state()
    }

    fn kinds(tokens: &[Token]) -> Vec<Kind> {
        tokens.iter().map(|ref tok| tok.kind.clone()).collect()
    }

    // Shorthand for testing
    fn kinds_tokenize(code: &str) -> Vec<Kind> {
        kinds(&Code::new(code).tokenize())
    }

    // Shorthand for testing
    fn kind_value_tokenize(code: &str) -> Vec<(Kind, Value)> {
        Code::new(code)
            .tokenize()
            .iter()
            .map(|tok| (tok.kind.clone(), tok.value.clone()))
            .collect()
    }

    #[test]
    fn tokenize_keywords() {
        assert_eq!(kinds_tokenize("architecture"), vec![Architecture]);
        assert_eq!(kinds_tokenize("entity"), vec![Entity]);
        assert_eq!(kinds_tokenize("is"), vec![Is]);
        assert_eq!(kinds_tokenize("generic"), vec![Generic]);
        assert_eq!(kinds_tokenize("port"), vec![Port]);
        assert_eq!(kinds_tokenize("begin"), vec![Begin]);
        assert_eq!(kinds_tokenize("end"), vec![End]);
        assert_eq!(kinds_tokenize("all"), vec![All]);
        assert_eq!(kinds_tokenize("abs"), vec![Abs]);
        assert_eq!(kinds_tokenize("not"), vec![Not]);
    }

    #[test]
    fn tokenize_newline() {
        assert_eq!(
            kinds_tokenize(
                "
entity is
end entity"
            ),
            vec![Entity, Is, End, Entity]
        );
    }

    #[test]
    fn tokenize_pos() {
        let code = Code::new("entity foo");
        let tokens = code.tokenize();

        assert_eq!(
            tokens[0],
            Token {
                kind: Entity,
                value: Value::NoValue,
                pos: code.s1("entity").pos(),
                next_state: state_at_end(&code.s1("entity ")),
                comments: None,
            }
        );

        assert_eq!(
            tokens[1],
            Token {
                kind: Identifier,
                value: Value::Identifier(code.symbol("foo")),
                pos: code.s1("foo").pos(),
                next_state: state_at_end(&code),
                comments: None,
            }
        );
    }

    #[test]
    fn tokenize_keywords_case_insensitive() {
        assert_eq!(kinds_tokenize("entity"), vec![Entity]);
        assert_eq!(kinds_tokenize("Entity"), vec![Entity]);
        assert_eq!(kinds_tokenize("arCHitecture"), vec![Architecture]);
    }

    #[test]
    fn tokenize_identifier() {
        let code = Code::new("my_ident");
        let tokens = code.tokenize();

        assert_eq!(
            tokens,
            vec![Token {
                kind: Identifier,
                value: Value::Identifier(code.symbol("my_ident")),
                pos: code.pos(),
                next_state: state_at_end(&code),
                comments: None,
            }]
        );
    }

    #[test]
    fn tokenize_identifier_case_insensitive() {
        let code = Code::new("My_Ident");
        let tokens = code.tokenize();

        assert_eq!(
            tokens,
            vec![Token {
                kind: Identifier,
                value: Value::Identifier(code.symbol("my_ident")),
                pos: code.pos(),
                next_state: state_at_end(&code),
                comments: None,
            }]
        );
    }

    #[test]
    fn tokenize_extended_identifier() {
        let code = Code::new("\\1$my_ident\\");
        let tokens = code.tokenize();

        assert_eq!(
            tokens,
            vec![Token {
                kind: Identifier,
                value: Value::Identifier(code.symbol("\\1$my_ident\\")),
                pos: code.pos(),
                next_state: state_at_end(&code),
                comments: None,
            }]
        );
        let code = Code::new("\\my\\\\_ident\\");
        let tokens = code.tokenize();
        assert_eq!(
            tokens,
            vec![Token {
                kind: Identifier,
                value: Value::Identifier(code.symbol("\\my\\_ident\\")),
                pos: code.pos(),
                next_state: state_at_end(&code),
                comments: None,
            }]
        );
    }

    #[test]
    fn tokenize_many_identifiers() {
        let code = Code::new("my_ident my_other_ident");
        let tokens = code.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: Identifier,
                    value: Value::Identifier(code.symbol("my_ident")),
                    pos: code.s1("my_ident").pos(),
                    next_state: state_at_end(&code.s1("my_ident ")),
                    comments: None,
                },
                Token {
                    kind: Identifier,
                    value: Value::Identifier(code.symbol("my_other_ident")),
                    pos: code.s1("my_other_ident").pos(),
                    next_state: state_at_end(&code),
                    comments: None,
                },
            ]
        );
    }

    #[test]
    fn tokenize_integer() {
        assert_eq!(
            kind_value_tokenize("100 -123 1_6_2 1e3 2E4"),
            vec![
                (
                    AbstractLiteral,
                    Value::AbstractLiteral(ast::AbstractLiteral::Integer(100))
                ),
                (Minus, Value::NoValue),
                (
                    AbstractLiteral,
                    Value::AbstractLiteral(ast::AbstractLiteral::Integer(123))
                ),
                (
                    AbstractLiteral,
                    Value::AbstractLiteral(ast::AbstractLiteral::Integer(162))
                ),
                (
                    AbstractLiteral,
                    Value::AbstractLiteral(ast::AbstractLiteral::Integer(1000))
                ),
                (
                    AbstractLiteral,
                    Value::AbstractLiteral(ast::AbstractLiteral::Integer(20000))
                ),
            ]
        );
    }

    #[test]
    fn tokenize_non_latin1_error() {
        // Euro takes 1 utf-16 code and Bomb emojii requires 2 utf-16 codes
        let code = Code::new("€\u{1F4A3}");
        let (tokens, _) = code.tokenize_result();

        assert_eq!(
            tokens,
            vec![
                Err(Diagnostic::error(
                    &code.s1("€"),
                    "Found invalid latin-1 character '€'"
                )),
                Err(Diagnostic::error(
                    &code.s1("\u{1F4A3}"),
                    "Found invalid latin-1 character '\u{1F4A3}'"
                ))
            ]
        );
    }

    #[test]
    fn tokenize_integer_negative_exponent() {
        let code = Code::new("1e-1");
        let (tokens, _) = code.tokenize_result();

        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(
                &code.pos(),
                "Integer literals may not have negative exponent"
            ))]
        );
    }

    #[test]
    fn tokenize_real() {
        assert_eq!(
            kind_value_tokenize("0.1 -2_2.3_3 2.0e3 3.33E2 2.1e-2 4.4e+1 2.5E+3"),
            vec![
                (
                    AbstractLiteral,
                    Value::AbstractLiteral(ast::AbstractLiteral::Real(0.1))
                ),
                (Minus, Value::NoValue),
                (
                    AbstractLiteral,
                    Value::AbstractLiteral(ast::AbstractLiteral::Real(22.33))
                ),
                (
                    AbstractLiteral,
                    Value::AbstractLiteral(ast::AbstractLiteral::Real(2000.0))
                ),
                (
                    AbstractLiteral,
                    Value::AbstractLiteral(ast::AbstractLiteral::Real(333.0))
                ),
                (
                    AbstractLiteral,
                    Value::AbstractLiteral(ast::AbstractLiteral::Real(0.021))
                ),
                (
                    AbstractLiteral,
                    Value::AbstractLiteral(ast::AbstractLiteral::Real(44.0))
                ),
                (
                    AbstractLiteral,
                    Value::AbstractLiteral(ast::AbstractLiteral::Real(2500.0))
                )
            ]
        );
    }

    #[test]
    fn tokenize_real_many_fractional_digits() {
        assert_eq!(
            kind_value_tokenize("0.1000_0000_0000_0000_0000_0000_0000_0000"),
            vec![(
                AbstractLiteral,
                Value::AbstractLiteral(ast::AbstractLiteral::Real(1e-1))
            )]
        );
    }

    #[test]
    fn tokenize_real_many_integer_digits() {
        assert_eq!(
            kind_value_tokenize("1000_0000_0000_0000_0000_0000_0000_0000.0"),
            vec![(
                AbstractLiteral,
                Value::AbstractLiteral(ast::AbstractLiteral::Real(1e31))
            )]
        );
    }

    #[test]
    fn tokenize_real_truncates_precision() {
        assert_eq!(
            kind_value_tokenize("2.71828182845904523536"),
            vec![(
                AbstractLiteral,
                Value::AbstractLiteral(ast::AbstractLiteral::Real(2.7182818284590452))
            )]
        );
    }

    #[test]
    fn tokenize_string_literal() {
        let code = Code::new("\"string\"");
        let tokens = code.tokenize();
        assert_eq!(
            tokens,
            vec![Token {
                kind: StringLiteral,
                value: Value::String(Latin1String::from_utf8_unchecked("string")),
                pos: code.pos(),
                next_state: state_at_end(&code),
                comments: None,
            },]
        );
    }

    #[test]
    fn tokenize_string_literal_quote() {
        let code = Code::new("\"str\"\"ing\"");
        let tokens = code.tokenize();
        assert_eq!(
            tokens,
            vec![Token {
                kind: StringLiteral,
                value: Value::String(Latin1String::from_utf8_unchecked("str\"ing")),
                pos: code.pos(),
                next_state: state_at_end(&code),
                comments: None,
            },]
        );
    }

    #[test]
    fn tokenize_string_literal_quote_separated() {
        let code = Code::new("\"str\" \"ing\"");
        let tokens = code.tokenize();
        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: StringLiteral,
                    value: Value::String(Latin1String::from_utf8_unchecked("str")),
                    pos: code.s1("\"str\"").pos(),
                    next_state: state_at_end(&code.s1("\"str\" ")),
                    comments: None,
                },
                Token {
                    kind: StringLiteral,
                    value: Value::String(Latin1String::from_utf8_unchecked("ing")),
                    pos: code.s1("\"ing\"").pos(),
                    next_state: state_at_end(&code),
                    comments: None,
                },
            ]
        );
    }

    #[test]
    fn tokenize_string_literal_error_on_multiline() {
        // Multiline is illegal
        let code = Code::new("\"str\ning\"");
        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(&code.pos(), "Multi line string"))]
        );
    }

    #[test]
    fn tokenize_string_literal_error_on_early_eof() {
        // End of file
        let code = Code::new("\"string");
        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(
                &code.pos(),
                "Reached EOF before end quote"
            ))]
        );
    }

    // @TODO test incorrect value for base, ex: 2x"ff"
    // @TODO test incorrect value for length ex: 2x"1111"
    #[test]
    fn tokenize_bit_string_literal() {
        use BaseSpecifier::{B, D, O, SB, SO, SX, UB, UO, UX, X};

        // Test all base specifiers
        for &base in [B, O, X, D, SB, SO, SX, UB, UO, UX].iter() {
            let (base_spec, value, length) = match base {
                B => ("b", "10", 2),
                O => ("o", "76543210", 8 * 3),
                X => ("x", "fedcba987654321", 16 * 4),
                D => ("d", "9876543210", 34),
                SB => ("sb", "10", 2),
                SO => ("so", "76543210", 8 * 3),
                SX => ("sx", "fedcba987654321", 16 * 4),
                UB => ("ub", "10", 2),
                UO => ("uo", "76543210", 8 * 3),
                UX => ("ux", "fedcba987654321", 16 * 4),
            };

            // Test with upper and lower case base specifier
            for &upper_case in [true, false].iter() {
                // Test with and without length prefix
                for &use_length in [true, false].iter() {
                    let (length_str, length_opt) = if use_length {
                        (length.to_string(), Some(length))
                    } else {
                        ("".to_owned(), None)
                    };

                    let code = format!("{}{}\"{}\"", length_str, base_spec, value);

                    let code = if upper_case {
                        code.to_ascii_uppercase()
                    } else {
                        code
                    };

                    let value = if upper_case {
                        value.to_ascii_uppercase()
                    } else {
                        value.to_owned()
                    };

                    let code = Code::new(code.as_str());
                    let tokens = code.tokenize();
                    assert_eq!(
                        tokens,
                        vec![Token {
                            kind: BitString,
                            value: Value::BitString(ast::BitString {
                                length: length_opt,
                                base: base,
                                value: Latin1String::from_utf8_unchecked(value.as_str())
                            }),
                            pos: code.pos(),
                            next_state: state_at_end(&code),
                            comments: None,
                        },]
                    );
                }
            }
        }
    }

    #[test]
    fn tokenize_illegal_bit_string() {
        let code = Code::new("10x");
        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(
                &code.pos(),
                "Invalid bit string literal"
            ))]
        );

        let code = Code::new("10ux");
        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(
                &code.pos(),
                "Invalid bit string literal"
            ))]
        );
    }

    #[test]
    fn tokenize_based_integer() {
        assert_eq!(
            kind_value_tokenize("2#101#"),
            vec![(
                AbstractLiteral,
                Value::AbstractLiteral(ast::AbstractLiteral::Integer(5))
            ),]
        );
        assert_eq!(
            kind_value_tokenize("8#321#"),
            vec![(
                AbstractLiteral,
                Value::AbstractLiteral(ast::AbstractLiteral::Integer(3 * 8 * 8 + 2 * 8 + 1))
            ),]
        );
        assert_eq!(
            kind_value_tokenize("16#eEFfa#"),
            vec![(
                AbstractLiteral,
                Value::AbstractLiteral(ast::AbstractLiteral::Integer(0xeEffa))
            ),]
        );
    }

    #[test]
    fn tokenize_illegal_integer() {
        let code = Code::new("100k");

        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(
                &code.s1("k"),
                "Invalid integer character 'k'"
            ))]
        );
    }

    #[test]
    fn tokenize_illegal_based_integer() {
        // Base may only be 2-16
        let code = Code::new("1#0#");
        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(
                &code.s1("1"),
                "Base must be at least 2 and at most 16, got 1"
            ))]
        );
        let code = Code::new("17#f#");
        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(
                &code.s1("17"),
                "Base must be at least 2 and at most 16, got 17"
            ))]
        );
        // May not use digit larger than or equal base
        let code = Code::new("3#3#");
        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(
                &code.s("3", 2),
                "Illegal digit '3' for base 3"
            ))]
        );
        let code = Code::new("15#f#");
        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(
                &code.s1("f"),
                "Illegal digit 'f' for base 15"
            ))]
        );
    }

    #[test]
    fn tokenize_char_literal() {
        assert_eq!(
            kind_value_tokenize("'c'"),
            vec![(Character, Value::Character(b'c'))]
        );
    }

    #[test]
    fn tokenize_tick() {
        assert_eq!(kinds_tokenize("'"), vec![Tick]);
    }

    #[test]
    fn tokenize_plus() {
        assert_eq!(kinds_tokenize("+"), vec![Plus]);
    }

    #[test]
    fn tokenize_minus() {
        assert_eq!(kinds_tokenize("-"), vec![Minus]);
    }

    #[test]
    fn tokenize_semi_colon() {
        assert_eq!(kinds_tokenize(";"), vec![SemiColon]);
    }

    #[test]
    fn tokenize_colon() {
        assert_eq!(kinds_tokenize(":"), vec![Colon]);
    }

    #[test]
    fn tokenize_bar() {
        assert_eq!(kinds_tokenize("|"), vec![Bar]);
    }

    #[test]
    fn tokenize_dot() {
        assert_eq!(kinds_tokenize("."), vec![Dot]);
    }

    #[test]
    fn tokenize_concat() {
        assert_eq!(kinds_tokenize("&"), vec![Concat]);
    }

    #[test]
    fn tokenize_eq() {
        assert_eq!(kinds_tokenize("="), vec![EQ]);
    }

    #[test]
    fn tokenize_colon_eq() {
        assert_eq!(kinds_tokenize(":="), vec![ColonEq]);
    }

    #[test]
    fn tokenize_right_arrow() {
        assert_eq!(kinds_tokenize("=>"), vec![RightArrow]);
    }

    #[test]
    fn tokenize_cmp() {
        assert_eq!(kinds_tokenize("< <= > >="), vec![LT, LTE, GT, GTE]);
    }

    #[test]
    fn tokenize_box() {
        assert_eq!(kinds_tokenize("<>"), vec![BOX]);
    }

    #[test]
    fn tokenize_external_name() {
        assert_eq!(kinds_tokenize("<< >>"), vec![LtLt, GtGt]);
    }

    #[test]
    fn tokenize_questionmark_cmp() {
        assert_eq!(
            kinds_tokenize("? ?< ?<= ?> ?>= ??"),
            vec![Que, QueLT, QueLTE, QueGT, QueGTE, QueQue]
        );
    }

    #[test]
    fn tokenize_ne() {
        assert_eq!(kinds_tokenize("/="), vec![NE]);
    }

    #[test]
    fn tokenize_times() {
        assert_eq!(kinds_tokenize("*"), vec![Times]);
    }

    #[test]
    fn tokenize_pow() {
        assert_eq!(kinds_tokenize("**"), vec![Pow]);
    }

    #[test]
    fn tokenize_div() {
        assert_eq!(kinds_tokenize("/"), vec![Div]);
    }

    #[test]
    fn tokenize_comma() {
        assert_eq!(kinds_tokenize(","), vec![Comma]);
    }

    #[test]
    fn tokenize_pars() {
        assert_eq!(kinds_tokenize("()"), vec![LeftPar, RightPar]);
    }

    #[test]
    fn tokenize_squares() {
        assert_eq!(kinds_tokenize("[]"), vec![LeftSquare, RightSquare]);
    }

    #[test]
    fn tokenize_ignores_comments() {
        assert_eq!(
            kinds_tokenize(
                "
1
--comment
-2
"
            ),
            vec![AbstractLiteral, Minus, AbstractLiteral]
        );
    }

    #[test]
    fn tokenize_ignores_multi_line_comments() {
        assert_eq!(
            kinds_tokenize(
                "
1

/*
comment
*/

-2 /*
comment
*/

"
            ),
            vec![AbstractLiteral, Minus, AbstractLiteral]
        );
    }

    #[test]
    fn tokenize_ir1045() {
        // http://www.eda-stds.org/isac/IRs-VHDL-93/IR1045.txt
        assert_eq!(
            kinds_tokenize("string'('a')"),
            vec![Identifier, Tick, LeftPar, Character, RightPar]
        );
    }

    #[test]
    fn tokenize_too_large_integer() {
        let large_int = "100000000000000000000000000000000000000000";
        let code = Code::new(large_int);
        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(
                &code.pos(),
                "Integer too large for 64-bit unsigned"
            ))]
        );

        let large_int = "1e100";
        let code = Code::new(large_int);
        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(
                &code.pos(),
                "Integer too large for 64-bit unsigned"
            ))]
        );

        let exponent_str = ((i32::max_value() as i64) + 1).to_string();
        let large_int = format!("1e{}", exponent_str);
        let code = Code::new(&large_int);
        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(
                &code.s1(&exponent_str),
                "Exponent too large for 32-bits signed"
            ))]
        );

        let exponent_str = ((i32::min_value() as i64) - 1).to_string();
        let large_int = format!("1.0e{}", exponent_str);
        let code = Code::new(&large_int);
        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(
                &code.s1(&exponent_str),
                "Exponent too large for 32-bits signed"
            ))]
        );

        let large_int = ((u64::max_value() as i128) + 1).to_string();
        let code = Code::new(&large_int);
        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(
                &code.pos(),
                "Integer too large for 64-bit unsigned"
            ))]
        );

        let large_int = u64::max_value().to_string();
        let code = Code::new(&large_int);
        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Ok(Token {
                kind: AbstractLiteral,
                value: Value::AbstractLiteral(ast::AbstractLiteral::Integer(u64::max_value())),
                pos: code.pos(),
                next_state: state_at_end(&code),
                comments: None,
            })]
        );
    }

    #[test]
    fn tokenize_illegal() {
        let code = Code::new("begin!end");
        let (tokens, _) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![
                Ok(Token {
                    kind: Begin,
                    value: Value::NoValue,
                    pos: code.s1("begin").pos(),
                    next_state: state_at_end(&code.s1("begin")),
                    comments: None,
                }),
                Err(Diagnostic::error(&code.s1("!"), "Illegal token")),
                Ok(Token {
                    kind: End,
                    value: Value::NoValue,
                    pos: code.s1("end").pos(),
                    next_state: state_at_end(&code),
                    comments: None,
                }),
            ]
        );
    }

    #[test]
    fn test_match_token_kind() {
        let tokens = Code::new("entity").tokenize();
        let result = match_token_kind!(
            tokens[0],
            Identifier => Ok(1),
            Entity => Ok(2));
        assert_eq!(result, Ok(2));
    }

    #[test]
    fn test_match_token_kind_error() {
        let tokens = Code::new("+").tokenize();
        let result = match_token_kind!(
            tokens[0],
            Identifier => Ok(1),
            Entity => Ok(2));
        assert_eq!(result, Err(tokens[0].kinds_error(&[Identifier, Entity])));
    }

    #[test]
    fn test_match_token_kind_pattern_error() {
        let tokens = Code::new("+").tokenize();
        let result = match_token_kind!(
            tokens[0],
            Identifier | StringLiteral => Ok(1),
            Entity => Ok(2));
        assert_eq!(
            result,
            Err(tokens[0].kinds_error(&[Identifier, StringLiteral, Entity]))
        );
    }

    #[test]
    fn extract_final_comments() {
        let code = Code::new("--final");
        let (tokens, final_comments) = code.tokenize_result();
        assert_eq!(tokens, vec![]);
        assert_eq!(
            final_comments,
            vec![Comment {
                value: "final".to_string(),
                range: code.s1("--final").pos().range(),
                multi_line: false
            },]
        );
    }

    #[test]
    fn extract_incomplete_multi_line_comment() {
        let code = Code::new("/* final");
        let (tokens, final_comments) = code.tokenize_result();
        assert_eq!(
            tokens,
            vec![Err(Diagnostic::error(
                &code.s1("/* final"),
                "Incomplete multi-line comment"
            ))]
        );

        assert_eq!(final_comments, vec![]);
    }

    #[test]
    fn extract_comments() {
        let code = Code::new(
            "
 --this is a plus
+--this is still a plus
--- this is not a minus

    -- Neither is this
- -- this is a minus
-- a comment at the end of the file
       -- and another one
",
        );
        let (tokens, final_comments) = code.tokenize_result();
        let minus_pos = code.s1("- --").s1("-").pos();

        assert_eq!(
            tokens,
            vec![
                Ok(Token {
                    kind: Plus,
                    value: Value::NoValue,
                    pos: code.s1("+").pos(),
                    next_state: state_at_end(&code.s1("+--this is still a plus")),
                    comments: Some(Box::new(TokenComments {
                        leading: vec![Comment {
                            value: "this is a plus".to_string(),
                            range: code.s1("--this is a plus").pos().range(),
                            multi_line: false
                        },],
                        trailing: Some(Comment {
                            range: code.s1("--this is still a plus").pos().range(),
                            value: "this is still a plus".to_string(),
                            multi_line: false
                        }),
                    })),
                }),
                Ok(Token {
                    kind: Minus,
                    value: Value::NoValue,
                    pos: minus_pos,
                    next_state: state_at_end(&code.s1("-- this is a minus")),
                    comments: Some(Box::new(TokenComments {
                        leading: vec![
                            Comment {
                                value: "- this is not a minus".to_string(),
                                range: code.s1("--- this is not a minus").pos().range(),
                                multi_line: false
                            },
                            Comment {
                                value: " Neither is this".to_string(),
                                range: code.s1("-- Neither is this").pos().range(),
                                multi_line: false
                            },
                        ],
                        trailing: Some(Comment {
                            range: code.s1("-- this is a minus").pos().range(),
                            value: " this is a minus".to_string(),
                            multi_line: false
                        }),
                    })),
                }),
            ]
        );
        assert_eq!(
            final_comments,
            vec![
                Comment {
                    value: " a comment at the end of the file".to_string(),
                    range: code.s1("-- a comment at the end of the file").pos().range(),
                    multi_line: false
                },
                Comment {
                    value: " and another one".to_string(),
                    range: code.s1("-- and another one").pos().range(),
                    multi_line: false
                },
            ]
        );
    }

    #[test]
    fn extract_multi_line_comments() {
        let code = Code::new(
            "
/*foo
com*ment
bar*/

2

/*final*/
",
        );
        let (tokens, final_comments) = code.tokenize_result();

        assert_eq!(
            tokens,
            vec![Ok(Token {
                kind: AbstractLiteral,
                value: Value::AbstractLiteral(ast::AbstractLiteral::Integer(2)),
                pos: code.s1("2").pos(),
                next_state: state_at_end(&code.s1("2")),
                comments: Some(Box::new(TokenComments {
                    leading: vec![Comment {
                        value: "foo\ncom*ment\nbar".to_string(),
                        range: code.s1("/*foo\ncom*ment\nbar*/").pos().range(),
                        multi_line: true
                    },],
                    trailing: None,
                })),
            }),]
        );
        assert_eq!(
            final_comments,
            vec![Comment {
                value: "final".to_string(),
                range: code.s1("/*final*/").pos().range(),
                multi_line: true
            },]
        );
    }

    #[test]
    fn comments_allow_non_latin1() {
        let code = Code::new(
            "
/* € */
entity -- €
",
        );
        let (tokens, _) = code.tokenize_result();

        assert_eq!(
            tokens,
            vec![Ok(Token {
                kind: Entity,
                value: Value::NoValue,
                pos: code.s1("entity").pos(),
                next_state: state_at_end(&code.s1("entity -- €")),
                comments: Some(Box::new(TokenComments {
                    leading: vec![Comment {
                        value: " € ".to_string(),
                        range: code.s1("/* € */").pos().range(),
                        multi_line: true
                    },],
                    trailing: Some(Comment {
                        value: " €".to_string(),
                        range: code.s1("-- €").pos().range(),
                        multi_line: false
                    }),
                })),
            })]
        );
    }
}
