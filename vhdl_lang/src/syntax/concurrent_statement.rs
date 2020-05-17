// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2018, Olof Kraigher olof.kraigher@gmail.com

use super::common::error_on_end_identifier_mismatch;
use super::common::ParseResult;
use super::declarative_part::{is_declarative_part, parse_declarative_part};
use super::expression::parse_aggregate_leftpar_known;
use super::expression::{parse_choices, parse_expression};
use super::interface_declaration::{parse_generic_interface_list, parse_port_interface_list};
use super::names::{
    expression_to_ident, into_selected_name, parse_association_list, parse_name_initial_token,
    parse_selected_name,
};
use super::range::parse_discrete_range;
use super::sequential_statement::{
    parse_assert_statement_known_keyword, parse_labeled_sequential_statements, parse_selection,
    parse_signal_assignment_right_hand, parse_target,
};
use super::tokens::{Kind::*, Token, TokenStream};
use super::waveform::{parse_delay_mechanism, parse_waveform};
use crate::ast::*;
use crate::data::*;

/// LRM 11.2 Block statement
pub fn parse_block_statement(
    block_token: Token,
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<BlockStatement> {
    let token = stream.peek_expect()?;
    let guard_condition = {
        match token.kind {
            LeftPar => {
                stream.move_after(&token);
                let expr = parse_expression(stream)?;
                stream.pop_if_kind(RightPar)?;
                Some(expr)
            }
            _ => None,
        }
    };
    stream.pop_if_kind(Is)?;
    let header = parse_block_header(stream, diagnostics)?;
    let decl = parse_declarative_part(stream, diagnostics, true)?;
    let statements = parse_labeled_concurrent_statements(stream, diagnostics)?;
    stream.expect_kind(Block)?;
    // @TODO check name
    stream.pop_if_kind(Identifier)?;
    let semi_token = stream.expect_kind(SemiColon)?.into();
    Ok(BlockStatement {
        guard_condition,
        header,
        decl,
        statements,
        block_token: block_token.into(),
        semi_token,
    })
}

fn parse_block_header(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<BlockHeader> {
    let mut generic_clause = None;
    let mut generic_map = None;
    let mut port_clause = None;
    let mut port_map = None;

    loop {
        let token = stream.peek_expect()?;
        match token.kind {
            Generic => {
                stream.move_after(&token);
                if let Some(map_token) = stream.pop_if_kind(Map)? {
                    if port_clause.is_some() || port_map.is_some() {
                        diagnostics.push(Diagnostic::error(
                            map_token,
                            "Generic map must come before port clause and port map",
                        ));
                    } else if generic_clause.is_none() {
                        diagnostics.push(Diagnostic::error(
                            map_token,
                            "Generic map declared without preceeding generic clause",
                        ));
                    } else if generic_map.is_some() {
                        diagnostics.push(Diagnostic::error(map_token, "Duplicate generic map"));
                    }
                    let parsed_generic_map = Some(parse_association_list(stream)?);
                    stream.expect_kind(SemiColon)?;
                    if generic_map.is_none() {
                        generic_map = parsed_generic_map;
                    }
                } else {
                    if generic_map.is_some() {
                        diagnostics.push(Diagnostic::error(
                            token,
                            "Generic clause must come before generic map",
                        ));
                    } else if generic_clause.is_some() {
                        diagnostics.push(Diagnostic::error(token, "Duplicate generic clause"));
                    }
                    let parsed_generic_list = parse_generic_interface_list(stream, diagnostics)?;
                    stream.expect_kind(SemiColon)?;
                    if generic_clause.is_none() {
                        generic_clause = Some(parsed_generic_list);
                    }
                }
            }
            Port => {
                stream.move_after(&token);
                if let Some(map_token) = stream.pop_if_kind(Map)? {
                    if port_clause.is_none() {
                        diagnostics.push(Diagnostic::error(
                            map_token,
                            "Port map declared without preceeding port clause",
                        ));
                    } else if port_map.is_some() {
                        diagnostics.push(Diagnostic::error(map_token, "Duplicate port map"));
                    }
                    let parsed_port_map = Some(parse_association_list(stream)?);
                    stream.expect_kind(SemiColon)?;
                    if port_map.is_none() {
                        port_map = parsed_port_map;
                    }
                } else {
                    if port_map.is_some() {
                        diagnostics.push(Diagnostic::error(
                            token,
                            "Port clause declared after port map",
                        ));
                    } else if port_clause.is_some() {
                        diagnostics.push(Diagnostic::error(token, "Duplicate port clause"));
                    }
                    let parsed_port_list = parse_port_interface_list(stream, diagnostics)?;
                    stream.expect_kind(SemiColon)?;
                    if port_clause.is_none() {
                        port_clause = Some(parsed_port_list);
                    }
                }
            }
            _ => break,
        }
    }

    Ok(BlockHeader {
        generic_clause,
        generic_map,
        port_clause,
        port_map,
    })
}

/// LRM 11.3 Process statement
pub fn parse_process_statement(
    stream: &mut TokenStream,
    start_token: Token,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<ProcessStatement> {
    let postponed = start_token.kind == Postponed;
    let token = stream.peek_expect()?;
    let sensitivity_list = {
        match token.kind {
            LeftPar => {
                stream.move_after(&token);
                let token = stream.expect()?;

                if token.kind == All {
                    stream.expect_kind(RightPar)?;
                    Some(SensitivityList::All)
                } else {
                    let mut names = Vec::with_capacity(1);
                    let mut token = token;
                    loop {
                        match token.kind {
                            RightPar => {
                                if names.is_empty() {
                                    diagnostics.push(
                                        Diagnostic::error(token, "Processes with sensitivity lists must contain at least one element.")
                                    );
                                }
                                break Some(SensitivityList::Names(names));
                            }
                            Comma => {}
                            _ => {
                                names.push(parse_name_initial_token(stream, token)?);
                            }
                        };

                        token = stream.expect()?;
                    }
                }
            }
            _ => None,
        }
    };
    stream.pop_if_kind(Is)?;
    let decl = parse_declarative_part(stream, diagnostics, true)?;
    let (statements, end_token) = parse_labeled_sequential_statements(stream, diagnostics)?;
    try_token_kind!(end_token, End => {});
    if let Some(token) = stream.pop_if_kind(Postponed)? {
        if !postponed {
            diagnostics.push(Diagnostic::error(
                token,
                "'postponed' at the end of non-postponed process.",
            ));
        }
    }
    stream.expect_kind(Process)?;
    // @TODO check name
    stream.pop_if_kind(Identifier)?;
    let semi_token = stream.expect_kind(SemiColon)?.into();
    Ok(ProcessStatement {
        postponed,
        sensitivity_list,
        decl,
        statements,
        start_token: start_token.into(),
        semi_token,
    })
}

fn to_procedure_call(
    target: WithPos<Target>,
    postponed: bool,
) -> ParseResult<ConcurrentProcedureCall> {
    match target.item {
        Target::Name(Name::FunctionCall(call)) => Ok(ConcurrentProcedureCall {
            postponed,
            call: *call,
        }),
        Target::Name(name) => Ok(ConcurrentProcedureCall {
            postponed,
            call: FunctionCall {
                name: WithPos::from(name, target.pos),
                parameters: vec![],
            },
        }),
        Target::Aggregate(..) => Err(Diagnostic::error(
            target,
            "Expected procedure call, got aggregate",
        )),
    }
}

/// Assume target and <= is parsed already
fn parse_assignment_known_target(
    stream: &mut TokenStream,
    target: WithPos<Target>,
) -> ParseResult<ConcurrentStatement> {
    // @TODO postponed
    let postponed = false;
    // @TODO guarded
    let guarded = false;
    let delay_mechanism = parse_delay_mechanism(stream)?;
    Ok(ConcurrentStatement::Assignment(
        ConcurrentSignalAssignment {
            postponed,
            guarded,
            target,
            delay_mechanism,
            rhs: parse_signal_assignment_right_hand(stream)?,
        },
    ))
}

fn parse_assignment_or_procedure_call(
    stream: &mut TokenStream,
    token: &Token,
    target: WithPos<Target>,
) -> ParseResult<ConcurrentStatement> {
    match_token_kind!(
    token,
    LTE => {
        parse_assignment_known_target(stream, target)
    },
    SemiColon => {
        Ok(ConcurrentStatement::ProcedureCall(to_procedure_call(target, false)?))
    })
}

fn parse_selected_signal_assignment(
    stream: &mut TokenStream,
    postponed: bool,
) -> ParseResult<ConcurrentSignalAssignment> {
    let expression = parse_expression(stream)?;
    stream.expect_kind(Select)?;
    let target = parse_target(stream)?;
    stream.expect_kind(LTE)?;
    // @TODO guarded
    let guarded = false;
    let delay_mechanism = parse_delay_mechanism(stream)?;
    let rhs = AssignmentRightHand::Selected(parse_selection(stream, expression, parse_waveform)?);
    Ok(ConcurrentSignalAssignment {
        postponed,
        guarded,
        target,
        delay_mechanism,
        rhs,
    })
}

pub fn parse_concurrent_assert_statement(
    stream: &mut TokenStream,
    postponed: bool,
) -> ParseResult<ConcurrentAssertStatement> {
    Ok(ConcurrentAssertStatement {
        postponed,
        statement: parse_assert_statement_known_keyword(stream)?,
    })
}

#[allow(clippy::type_complexity)]
pub fn parse_generic_and_port_map(
    stream: &mut TokenStream,
) -> ParseResult<(
    Option<Vec<AssociationElement>>,
    Option<Vec<AssociationElement>>,
)> {
    let generic_map = {
        if stream.skip_if_kind(Generic)? {
            stream.expect_kind(Map)?;
            Some(parse_association_list(stream)?)
        } else {
            None
        }
    };
    let port_map = {
        if stream.skip_if_kind(Port)? {
            stream.expect_kind(Map)?;
            Some(parse_association_list(stream)?)
        } else {
            None
        }
    };

    Ok((generic_map, port_map))
}

pub fn parse_instantiation_statement(
    stream: &mut TokenStream,
    unit: InstantiatedUnit,
) -> ParseResult<InstantiationStatement> {
    let (generic_map, port_map) = parse_generic_and_port_map(stream)?;

    let inst = InstantiationStatement {
        unit,
        generic_map: generic_map.unwrap_or_else(Vec::new),
        port_map: port_map.unwrap_or_else(Vec::new),
    };
    stream.expect_kind(SemiColon)?;
    Ok(inst)
}

fn parse_optional_declarative_part(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<Option<Vec<Declaration>>> {
    if is_declarative_part(stream, true)? {
        Ok(Some(parse_declarative_part(stream, diagnostics, true)?))
    } else {
        Ok(None)
    }
}

fn parse_generate_body_end_token(
    stream: &mut TokenStream,
    alternative_label: Option<Ident>,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<(GenerateBody, Token)> {
    let decl = parse_optional_declarative_part(stream, diagnostics)?;
    let (statements, mut end_token) =
        parse_labeled_concurrent_statements_end_token(stream, diagnostics)?;

    // Potential inner end [ alternative_label ];
    if end_token.kind == End {
        let token = stream.peek_expect()?;
        try_token_kind!(
        token,
        Generate => {
            // Outer end
        },
        SemiColon => {
            // Inner end no label
            stream.move_after(&token);
            stream.expect_kind(SemiColon)?;
            end_token = stream.expect()?;
        },
        Identifier => {
            stream.move_after(&token);
            // Inner with identifier
            let end_ident = token.expect_ident()?;
            if let Some(ref ident) = alternative_label {
                diagnostics.push_some(error_on_end_identifier_mismatch(ident, &Some(end_ident)));
            };
            stream.expect_kind(SemiColon)?;
            end_token = stream.expect()?;
        });
    }

    let body = GenerateBody {
        alternative_label,
        decl,
        statements,
    };

    Ok((body, end_token))
}

fn parse_generate_body(
    stream: &mut TokenStream,
    alternative_label: Option<Ident>,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<GenerateBody> {
    let (body, end_token) = parse_generate_body_end_token(stream, alternative_label, diagnostics)?;
    end_token.expect_kind(End)?;
    Ok(body)
}

/// 11.8 Generate statements
fn parse_for_generate_statement(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<ForGenerateStatement> {
    let index_name = stream.expect_ident()?;
    stream.expect_kind(In)?;
    let discrete_range = parse_discrete_range(stream)?;
    stream.expect_kind(Generate)?;
    let body = parse_generate_body(stream, None, diagnostics)?;
    stream.expect_kind(Generate)?;
    // @TODO check identifier
    stream.pop_if_kind(Identifier)?;
    stream.expect_kind(SemiColon)?;

    Ok(ForGenerateStatement {
        index_name,
        discrete_range,
        body,
    })
}

/// 11.8 Generate statements
fn parse_if_generate_statement(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<IfGenerateStatement> {
    let mut conditionals = Vec::new();
    let else_branch;

    loop {
        let mut condition = parse_expression(stream)?;
        let mut alternative_label = None;
        if stream.skip_if_kind(Colon)? {
            alternative_label = Some(expression_to_ident(condition)?);
            condition = parse_expression(stream)?;
        }
        stream.expect_kind(Generate)?;
        let (body, end_token) =
            parse_generate_body_end_token(stream, alternative_label, diagnostics)?;

        let conditional = Conditional {
            condition,
            item: body,
        };

        conditionals.push(conditional);

        try_token_kind!(
            end_token,
            End => {
                else_branch = None;
                break;
            },
            Elsif => {
                continue;
            },
            Else => {
                let token = stream.expect()?;
                let alternative_label = try_token_kind!(
                    token,
                    Generate => {
                        None
                    },
                    Identifier => {
                        stream.expect_kind(Colon)?;
                        stream.expect_kind(Generate)?;
                        Some(token.expect_ident()?)
                    }
                );
                let body = parse_generate_body(stream, alternative_label, diagnostics)?;
                else_branch = Some(body);
                break;
            }
        );
    }

    stream.expect_kind(Generate)?;
    // @TODO check identifier
    stream.pop_if_kind(Identifier)?;
    stream.expect_kind(SemiColon)?;

    Ok(IfGenerateStatement {
        conditionals,
        else_item: else_branch,
    })
}

/// 11.8 Generate statements
fn parse_case_generate_statement(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<CaseGenerateStatement> {
    let expression = parse_expression(stream)?;
    stream.expect_kind(Generate)?;
    stream.expect_kind(When)?;

    let mut alternatives = Vec::with_capacity(2);
    loop {
        let alternative_label = {
            if stream.next_kinds_are(&[Identifier, Colon])? {
                let ident = stream.expect_ident()?;
                stream.expect_kind(Colon)?;
                Some(ident)
            } else {
                None
            }
        };
        let choices = parse_choices(stream)?;
        stream.expect_kind(RightArrow)?;
        let (body, end_token) =
            parse_generate_body_end_token(stream, alternative_label, diagnostics)?;
        alternatives.push(Alternative {
            choices,
            item: body,
        });

        try_token_kind!(
            end_token,
            End => break,
            When => continue
        );
    }

    stream.expect_kind(Generate)?;
    // @TODO check identifier
    stream.pop_if_kind(Identifier)?;
    stream.expect_kind(SemiColon)?;

    Ok(CaseGenerateStatement {
        expression,
        alternatives,
    })
}

pub fn parse_concurrent_statement(
    stream: &mut TokenStream,
    token: Token,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<ConcurrentStatement> {
    let statement = {
        try_token_kind!(
            token,
            Block => {
                ConcurrentStatement::Block(parse_block_statement(token, stream, diagnostics)?)
            },
            Process => {
                ConcurrentStatement::Process(parse_process_statement(stream, token, diagnostics)?)
            },
            Component => {
                let unit = InstantiatedUnit::Component(parse_selected_name(stream)?);
                ConcurrentStatement::Instance(parse_instantiation_statement(stream, unit)?)
            },
            Configuration => {
                let unit = InstantiatedUnit::Configuration(parse_selected_name(stream)?);
                ConcurrentStatement::Instance(parse_instantiation_statement(stream, unit)?)
            },
            Entity => {
                let name = parse_selected_name(stream)?;
                let arch = {
                    if stream.skip_if_kind(LeftPar)? {
                        let ident = stream.expect_ident()?;
                        stream.expect_kind(RightPar)?;
                        Some(ident)
                    } else {
                        None
                    }
                };
                let unit = InstantiatedUnit::Entity(name, arch);
                ConcurrentStatement::Instance(parse_instantiation_statement(stream, unit)?)
            },
            For => ConcurrentStatement::ForGenerate(parse_for_generate_statement(stream, diagnostics)?),
            If => ConcurrentStatement::IfGenerate(parse_if_generate_statement(stream, diagnostics)?),
            Case => ConcurrentStatement::CaseGenerate(parse_case_generate_statement(stream, diagnostics)?),
            Assert => ConcurrentStatement::Assert(parse_concurrent_assert_statement(stream, false)?),
            Postponed => {
                let next_token = stream.expect()?;
                match next_token.kind {
                    Process => ConcurrentStatement::Process(parse_process_statement(stream, token, diagnostics)?),
                    Assert => ConcurrentStatement::Assert(parse_concurrent_assert_statement(stream, true)?),
                    With => ConcurrentStatement::Assignment(parse_selected_signal_assignment(stream, true)?),
                    _ => {
                        let target = parse_name_initial_token(stream, next_token)?.map_into(Target::Name);
                        stream.expect_kind(SemiColon)?;
                        ConcurrentStatement::ProcedureCall(to_procedure_call(target, true)?)
                    }
                }
            },
            With => ConcurrentStatement::Assignment(parse_selected_signal_assignment(stream, false)?),
            Identifier => {
                let name = parse_name_initial_token(stream, token)?;
                let token = stream.peek_expect()?;
                match token.kind {
                    Generic|Port => {
                        let unit = InstantiatedUnit::Component(into_selected_name(name)?);
                        ConcurrentStatement::Instance(parse_instantiation_statement(stream, unit)?)
                    }
                    _ => {
                        stream.move_after(&token);
                        parse_assignment_or_procedure_call(stream, &token, name.map_into(Target::Name))?
                    }
                }
            },
            LtLt => {
                let name = parse_name_initial_token(stream, token)?;
                stream.expect_kind(LTE)?;
                parse_assignment_known_target(stream, name.map_into(Target::Name))?
            },
            LeftPar => {
                let target = parse_aggregate_leftpar_known(stream)?.map_into(Target::Aggregate);
                let token = stream.expect()?;
                parse_assignment_or_procedure_call(stream, &token, target)?
            }
        )
    };
    Ok(statement)
}

pub fn parse_labeled_concurrent_statements_end_token(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<(Vec<LabeledConcurrentStatement>, Token)> {
    let mut statements = Vec::new();
    loop {
        let token = stream.expect()?;
        match token.kind {
            End | Elsif | Else | When => {
                break Ok((statements, token));
            }
            _ => {
                statements.push(parse_labeled_concurrent_statement_initial_token(
                    stream,
                    token,
                    diagnostics,
                )?);
            }
        }
    }
}

pub fn parse_labeled_concurrent_statements(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<Vec<LabeledConcurrentStatement>> {
    let (statement, _) = parse_labeled_concurrent_statements_end_token(stream, diagnostics)?;
    Ok(statement)
}

pub fn parse_labeled_concurrent_statement_initial_token(
    stream: &mut TokenStream,
    token: Token,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<LabeledConcurrentStatement> {
    if token.kind == Identifier {
        let name = parse_name_initial_token(stream, token)?;
        let token = stream.expect()?;
        if token.kind == Colon {
            let label = Some(to_simple_name(name)?);
            let token = stream.expect()?;
            let statement = parse_concurrent_statement(stream, token, diagnostics)?;
            Ok(LabeledConcurrentStatement { label, statement })
        } else {
            let target = name.map_into(Target::Name);
            let statement = parse_assignment_or_procedure_call(stream, &token, target)?;
            Ok(LabeledConcurrentStatement {
                label: None,
                statement,
            })
        }
    } else {
        let statement = parse_concurrent_statement(stream, token, diagnostics)?;
        Ok(LabeledConcurrentStatement {
            label: None,
            statement,
        })
    }
}

#[cfg(test)]
pub fn parse_labeled_concurrent_statement(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<LabeledConcurrentStatement> {
    let token = stream.expect()?;
    parse_labeled_concurrent_statement_initial_token(stream, token, diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Alternative, AssertStatement, DelayMechanism, Selection};
    use crate::syntax::test::Code;

    #[test]
    fn test_concurrent_procedure() {
        let code = Code::new(
            "\
            foo(clk);
",
        );
        let call = ConcurrentProcedureCall {
            postponed: false,
            call: code.s1("foo(clk)").function_call(),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, None);
        assert_eq!(stmt.statement, ConcurrentStatement::ProcedureCall(call));
    }

    #[test]
    fn test_postponed_concurrent_procedure() {
        let code = Code::new(
            "\
            postponed foo(clk);
",
        );
        let call = ConcurrentProcedureCall {
            postponed: true,
            call: code.s1("foo(clk)").function_call(),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, None);
        assert_eq!(stmt.statement, ConcurrentStatement::ProcedureCall(call));
    }

    #[test]
    fn test_labeled_concurrent_procedure() {
        let code = Code::new(
            "\
            name: foo(clk);
",
        );
        let call = ConcurrentProcedureCall {
            postponed: false,
            call: code.s1("foo(clk)").function_call(),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("name").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::ProcedureCall(call));
    }

    #[test]
    fn test_concurrent_procedure_no_args() {
        let code = Code::new(
            "\
            foo;
",
        );
        let call = ConcurrentProcedureCall {
            postponed: false,
            call: code.s1("foo").function_call(),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, None);
        assert_eq!(stmt.statement, ConcurrentStatement::ProcedureCall(call));
    }

    #[test]
    fn test_block() {
        let code = Code::new(
            "\
name : block
  constant const : natural := 0;
begin
  name2: foo(clk);
end block;
",
        );
        let call = ConcurrentProcedureCall {
            postponed: false,
            call: code.s1("foo(clk)").function_call(),
        };

        let block = BlockStatement {
            guard_condition: None,
            header: BlockHeader {
                generic_clause: None,
                generic_map: None,
                port_clause: None,
                port_map: None,
            },
            decl: code.s1("constant const : natural := 0;").declarative_part(),
            statements: vec![LabeledConcurrentStatement {
                label: Some(code.s1("name2").ident()),
                statement: ConcurrentStatement::ProcedureCall(call),
            }],
            block_token: code.keyword_token(Block, 1),
            semi_token: code.keyword_token(SemiColon, -1),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("name").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::Block(block));
    }

    #[test]
    fn test_block_variant() {
        let code = Code::new(
            "\
name : block is
begin
end block name;
",
        );
        let block = BlockStatement {
            guard_condition: None,
            header: BlockHeader {
                generic_clause: None,
                generic_map: None,
                port_clause: None,
                port_map: None,
            },
            decl: vec![],
            statements: vec![],
            block_token: code.keyword_token(Block, 1),
            semi_token: code.keyword_token(SemiColon, -1),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("name").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::Block(block));
    }

    #[test]
    fn test_guarded_block() {
        let code = Code::new(
            "\
name : block (cond = true)
begin
end block;
",
        );
        let block = BlockStatement {
            guard_condition: Some(code.s1("cond = true").expr()),
            header: BlockHeader {
                generic_clause: None,
                generic_map: None,
                port_clause: None,
                port_map: None,
            },
            decl: vec![],
            statements: vec![],
            block_token: code.keyword_token(Block, 1),
            semi_token: code.keyword_token(SemiColon, -1),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("name").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::Block(block));
    }

    #[test]
    fn test_guarded_block_variant() {
        let code = Code::new(
            "\
name : block (cond = true) is
begin
end block;
",
        );
        let block = BlockStatement {
            guard_condition: Some(code.s1("cond = true").expr()),
            header: BlockHeader {
                generic_clause: None,
                generic_map: None,
                port_clause: None,
                port_map: None,
            },
            decl: vec![],
            statements: vec![],
            block_token: code.keyword_token(Block, 1),
            semi_token: code.keyword_token(SemiColon, -1),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("name").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::Block(block));
    }

    #[test]
    fn test_block_header() {
        let code = Code::new(
            "\
name: block is
  generic(gen: integer := 1);
  generic map(gen => 1);
  port(prt: integer := 1);
  port map(prt => 2);
begin
end block;
",
        );
        let block = BlockStatement {
            guard_condition: None,
            header: BlockHeader {
                generic_clause: Some(vec![code.s1("gen: integer := 1").generic()]),
                generic_map: Some(code.s1("(gen => 1)").association_list()),
                port_clause: Some(vec![code.s1("prt: integer := 1").port()]),
                port_map: Some(code.s1("(prt => 2)").association_list()),
            },
            decl: vec![],
            statements: vec![],
            block_token: code.keyword_token(Block, 1),
            semi_token: code.keyword_token(SemiColon, -1),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("name").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::Block(block));
    }

    #[test]
    fn test_process_statement() {
        let code = Code::new(
            "\
process
begin
end process;
",
        );
        let process = ProcessStatement {
            postponed: false,
            sensitivity_list: None,
            decl: vec![],
            statements: vec![],
            start_token: code.keyword_token(Process, 1),
            semi_token: code.keyword_token(SemiColon, -1),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, None);
        assert_eq!(stmt.statement, ConcurrentStatement::Process(process));
    }

    #[test]
    fn test_process_statement_variant() {
        let code = Code::new(
            "\
name : process is
begin
end process name;
",
        );
        let process = ProcessStatement {
            postponed: false,
            sensitivity_list: None,
            decl: vec![],
            statements: vec![],
            start_token: code.keyword_token(Process, 1),
            semi_token: code.keyword_token(SemiColon, -1),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("name").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::Process(process));
    }

    #[test]
    fn test_postponed_process_statement() {
        let code = Code::new(
            "\
postponed process
begin
end process;
",
        );
        let process = ProcessStatement {
            postponed: true,
            sensitivity_list: None,
            decl: vec![],
            statements: vec![],
            start_token: code.keyword_token(Postponed, 1),
            semi_token: code.keyword_token(SemiColon, -1),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, None);
        assert_eq!(stmt.statement, ConcurrentStatement::Process(process));
    }

    #[test]
    fn test_postponed_process_statement_end_postponed() {
        let code = Code::new(
            "\
postponed process
begin
end postponed process;
",
        );
        let process = ProcessStatement {
            postponed: true,
            sensitivity_list: None,
            decl: vec![],
            statements: vec![],
            start_token: code.keyword_token(Postponed, 1),
            semi_token: code.keyword_token(SemiColon, -1),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, None);
        assert_eq!(stmt.statement, ConcurrentStatement::Process(process));
    }

    #[test]
    fn test_process_statement_end_postponed() {
        let code = Code::new(
            "\
process is
begin
end postponed process;
",
        );
        let (stmt, diagnostics) = code.with_stream_diagnostics(parse_labeled_concurrent_statement);
        let process = ProcessStatement {
            postponed: false,
            sensitivity_list: None,
            decl: Vec::new(),
            statements: Vec::new(),
            start_token: code.keyword_token(Process, 1),
            semi_token: code.keyword_token(SemiColon, -1),
        };
        assert_eq!(
            diagnostics,
            vec![Diagnostic::error(
                code.s1("postponed"),
                "'postponed' at the end of non-postponed process."
            )]
        );
        assert_eq!(stmt.statement, ConcurrentStatement::Process(process));
    }

    #[test]
    fn test_process_statement_sensitivity() {
        let code = Code::new(
            "\
process (clk, vec(1)) is
begin
end process;
",
        );
        let process = ProcessStatement {
            postponed: false,
            sensitivity_list: Some(SensitivityList::Names(vec![
                code.s1("clk").name(),
                code.s1("vec(1)").name(),
            ])),
            decl: vec![],
            statements: vec![],
            start_token: code.keyword_token(Process, 1),
            semi_token: code.keyword_token(SemiColon, -1),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, None);
        assert_eq!(stmt.statement, ConcurrentStatement::Process(process));
    }

    #[test]
    fn test_process_empty_sensitivity() {
        let code = Code::new(
            "\
process () is
begin
end process;
",
        );
        let (stmt, diagnostics) = code.with_stream_diagnostics(parse_labeled_concurrent_statement);
        let process = ProcessStatement {
            postponed: false,
            sensitivity_list: Some(SensitivityList::Names(Vec::new())),
            decl: Vec::new(),
            statements: Vec::new(),
            start_token: code.keyword_token(Process, 1),
            semi_token: code.keyword_token(SemiColon, -1),
        };
        assert_eq!(
            diagnostics,
            vec![Diagnostic::error(
                code.s1(")"),
                "Processes with sensitivity lists must contain at least one element."
            )]
        );
        assert_eq!(stmt.statement, ConcurrentStatement::Process(process));
    }

    #[test]
    fn test_process_statement_full() {
        let code = Code::new(
            "\
process (all) is
  variable foo : boolean;
begin
  foo <= true;
  wait;
end process;
",
        );
        let process = ProcessStatement {
            postponed: false,
            sensitivity_list: Some(SensitivityList::All),
            decl: code.s1("variable foo : boolean;").declarative_part(),
            statements: vec![
                code.s1("foo <= true;").sequential_statement(),
                code.s1("wait;").sequential_statement(),
            ],
            start_token: code.keyword_token(Process, 1),
            semi_token: code.keyword_token(SemiColon, -1),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, None);
        assert_eq!(stmt.statement, ConcurrentStatement::Process(process));
    }

    #[test]
    fn test_concurrent_assert() {
        let code = Code::new(
            "\
assert cond = true;
",
        );
        let assert = ConcurrentAssertStatement {
            postponed: false,
            statement: AssertStatement {
                condition: code.s1("cond = true").expr(),
                report: None,
                severity: None,
            },
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, None);
        assert_eq!(stmt.statement, ConcurrentStatement::Assert(assert));
    }

    #[test]
    fn test_postponed_concurrent_assert() {
        let code = Code::new(
            "\
postponed assert cond = true;
",
        );
        let assert = ConcurrentAssertStatement {
            postponed: true,
            statement: AssertStatement {
                condition: code.s1("cond = true").expr(),
                report: None,
                severity: None,
            },
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, None);
        assert_eq!(stmt.statement, ConcurrentStatement::Assert(assert));
    }

    #[test]
    fn test_concurrent_signal_assignment() {
        let code = Code::new(
            "\
foo <= bar(2 to 3);
",
        );
        let assign = ConcurrentSignalAssignment {
            postponed: false,
            guarded: false,
            target: code.s1("foo").name().map_into(Target::Name),
            delay_mechanism: None,
            rhs: AssignmentRightHand::Simple(code.s1("bar(2 to 3)").waveform()),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, None);
        assert_eq!(stmt.statement, ConcurrentStatement::Assignment(assign));
    }

    #[test]
    fn test_concurrent_signal_assignment_external_name() {
        let code = Code::new(
            "\
<< signal dut.foo : std_logic >> <= bar(2 to 3);
",
        );
        let assign = ConcurrentSignalAssignment {
            postponed: false,
            guarded: false,
            target: code
                .s1("<< signal dut.foo : std_logic >>")
                .name()
                .map_into(Target::Name),
            delay_mechanism: None,
            rhs: AssignmentRightHand::Simple(code.s1("bar(2 to 3)").waveform()),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, None);
        assert_eq!(stmt.statement, ConcurrentStatement::Assignment(assign));
    }

    #[test]
    fn parse_selected_signal_assignment() {
        let code = Code::new(
            "\
with x(0) + 1 select
   foo(0) <= transport bar(1,2) after 2 ns when 0|1;",
        );

        let selection = Selection {
            expression: code.s1("x(0) + 1").expr(),
            alternatives: vec![Alternative {
                choices: code.s1("0|1").choices(),
                item: code.s1("bar(1,2) after 2 ns").waveform(),
            }],
        };

        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, None);
        assert_eq!(
            stmt.statement,
            ConcurrentStatement::Assignment(ConcurrentSignalAssignment {
                postponed: false,
                guarded: false,
                target: code.s1("foo(0)").name().map_into(Target::Name),
                delay_mechanism: Some(DelayMechanism::Transport),
                rhs: AssignmentRightHand::Selected(selection)
            })
        );
    }

    #[test]
    fn test_component_instantiation() {
        let code = Code::new(
            "\
inst: component lib.foo.bar;
",
        );

        let inst = InstantiationStatement {
            unit: InstantiatedUnit::Component(code.s1("lib.foo.bar").selected_name()),
            generic_map: vec![],
            port_map: vec![],
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("inst").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::Instance(inst));
    }

    #[test]
    fn test_configuration_instantiation() {
        let code = Code::new(
            "\
inst: configuration lib.foo.bar;
",
        );

        let inst = InstantiationStatement {
            unit: InstantiatedUnit::Configuration(code.s1("lib.foo.bar").selected_name()),
            generic_map: vec![],
            port_map: vec![],
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("inst").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::Instance(inst));
    }

    #[test]
    fn test_entity_instantiation() {
        let code = Code::new(
            "\
inst: entity lib.foo.bar;
",
        );

        let inst = InstantiationStatement {
            unit: InstantiatedUnit::Entity(code.s1("lib.foo.bar").selected_name(), None),
            generic_map: vec![],
            port_map: vec![],
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("inst").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::Instance(inst));
    }

    #[test]
    fn test_entity_architecture_instantiation() {
        let code = Code::new(
            "\
inst: entity lib.foo.bar(arch);
",
        );

        let inst = InstantiationStatement {
            unit: InstantiatedUnit::Entity(
                code.s1("lib.foo.bar").selected_name(),
                Some(code.s1("arch").ident()),
            ),
            generic_map: vec![],
            port_map: vec![],
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("inst").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::Instance(inst));
    }

    #[test]
    fn test_component_aspect_maps() {
        let code = Code::new(
            "\
inst: component lib.foo.bar
  generic map (
   const => 1
  )
  port map (
   clk => clk_foo
  );
",
        );

        let inst = InstantiationStatement {
            unit: InstantiatedUnit::Component(code.s1("lib.foo.bar").selected_name()),
            generic_map: code
                .s1("(
   const => 1
  )")
                .association_list(),
            port_map: code
                .s1("(
   clk => clk_foo
  )")
                .association_list(),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("inst").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::Instance(inst));
    }

    #[test]
    fn test_component_no_keyword_port_aspect_map() {
        let code = Code::new(
            "\
inst: lib.foo.bar
  port map (
   clk => clk_foo
  );
",
        );

        let inst = InstantiationStatement {
            unit: InstantiatedUnit::Component(code.s1("lib.foo.bar").selected_name()),
            generic_map: vec![],
            port_map: code
                .s1("(
   clk => clk_foo
  )")
                .association_list(),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("inst").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::Instance(inst));
    }

    #[test]
    fn test_component_no_keyword_generic_aspect_map() {
        let code = Code::new(
            "\
            inst: lib.foo.bar
  generic map (
   const => 1
  );
",
        );

        let inst = InstantiationStatement {
            unit: InstantiatedUnit::Component(code.s1("lib.foo.bar").selected_name()),
            generic_map: code
                .s1("(
   const => 1
  )")
                .association_list(),
            port_map: vec![],
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("inst").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::Instance(inst));
    }

    #[test]
    fn test_for_generate_empty() {
        let code = Code::new(
            "\
gen: for idx in 0 to 1 generate
end generate;
",
        );
        let gen = ForGenerateStatement {
            index_name: code.s1("idx").ident(),
            discrete_range: code.s1("0 to 1").discrete_range(),
            body: GenerateBody {
                alternative_label: None,
                decl: None,
                statements: vec![],
            },
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("gen").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::ForGenerate(gen));
    }

    #[test]
    fn test_for_generate() {
        let code = Code::new(
            "\
gen: for idx in 0 to 1 generate
  foo <= bar;
end generate;
",
        );
        let gen = ForGenerateStatement {
            index_name: code.s1("idx").ident(),
            discrete_range: code.s1("0 to 1").discrete_range(),
            body: GenerateBody {
                alternative_label: None,
                decl: None,
                statements: vec![code.s1("foo <= bar;").concurrent_statement()],
            },
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("gen").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::ForGenerate(gen));
    }

    #[test]
    fn test_for_generate_empty_declarations() {
        let code = Code::new(
            "\
gen: for idx in 0 to 1 generate
begin
  foo <= bar;
end generate;
",
        );
        let gen = ForGenerateStatement {
            index_name: code.s1("idx").ident(),
            discrete_range: code.s1("0 to 1").discrete_range(),
            body: GenerateBody {
                alternative_label: None,
                decl: Some(vec![]),
                statements: vec![code.s1("foo <= bar;").concurrent_statement()],
            },
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("gen").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::ForGenerate(gen));
    }

    #[test]
    fn test_for_generate_declarations() {
        let code = Code::new(
            "\
gen: for idx in 0 to 1 generate
  signal foo : natural;
begin
  foo <= bar;
end generate;
",
        );
        let gen = ForGenerateStatement {
            index_name: code.s1("idx").ident(),
            discrete_range: code.s1("0 to 1").discrete_range(),
            body: GenerateBody {
                alternative_label: None,
                decl: Some(code.s1("signal foo : natural;").declarative_part()),
                statements: vec![code.s1("foo <= bar;").concurrent_statement()],
            },
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("gen").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::ForGenerate(gen));
    }

    #[test]
    fn test_if_generate_empty() {
        let code = Code::new(
            "\
gen: if cond = true generate
end generate;
",
        );
        let gen = IfGenerateStatement {
            conditionals: vec![Conditional {
                condition: code.s1("cond = true").expr(),
                item: GenerateBody {
                    alternative_label: None,
                    decl: None,
                    statements: vec![],
                },
            }],
            else_item: None,
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("gen").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::IfGenerate(gen));
    }

    #[test]
    fn test_if_generate_declarative_region() {
        let code = Code::new(
            "\
gen: if cond = true generate
begin
end generate;
",
        );
        let gen = IfGenerateStatement {
            conditionals: vec![Conditional {
                condition: code.s1("cond = true").expr(),
                item: GenerateBody {
                    alternative_label: None,
                    decl: Some(vec![]),
                    statements: vec![],
                },
            }],
            else_item: None,
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("gen").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::IfGenerate(gen));
    }

    #[test]
    fn test_if_elseif_else_generate_empty() {
        let code = Code::new(
            "\
gen: if cond = true generate
elsif cond2 = true generate
else generate
end generate;
",
        );
        let gen = IfGenerateStatement {
            conditionals: vec![
                Conditional {
                    condition: code.s1("cond = true").expr(),
                    item: GenerateBody {
                        alternative_label: None,
                        decl: None,
                        statements: vec![],
                    },
                },
                Conditional {
                    condition: code.s1("cond2 = true").expr(),
                    item: GenerateBody {
                        alternative_label: None,
                        decl: None,
                        statements: vec![],
                    },
                },
            ],
            else_item: Some(GenerateBody {
                alternative_label: None,
                decl: None,
                statements: vec![],
            }),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("gen").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::IfGenerate(gen));
    }
    #[test]
    fn test_if_elseif_else_generate() {
        let code = Code::new(
            "\
gen: if cond = true generate
  variable v1 : boolean;
begin
  foo1(clk);
elsif cond2 = true generate
  variable v2 : boolean;
begin
  foo2(clk);
else generate
  variable v3 : boolean;
begin
  foo3(clk);
end generate;
",
        );
        let gen = IfGenerateStatement {
            conditionals: vec![
                Conditional {
                    condition: code.s1("cond = true").expr(),
                    item: GenerateBody {
                        alternative_label: None,
                        decl: Some(code.s1("variable v1 : boolean;").declarative_part()),
                        statements: vec![code.s1("foo1(clk);").concurrent_statement()],
                    },
                },
                Conditional {
                    condition: code.s1("cond2 = true").expr(),
                    item: GenerateBody {
                        alternative_label: None,
                        decl: Some(code.s1("variable v2 : boolean;").declarative_part()),
                        statements: vec![code.s1("foo2(clk);").concurrent_statement()],
                    },
                },
            ],
            else_item: Some(GenerateBody {
                alternative_label: None,
                decl: Some(code.s1("variable v3 : boolean;").declarative_part()),
                statements: vec![code.s1("foo3(clk);").concurrent_statement()],
            }),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("gen").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::IfGenerate(gen));
    }

    #[test]
    fn test_if_elseif_else_generate_alternative_label() {
        let code = Code::new(
            "\
gen: if alt1: cond = true generate
elsif alt2: cond2 = true generate
else alt3: generate
end alt4;
end generate;
",
        );
        let gen = IfGenerateStatement {
            conditionals: vec![
                Conditional {
                    condition: code.s1("cond = true").expr(),
                    item: GenerateBody {
                        alternative_label: Some(code.s1("alt1").ident()),
                        decl: None,
                        statements: vec![],
                    },
                },
                Conditional {
                    condition: code.s1("cond2 = true").expr(),
                    item: GenerateBody {
                        alternative_label: Some(code.s1("alt2").ident()),
                        decl: None,
                        statements: vec![],
                    },
                },
            ],
            else_item: Some(GenerateBody {
                alternative_label: Some(code.s1("alt3").ident()),
                decl: None,
                statements: vec![],
            }),
        };
        let (stmt, diagnostics) = code.with_stream_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("gen").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::IfGenerate(gen));
        assert_eq!(
            diagnostics,
            vec![Diagnostic::error(
                code.s1("alt4"),
                "End identifier mismatch, expected alt3"
            )]
        );
    }
    #[test]
    fn test_if_elseif_else_generate_inner_end() {
        let code = Code::new(
            "\
gen: if alt1: cond = true generate
end alt1;
elsif alt2: cond2 = true generate
end alt2;
else alt3: generate
end alt3;
end generate;
",
        );
        let gen = IfGenerateStatement {
            conditionals: vec![
                Conditional {
                    condition: code.s1("cond = true").expr(),
                    item: GenerateBody {
                        alternative_label: Some(code.s1("alt1").ident()),
                        decl: None,
                        statements: vec![],
                    },
                },
                Conditional {
                    condition: code.s1("cond2 = true").expr(),
                    item: GenerateBody {
                        alternative_label: Some(code.s1("alt2").ident()),
                        decl: None,
                        statements: vec![],
                    },
                },
            ],
            else_item: Some(GenerateBody {
                alternative_label: Some(code.s1("alt3").ident()),
                decl: None,
                statements: vec![],
            }),
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("gen").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::IfGenerate(gen));
    }

    #[test]
    fn test_case_generate() {
        let code = Code::new(
            "\
gen: case expr(0) + 2 generate
  when 1 | 2 =>
    sig <= value;
  when others =>
    foo(clk);
end generate;
",
        );
        let gen = CaseGenerateStatement {
            expression: code.s1("expr(0) + 2").expr(),
            alternatives: vec![
                Alternative {
                    choices: code.s1("1 | 2").choices(),
                    item: GenerateBody {
                        alternative_label: None,
                        decl: None,
                        statements: vec![code.s1("sig <= value;").concurrent_statement()],
                    },
                },
                Alternative {
                    choices: code.s1("others").choices(),
                    item: GenerateBody {
                        alternative_label: None,
                        decl: None,
                        statements: vec![code.s1("foo(clk);").concurrent_statement()],
                    },
                },
            ],
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("gen").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::CaseGenerate(gen));
    }

    #[test]
    fn test_case_alternative_label() {
        let code = Code::new(
            "\
gen: case expr(0) + 2 generate
  when alt1: 1 | 2 =>
    sig <= value;
  when alt2: others =>
    foo(clk);
end generate;
",
        );
        let gen = CaseGenerateStatement {
            expression: code.s1("expr(0) + 2").expr(),
            alternatives: vec![
                Alternative {
                    choices: code.s1("1 | 2").choices(),
                    item: GenerateBody {
                        alternative_label: Some(code.s1("alt1").ident()),
                        decl: None,
                        statements: vec![code.s1("sig <= value;").concurrent_statement()],
                    },
                },
                Alternative {
                    choices: code.s1("others").choices(),
                    item: GenerateBody {
                        alternative_label: Some(code.s1("alt2").ident()),
                        decl: None,
                        statements: vec![code.s1("foo(clk);").concurrent_statement()],
                    },
                },
            ],
        };
        let stmt = code.with_stream_no_diagnostics(parse_labeled_concurrent_statement);
        assert_eq!(stmt.label, Some(code.s1("gen").ident()));
        assert_eq!(stmt.statement, ConcurrentStatement::CaseGenerate(gen));
    }
}
