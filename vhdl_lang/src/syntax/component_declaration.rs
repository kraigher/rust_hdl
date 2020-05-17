// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2018, Olof Kraigher olof.kraigher@gmail.com

use super::common::error_on_end_identifier_mismatch;
use super::common::ParseResult;
use super::interface_declaration::{parse_generic_interface_list, parse_port_interface_list};
use super::tokens::{Kind::*, TokenStream};
use crate::ast::{ComponentDeclaration, InterfaceList};
use crate::data::{Diagnostic, DiagnosticHandler};

pub fn parse_optional_generic_list(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<Option<InterfaceList>> {
    let mut list = None;
    loop {
        let token = stream.peek_expect()?;
        match token.kind {
            Generic => {
                stream.move_after(&token);
                let new_list = parse_generic_interface_list(stream, diagnostics)?;
                let semi_token = stream.expect_kind(SemiColon)?;
                if list.is_some() {
                    diagnostics.push(Diagnostic::error(token, "Duplicate generic clause"));
                } else {
                    list = Some(InterfaceList {
                        items: new_list,
                        start_token: token.into(),
                        semi_token: semi_token.into(),
                    });
                }
            }
            _ => break,
        }
    }

    Ok(list)
}

pub fn parse_optional_port_list(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<Option<InterfaceList>> {
    let mut list = None;
    loop {
        let token = stream.peek_expect()?;
        match token.kind {
            Port => {
                stream.move_after(&token);
                let new_list = parse_port_interface_list(stream, diagnostics)?;
                let semi_token = stream.expect_kind(SemiColon)?;
                if list.is_some() {
                    diagnostics.push(Diagnostic::error(token, "Duplicate port clause"));
                } else {
                    list = Some(InterfaceList {
                        items: new_list,
                        start_token: token.into(),
                        semi_token: semi_token.into(),
                    });
                }
            }
            Generic => {
                stream.move_after(&token);
                parse_generic_interface_list(stream, diagnostics)?;
                stream.expect_kind(SemiColon)?;
                diagnostics.push(Diagnostic::error(
                    token,
                    "Generic clause must come before port clause",
                ));
            }
            _ => break,
        }
    }

    Ok(list)
}

pub fn parse_component_declaration(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<ComponentDeclaration> {
    stream.expect_kind(Component)?;
    let ident = stream.expect_ident()?;
    stream.pop_if_kind(Is)?;

    let generic_list = parse_optional_generic_list(stream, diagnostics)?;
    let port_list = parse_optional_port_list(stream, diagnostics)?;
    stream.expect_kind(End)?;
    stream.expect_kind(Component)?;
    if let Some(token) = stream.pop_if_kind(Identifier)? {
        diagnostics.push_some(error_on_end_identifier_mismatch(
            &ident,
            &Some(token.expect_ident()?),
        ));
    }
    stream.expect_kind(SemiColon)?;

    Ok(ComponentDeclaration {
        ident,
        generic_list: generic_list.map_or(Vec::new(), |list| list.items),
        port_list: port_list.map_or(Vec::new(), |list| list.items),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::{Ident, InterfaceDeclaration};
    use crate::syntax::test::Code;

    fn to_component(
        ident: Ident,
        generic_list: Vec<InterfaceDeclaration>,
        port_list: Vec<InterfaceDeclaration>,
    ) -> ComponentDeclaration {
        ComponentDeclaration {
            ident,
            generic_list,
            port_list,
        }
    }

    #[test]
    fn test_component() {
        let code = Code::new(
            "\
component foo
end component;
",
        );
        let component = code.with_stream_no_diagnostics(parse_component_declaration);
        assert_eq!(
            component,
            to_component(code.s1("foo").ident(), vec![], vec![])
        );

        let code = Code::new(
            "\
component foo is
end component;
",
        );
        let component = code.with_stream_no_diagnostics(parse_component_declaration);
        assert_eq!(
            component,
            to_component(code.s1("foo").ident(), vec![], vec![])
        );

        let code = Code::new(
            "\
component foo is
end component foo;
",
        );
        let component = code.with_stream_no_diagnostics(parse_component_declaration);
        assert_eq!(
            component,
            to_component(code.s1("foo").ident(), vec![], vec![])
        );
    }

    #[test]
    fn test_component_with_generic() {
        let code = Code::new(
            "\
component foo is
  generic (
    foo : natural
  );
end component;
",
        );
        let component = code.with_stream_no_diagnostics(parse_component_declaration);
        assert_eq!(
            component,
            to_component(
                code.s1("foo").ident(),
                vec![code.s1("foo : natural").generic()],
                vec![]
            )
        );
    }

    #[test]
    fn test_component_with_port() {
        let code = Code::new(
            "\
component foo is
  port (
    foo : natural
  );
end component;
",
        );
        let component = code.with_stream_no_diagnostics(parse_component_declaration);
        assert_eq!(
            component,
            to_component(
                code.s1("foo").ident(),
                vec![],
                vec![code.s1("foo : natural").port()]
            )
        );
    }

    #[test]
    fn error_on_duplicate_generic_clause() {
        let code = Code::new(
            "\
generic (
  foo : natural
);
generic (
  bar : natural
);
end
",
        );

        let (result, diagnostics) =
            code.with_partial_stream_diagnostics(parse_optional_generic_list);
        assert_eq!(
            diagnostics,
            vec![Diagnostic::error(
                &code.s("generic", 2).pos(),
                "Duplicate generic clause"
            )]
        );
        assert_eq!(
            result,
            Ok(Some(InterfaceList {
                items: vec![code.s1("foo : natural").generic()],
                start_token: code.keyword_token(Generic, 1),
                semi_token: code.keyword_token(SemiColon, 1)
            })),
        );
    }

    #[test]
    fn error_on_duplicate_port_clause() {
        let code = Code::new(
            "\
port (
  foo : natural
);
port (
  bar : natural
);
end
",
        );
        let (result, diagnostics) = code.with_partial_stream_diagnostics(parse_optional_port_list);
        assert_eq!(
            diagnostics,
            vec![Diagnostic::error(
                code.s("port", 2),
                "Duplicate port clause"
            )]
        );
        assert_eq!(
            result,
            Ok(Some(InterfaceList {
                items: vec![code.s1("foo : natural").port()],
                start_token: code.keyword_token(Port, 1),
                semi_token: code.keyword_token(SemiColon, 1)
            })),
        );
    }

    #[test]
    fn error_generic_after_port_clause() {
        let code = Code::new(
            "\
port (
  foo : natural
);
generic (
  bar : natural
);
end
",
        );
        let (result, diagnostics) = code.with_partial_stream_diagnostics(parse_optional_port_list);
        assert_eq!(
            diagnostics,
            vec![Diagnostic::error(
                code.s1("generic"),
                "Generic clause must come before port clause"
            )]
        );
        assert_eq!(
            result,
            Ok(Some(InterfaceList {
                items: vec![code.s1("foo : natural").port()],
                start_token: code.keyword_token(Port, 1),
                semi_token: code.keyword_token(SemiColon, 1)
            })),
        );
    }
}
