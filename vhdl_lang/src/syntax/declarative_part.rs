// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2020, Olof Kraigher olof.kraigher@gmail.com

use super::alias_declaration::parse_alias_declaration;
use super::attributes::parse_attribute;
use super::common::ParseResult;
use super::component_declaration::parse_component_declaration;
use super::configuration::parse_configuration_specification;
use super::context::parse_use_clause;
use super::names::{parse_association_list, parse_selected_name};
use super::object_declaration::{parse_file_declaration, parse_object_declaration};
use super::subprogram::parse_subprogram;
use super::tokens::{Kind::*, *};
use super::type_declaration::parse_type_declaration;
use crate::ast::{ContextClause, Declaration, PackageInstantiation};
use crate::data::DiagnosticHandler;

pub fn parse_package_instantiation(stream: &mut TokenStream) -> ParseResult<PackageInstantiation> {
    let package_token = stream.expect_kind(Package)?.into();
    let ident = stream.expect_ident()?;
    stream.expect_kind(Is)?;
    stream.expect_kind(New)?;
    let package_name = parse_selected_name(stream)?;

    let token = stream.expect()?;
    let (generic_map, semi_token) = try_token_kind!(
        token,
        Generic => {
            stream.expect_kind(Map)?;
            let association_list = parse_association_list(stream)?;
            let semi_token = stream.expect_kind(SemiColon)?;
            (Some(association_list), semi_token.into())
        },
        SemiColon => {
            (None, token.into())
        }
    );
    Ok(PackageInstantiation {
        context_clause: ContextClause::default(),
        ident,
        package_name,
        generic_map,
        package_token,
        semi_token,
    })
}

pub fn is_declarative_part(stream: &mut TokenStream, begin_is_end: bool) -> ParseResult<bool> {
    Ok(check_declarative_part(&stream.peek_expect()?, !begin_is_end, begin_is_end).is_ok())
}

fn check_declarative_part(token: &Token, may_end: bool, may_begin: bool) -> ParseResult<()> {
    match token.kind {
        Use | Type | Subtype | Shared | Constant | Signal | Variable | File | Component
        | Attribute | Alias | Impure | Function | Procedure | Package | For => Ok(()),
        Begin if may_begin => Ok(()),
        End if may_end => Ok(()),
        _ => {
            let decl_kinds = [
                Use, Type, Subtype, Shared, Constant, Signal, Variable, File, Component, Attribute,
                Alias, Impure, Function, Procedure, Package, For,
            ];

            Err(token.kinds_error(&decl_kinds))
        }
    }
}

pub fn parse_declarative_part(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
    begin_is_end: bool,
) -> ParseResult<Vec<Declaration>> {
    let end_token = if begin_is_end { Begin } else { End };
    let decl = parse_declarative_part_leave_end_token(stream, diagnostics)?;

    stream.expect_kind(end_token).log(diagnostics);
    Ok(decl)
}

pub fn parse_declarative_part_end_token(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
    begin_is_end: bool,
) -> ParseResult<(Vec<Declaration>, Token)> {
    let expected_end_token = if begin_is_end { Begin } else { End };
    let decl = parse_declarative_part_leave_end_token(stream, diagnostics)?;
    let end_token = stream.peek_expect()?;
    stream.expect_kind(expected_end_token).log(diagnostics);
    Ok((decl, end_token))
}

pub fn parse_declarative_part_leave_end_token(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<Vec<Declaration>> {
    let mut declarations: Vec<Declaration> = Vec::new();

    fn is_recover_token(kind: Kind) -> bool {
        match kind {
            Type | Subtype | Component | Impure | Function | Procedure | Package | For | File
            | Shared | Constant | Signal | Variable | Attribute | Use | Alias => true,
            _ => false,
        }
    };

    while let Some(token) = stream.peek()? {
        match token.kind {
            Begin | End => break,
            Type | Subtype | Component | Impure | Function | Procedure | Package | For => {
                let decl = match token.kind {
                    Type | Subtype => {
                        parse_type_declaration(stream, diagnostics).map(Declaration::Type)?
                    }
                    Component => parse_component_declaration(stream, diagnostics)
                        .map(Declaration::Component)?,
                    Impure | Function | Procedure => parse_subprogram(stream, diagnostics)?,
                    Package => parse_package_instantiation(stream).map(Declaration::Package)?,
                    For => {
                        parse_configuration_specification(stream).map(Declaration::Configuration)?
                    }
                    _ => unreachable!(),
                };
                declarations.push(decl);
            }

            File | Shared | Constant | Signal | Variable | Attribute => {
                let decls: ParseResult<Vec<Declaration>> = match token.kind {
                    File => parse_file_declaration(stream)
                        .map(|decls| decls.into_iter().map(Declaration::File).collect()),
                    Shared | Constant | Signal | Variable => parse_object_declaration(stream)
                        .map(|decls| decls.into_iter().map(Declaration::Object).collect()),
                    Attribute => parse_attribute(stream)
                        .map(|decls| decls.into_iter().map(Declaration::Attribute).collect()),
                    _ => unreachable!(),
                };
                match decls.or_recover_until(stream, diagnostics, is_recover_token) {
                    Ok(ref mut decls) => declarations.append(decls),
                    Err(err) => {
                        diagnostics.push(err);
                        continue;
                    }
                }
            }

            Use | Alias => {
                let decl: ParseResult<Declaration> = match token.kind {
                    Use => parse_use_clause(stream).map(Declaration::Use),
                    Alias => parse_alias_declaration(stream).map(Declaration::Alias),
                    _ => unreachable!(),
                };
                match decl.or_recover_until(stream, diagnostics, is_recover_token) {
                    Ok(decl) => declarations.push(decl),
                    Err(err) => {
                        diagnostics.push(err);
                        continue;
                    }
                }
            }

            _ => {
                diagnostics.push(token.kinds_error(&[
                    Type, Subtype, Component, Impure, Function, Procedure, Package, For, File,
                    Shared, Constant, Signal, Variable, Attribute, Use, Alias,
                ]));
                stream.skip_until(is_recover_token)?;
                continue;
            }
        }
    }

    Ok(declarations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{ObjectClass, ObjectDeclaration};
    use crate::data::Diagnostic;
    use crate::syntax::test::Code;

    #[test]
    fn package_instantiation() {
        let code = Code::new(
            "\
package ident is new lib.foo.bar;
",
        );
        assert_eq!(
            code.with_stream(parse_package_instantiation),
            PackageInstantiation {
                context_clause: ContextClause::default(),
                ident: code.s1("ident").ident(),
                package_name: code.s1("lib.foo.bar").selected_name(),
                generic_map: None,
                package_token: code.keyword_token(Package, 1),
                semi_token: code.keyword_token(SemiColon, -1),
            }
        );
    }

    #[test]
    fn package_instantiation_generic_map() {
        let code = Code::new(
            "\
package ident is new lib.foo.bar
  generic map (
    foo => bar
  );
",
        );
        assert_eq!(
            code.with_stream(parse_package_instantiation),
            PackageInstantiation {
                context_clause: ContextClause::default(),
                ident: code.s1("ident").ident(),
                package_name: code.s1("lib.foo.bar").selected_name(),
                generic_map: Some(
                    code.s1("(
    foo => bar
  )")
                        .association_list()
                ),
                package_token: code.keyword_token(Package, 1),
                semi_token: code.keyword_token(SemiColon, -1),
            }
        );
    }

    #[test]
    fn parse_declarative_part_recover() {
        let code = Code::new(
            "\
var invalid: broken;
constant x: natural := 5;
",
        );
        let (decls, msgs) =
            code.with_partial_stream_diagnostics(parse_declarative_part_leave_end_token);
        assert_eq!(
            decls,
            Ok(vec![Declaration::Object(ObjectDeclaration {
                class: ObjectClass::Constant,
                ident: code.s1("x").ident(),
                subtype_indication: code.s1("natural").subtype_indication(),
                expression: Some(code.s1("5").expr())
            })])
        );

        assert_eq!(
            msgs,
            vec![Diagnostic::error(
                code.s1("var").pos(),
                "Expected 'type', 'subtype', 'component', 'impure', \
                 'function', 'procedure', 'package', 'for', 'file', \
                 'shared', 'constant', 'signal', 'variable', 'attribute', \
                 'use' or 'alias'"
            )]
        );
    }

    #[test]
    fn parse_declarative_part_error() {
        // Just checking that there is not an infinite loop
        let code = Code::new("invalid");
        let (decl, _) =
            code.with_partial_stream_diagnostics(parse_declarative_part_leave_end_token);
        assert!(decl.is_err());
    }
}
