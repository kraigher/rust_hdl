// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2020, Olof Kraigher olof.kraigher@gmail.com

use super::common::error_on_end_identifier_mismatch;
use super::common::ParseResult;
use super::concurrent_statement::parse_generic_and_port_map;
use super::context::parse_use_clause_no_keyword;
use super::names::{parse_name, parse_name_initial_token, parse_selected_name};
use super::tokens::{Kind::*, TokenStream};
use crate::ast::*;
use crate::data::*;

/// LRM 7.3.2.2
fn parse_entity_aspect(stream: &mut TokenStream) -> ParseResult<EntityAspect> {
    let token = stream.expect()?;
    let entity_aspect = try_token_kind!(
        token,
        Open => EntityAspect::Open,
        Configuration => EntityAspect::Configuration(parse_selected_name(stream)?),
        Entity => {
            let entity_name = parse_selected_name(stream)?;
            let arch_name = {
                if stream.skip_if_kind(LeftPar)? {
                    let ident = stream.expect_ident()?;
                    stream.expect_kind(RightPar)?;
                    Some(ident)
                } else {
                    None
                }
            };
            EntityAspect::Entity(entity_name, arch_name)
        }
    );
    Ok(entity_aspect)
}

fn parse_binding_indication_known_entity_aspect(
    entity_aspect: Option<EntityAspect>,
    stream: &mut TokenStream,
) -> ParseResult<BindingIndication> {
    let (generic_map, port_map) = parse_generic_and_port_map(stream)?;

    stream.expect_kind(SemiColon)?;
    Ok(BindingIndication {
        entity_aspect,
        generic_map,
        port_map,
    })
}

/// LRM 7.3.2
fn parse_binding_indication(stream: &mut TokenStream) -> ParseResult<BindingIndication> {
    let entity_aspect = if stream.skip_if_kind(Use)? {
        Some(parse_entity_aspect(stream)?)
    } else {
        None
    };
    parse_binding_indication_known_entity_aspect(entity_aspect, stream)
}

fn parse_component_configuration_known_spec(
    stream: &mut TokenStream,
    spec: ComponentSpecification,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<ComponentConfiguration> {
    let token = stream.peek_expect()?;
    let (bind_ind, vunit_bind_inds) = try_token_kind!(
        token,
        End => (None, Vec::new()),
        For => (None, Vec::new()),
        Use => {
            stream.move_after(&token);
            if stream.peek_kind()? == Some(Vunit) {
                let vunit_bind_inds = parse_vunit_binding_indication_list_known_keyword(stream)?;
                (None, vunit_bind_inds)
            } else {
                let aspect = parse_entity_aspect(stream)?;
                let bind_ind = parse_binding_indication_known_entity_aspect(Some(aspect), stream)?;

                if stream.skip_if_kind(Use)? {
                    (Some(bind_ind), parse_vunit_binding_indication_list_known_keyword(stream)?)
                } else {
                    (Some(bind_ind), Vec::new())
                }
            }
        }
    );

    let token = stream.expect()?;
    let block_config = try_token_kind!(
        token,
        End => None,
        For => {
            let block_config = parse_block_configuration_known_keyword(stream, diagnostics)?;
            stream.expect_kind(End)?;
            Some(block_config)
        }
    );

    stream.expect_kind(For)?;
    stream.expect_kind(SemiColon)?;
    Ok(ComponentConfiguration {
        spec,
        bind_ind,
        vunit_bind_inds,
        block_config,
    })
}

enum ComponentSpecificationOrName {
    ComponentSpec(ComponentSpecification),
    Name(WithPos<Name>),
}

fn parse_component_specification_or_name(
    stream: &mut TokenStream,
) -> ParseResult<ComponentSpecificationOrName> {
    let name_token = stream.expect()?;
    try_token_kind!(
        name_token,
        All => {
            stream.expect_kind(Colon)?;
            let component_name = parse_selected_name(stream)?;
            Ok(ComponentSpecificationOrName::ComponentSpec(ComponentSpecification {
                instantiation_list: InstantiationList::All,
                component_name,
            }))

        },
        Others => {
            stream.expect_kind(Colon)?;
            let component_name = parse_selected_name(stream)?;
            Ok(ComponentSpecificationOrName::ComponentSpec(ComponentSpecification {
                instantiation_list: InstantiationList::Others,
                component_name,
            }))
        },
        Identifier => {
            let name = parse_name_initial_token(stream, name_token)?;
            let sep_token = stream.peek_expect()?;
            match sep_token.kind {
                Colon => {
                    stream.move_after(&sep_token);
                    let ident = to_simple_name(name)?;
                    let component_name = parse_selected_name(stream)?;
                    Ok(ComponentSpecificationOrName::ComponentSpec(ComponentSpecification {
                        instantiation_list: InstantiationList::Labels(vec![ident]),
                        component_name,
                    }))
                }
                Comma => {
                    stream.move_after(&sep_token);
                    let mut idents = vec![to_simple_name(name)?];
                    loop {
                        idents.push(stream.expect_ident()?);
                        let next_token = stream.expect()?;
                        try_token_kind!(
                            next_token,
                            Comma => {},
                            Colon => break
                        );
                    }
                    let component_name = parse_selected_name(stream)?;
                    Ok(ComponentSpecificationOrName::ComponentSpec(ComponentSpecification {
                        instantiation_list: InstantiationList::Labels(idents),
                        component_name,
                    }))
                }
                _ => Ok(ComponentSpecificationOrName::Name(name))
            }
        }
    )
}

fn parse_configuration_item_known_keyword(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<ConfigurationItem> {
    match parse_component_specification_or_name(stream)? {
        ComponentSpecificationOrName::ComponentSpec(component_spec) => {
            Ok(ConfigurationItem::Component(
                parse_component_configuration_known_spec(stream, component_spec, diagnostics)?,
            ))
        }
        ComponentSpecificationOrName::Name(name) => Ok(ConfigurationItem::Block(
            parse_block_configuration_known_name(stream, name, diagnostics)?,
        )),
    }
}

fn parse_block_configuration_known_name(
    stream: &mut TokenStream,
    name: WithPos<Name>,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<BlockConfiguration> {
    let block_spec = name;
    // @TODO use clauses
    let use_clauses = Vec::new();
    let mut items = Vec::new();

    loop {
        let token = stream.expect()?;
        try_token_kind!(
            token,
            End => {
                break;
            },
            For => {
                items.push(parse_configuration_item_known_keyword(stream, diagnostics)?);
            }
        );
    }
    stream.expect_kind(For)?;
    stream.expect_kind(SemiColon)?;
    Ok(BlockConfiguration {
        block_spec,
        use_clauses,
        items,
    })
}

fn parse_block_configuration_known_keyword(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<BlockConfiguration> {
    let name = parse_name(stream)?;
    parse_block_configuration_known_name(stream, name, diagnostics)
}

fn parse_vunit_binding_indication_list_known_keyword(
    stream: &mut TokenStream,
) -> ParseResult<Vec<VUnitBindingIndication>> {
    let mut indications = Vec::new();
    loop {
        stream.expect_kind(Vunit)?;

        let mut vunit_list = Vec::new();

        let vunit_bind_ind = loop {
            vunit_list.push(parse_name(stream)?);
            let token = stream.peek_expect()?;
            try_token_kind!(
                token,
                Comma => {
                    stream.move_after(&token);
                },
                SemiColon => {
                    stream.move_after(&token);
                    break VUnitBindingIndication { vunit_list };
                }
            );
        };

        indications.push(vunit_bind_ind);

        if !stream.skip_if_kind(Use)? {
            break;
        }
    }
    Ok(indications)
}

/// LRM 3.4 Configuration declaration
pub fn parse_configuration_declaration(
    stream: &mut TokenStream,
    diagnostics: &mut dyn DiagnosticHandler,
) -> ParseResult<ConfigurationDeclaration> {
    let configuration_token = stream.expect_kind(Configuration)?.into();
    let ident = stream.expect_ident()?;
    stream.expect_kind(Of)?;
    let entity_name = parse_selected_name(stream)?;
    let is_token = stream.expect_kind(Is)?.into();
    let mut decl = Vec::new();

    let vunit_bind_inds = loop {
        let token = stream.peek_expect()?;
        match token.kind {
            Use => {
                stream.move_after(&token);
                if stream.peek_kind()? == Some(Vunit) {
                    break parse_vunit_binding_indication_list_known_keyword(stream)?;
                }

                decl.push(ConfigurationDeclarativeItem::Use(
                    parse_use_clause_no_keyword(token, stream)?,
                ));
            }
            _ => break Vec::new(),
        }
    };

    stream.expect_kind(For)?;
    let block_config = parse_block_configuration_known_keyword(stream, diagnostics)?;

    let end_token = stream.expect_kind(End)?.into();
    stream.pop_if_kind(Configuration)?;
    let end_ident = stream.pop_optional_ident()?;
    if let Some(diagnostic) = error_on_end_identifier_mismatch(&ident, &end_ident) {
        diagnostics.push(diagnostic)
    }
    let semi_token = stream.expect_kind(SemiColon)?.into();
    Ok(ConfigurationDeclaration {
        context_clause: ContextClause::default(),
        ident,
        entity_name,
        decl,
        vunit_bind_inds,
        block_config,
        configuration_token,
        is_token,
        end_token,
        semi_token,
    })
}

/// LRM 7.3 Configuration Specification
pub fn parse_configuration_specification(
    stream: &mut TokenStream,
) -> ParseResult<ConfigurationSpecification> {
    stream.expect_kind(For)?;
    match parse_component_specification_or_name(stream)? {
        ComponentSpecificationOrName::ComponentSpec(spec) => {
            let bind_ind = parse_binding_indication(stream)?;
            if stream.skip_if_kind(Use)? {
                let vunit_bind_inds = parse_vunit_binding_indication_list_known_keyword(stream)?;
                stream.expect_kind(End)?;
                stream.expect_kind(For)?;
                stream.expect_kind(SemiColon)?;
                Ok(ConfigurationSpecification {
                    spec,
                    bind_ind,
                    vunit_bind_inds,
                })
            } else {
                if stream.skip_if_kind(End)? {
                    stream.expect_kind(For)?;
                    stream.expect_kind(SemiColon)?;
                }
                Ok(ConfigurationSpecification {
                    spec,
                    bind_ind,
                    vunit_bind_inds: Vec::new(),
                })
            }
        }
        ComponentSpecificationOrName::Name(name) => {
            Err(Diagnostic::error(name, "Expected component specification"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::test::Code;
    use pretty_assertions::assert_eq;

    #[test]
    fn empty_configuration() {
        let code = Code::new(
            "\
configuration cfg of entity_name is
  for rtl(0)
  end for;
end;
",
        );
        assert_eq!(
            code.with_stream_no_diagnostics(parse_configuration_declaration),
            ConfigurationDeclaration {
                context_clause: ContextClause::default(),
                ident: code.s1("cfg").ident(),
                entity_name: code.s1("entity_name").selected_name(),
                decl: vec![],
                vunit_bind_inds: Vec::new(),
                block_config: BlockConfiguration {
                    block_spec: code.s1("rtl(0)").name(),
                    use_clauses: vec![],
                    items: vec![],
                },
                configuration_token: code.keyword_token(Configuration, 1),
                is_token: code.keyword_token(Is, 1),
                end_token: code.keyword_token(End, -1),
                semi_token: code.keyword_token(SemiColon, -1),
            }
        );
    }

    #[test]
    fn empty_configuration_variant() {
        let code = Code::new(
            "\
configuration cfg of entity_name is
  for rtl(0)
  end for;
end configuration cfg;
",
        );
        assert_eq!(
            code.with_stream_no_diagnostics(parse_configuration_declaration),
            ConfigurationDeclaration {
                context_clause: ContextClause::default(),
                ident: code.s1("cfg").ident(),
                entity_name: code.s1("entity_name").selected_name(),
                decl: vec![],
                vunit_bind_inds: Vec::new(),
                block_config: BlockConfiguration {
                    block_spec: code.s1("rtl(0)").name(),
                    use_clauses: vec![],
                    items: vec![],
                },
                configuration_token: code.keyword_token(Configuration, 1),
                is_token: code.keyword_token(Is, 1),
                end_token: code.keyword_token(End, -1),
                semi_token: code.keyword_token(SemiColon, -1),
            }
        );
    }
    #[test]
    fn configuration_use_clause() {
        let code = Code::new(
            "\
configuration cfg of entity_name is
  use lib.foo.bar;
  use lib2.foo.bar;
  for rtl(0)
  end for;
end configuration cfg;
",
        );
        assert_eq!(
            code.with_stream_no_diagnostics(parse_configuration_declaration),
            ConfigurationDeclaration {
                context_clause: ContextClause::default(),
                ident: code.s1("cfg").ident(),
                entity_name: code.s1("entity_name").selected_name(),
                decl: vec![
                    ConfigurationDeclarativeItem::Use(code.s1("use lib.foo.bar;").use_clause()),
                    ConfigurationDeclarativeItem::Use(code.s1("use lib2.foo.bar;").use_clause())
                ],
                vunit_bind_inds: Vec::new(),
                block_config: BlockConfiguration {
                    block_spec: code.s1("rtl(0)").name(),
                    use_clauses: vec![],
                    items: vec![],
                },
                configuration_token: code.keyword_token(Configuration, 1),
                is_token: code.keyword_token(Is, 1),
                end_token: code.keyword_token(End, -1),
                semi_token: code.keyword_token(SemiColon, -1),
            }
        );
    }

    #[test]
    fn configuration_vunit_binding_indication() {
        let code = Code::new(
            "\
configuration cfg of entity_name is
  use lib.foo.bar;
  use vunit baz.foobar;
  for rtl(0)
  end for;
end configuration cfg;
",
        );
        assert_eq!(
            code.with_stream_no_diagnostics(parse_configuration_declaration),
            ConfigurationDeclaration {
                context_clause: ContextClause::default(),
                ident: code.s1("cfg").ident(),
                entity_name: code.s1("entity_name").selected_name(),
                decl: vec![ConfigurationDeclarativeItem::Use(
                    code.s1("use lib.foo.bar;").use_clause()
                ),],
                vunit_bind_inds: vec![VUnitBindingIndication {
                    vunit_list: vec![code.s1("baz.foobar").name()]
                }],
                block_config: BlockConfiguration {
                    block_spec: code.s1("rtl(0)").name(),
                    use_clauses: vec![],
                    items: vec![],
                },
                configuration_token: code.keyword_token(Configuration, 1),
                is_token: code.keyword_token(Is, 1),
                end_token: code.keyword_token(End, -1),
                semi_token: code.keyword_token(SemiColon, -1),
            }
        );
    }

    #[test]
    fn configuration_block_configuration() {
        let code = Code::new(
            "\
configuration cfg of entity_name is
  for rtl(0)
  end for;
end configuration cfg;
",
        );
        assert_eq!(
            code.with_stream_no_diagnostics(parse_configuration_declaration),
            ConfigurationDeclaration {
                context_clause: ContextClause::default(),
                ident: code.s1("cfg").ident(),
                entity_name: code.s1("entity_name").selected_name(),
                decl: vec![],
                vunit_bind_inds: Vec::new(),
                block_config: BlockConfiguration {
                    block_spec: code.s1("rtl(0)").name(),
                    use_clauses: vec![],
                    items: vec![],
                },
                configuration_token: code.keyword_token(Configuration, 1),
                is_token: code.keyword_token(Is, 1),
                end_token: code.keyword_token(End, -1),
                semi_token: code.keyword_token(SemiColon, -1),
            }
        );
    }

    #[test]
    fn configuration_nested_block_configuration() {
        let code = Code::new(
            "\
configuration cfg of entity_name is
  for rtl(0)
    for name(0 to 3)
    end for;
    for other_name
    end for;
  end for;
end configuration cfg;
",
        );
        assert_eq!(
            code.with_stream_no_diagnostics(parse_configuration_declaration),
            ConfigurationDeclaration {
                context_clause: ContextClause::default(),
                ident: code.s1("cfg").ident(),
                entity_name: code.s1("entity_name").selected_name(),
                decl: vec![],
                vunit_bind_inds: Vec::new(),
                block_config: BlockConfiguration {
                    block_spec: code.s1("rtl(0)").name(),
                    use_clauses: vec![],
                    items: vec![
                        ConfigurationItem::Block(BlockConfiguration {
                            block_spec: code.s1("name(0 to 3)").name(),
                            use_clauses: vec![],
                            items: vec![],
                        }),
                        ConfigurationItem::Block(BlockConfiguration {
                            block_spec: code.s1("other_name").name(),
                            use_clauses: vec![],
                            items: vec![],
                        })
                    ],
                },
                configuration_token: code.keyword_token(Configuration, 1),
                is_token: code.keyword_token(Is, 1),
                end_token: code.keyword_token(End, -1),
                semi_token: code.keyword_token(SemiColon, -1),
            }
        );
    }

    #[test]
    fn configuration_component_configuration_nested() {
        let code = Code::new(
            "\
configuration cfg of entity_name is
  for rtl(0)
    for inst : lib.pkg.comp
      for arch
      end for;
    end for;
  end for;
end configuration cfg;
",
        );
        assert_eq!(
            code.with_stream_no_diagnostics(parse_configuration_declaration),
            ConfigurationDeclaration {
                context_clause: ContextClause::default(),
                ident: code.s1("cfg").ident(),
                entity_name: code.s1("entity_name").selected_name(),
                decl: vec![],
                vunit_bind_inds: Vec::new(),
                block_config: BlockConfiguration {
                    block_spec: code.s1("rtl(0)").name(),
                    use_clauses: vec![],
                    items: vec![ConfigurationItem::Component(ComponentConfiguration {
                        spec: ComponentSpecification {
                            instantiation_list: InstantiationList::Labels(vec![code
                                .s1("inst")
                                .ident()]),
                            component_name: code.s1("lib.pkg.comp").selected_name()
                        },
                        bind_ind: None,
                        vunit_bind_inds: Vec::new(),
                        block_config: Some(BlockConfiguration {
                            block_spec: code.s1("arch").name(),
                            use_clauses: vec![],
                            items: vec![],
                        }),
                    }),],
                },
                configuration_token: code.keyword_token(Configuration, 1),
                is_token: code.keyword_token(Is, 1),
                end_token: code.keyword_token(End, -1),
                semi_token: code.keyword_token(SemiColon, -1),
            }
        );
    }

    #[test]
    fn configuration_component_configuration_vunit_binding_indication() {
        let code = Code::new(
            "\
configuration cfg of entity_name is
  for rtl(0)
    for inst : lib.pkg.comp
      use entity work.bar;
      use vunit baz;
      for arch
      end for;
    end for;
  end for;
end configuration cfg;
",
        );
        assert_eq!(
            code.with_stream_no_diagnostics(parse_configuration_declaration),
            ConfigurationDeclaration {
                context_clause: ContextClause::default(),
                ident: code.s1("cfg").ident(),
                entity_name: code.s1("entity_name").selected_name(),
                decl: vec![],
                vunit_bind_inds: Vec::new(),
                block_config: BlockConfiguration {
                    block_spec: code.s1("rtl(0)").name(),
                    use_clauses: vec![],
                    items: vec![ConfigurationItem::Component(ComponentConfiguration {
                        spec: ComponentSpecification {
                            instantiation_list: InstantiationList::Labels(vec![code
                                .s1("inst")
                                .ident()]),
                            component_name: code.s1("lib.pkg.comp").selected_name()
                        },
                        bind_ind: Some(BindingIndication {
                            entity_aspect: Some(EntityAspect::Entity(
                                code.s1("work.bar").selected_name(),
                                None
                            )),
                            generic_map: None,
                            port_map: None
                        }),
                        vunit_bind_inds: vec![VUnitBindingIndication {
                            vunit_list: vec![code.s1("baz").name()]
                        },],
                        block_config: Some(BlockConfiguration {
                            block_spec: code.s1("arch").name(),
                            use_clauses: vec![],
                            items: vec![],
                        }),
                    }),],
                },
                configuration_token: code.keyword_token(Configuration, 1),
                is_token: code.keyword_token(Is, 1),
                end_token: code.keyword_token(End, -1),
                semi_token: code.keyword_token(SemiColon, -1),
            }
        );
    }

    #[test]
    fn configuration_component_configuration_binding_indication() {
        let code = Code::new(
            "\
configuration cfg of entity_name is
  for rtl(0)
    for inst : lib.pkg.comp
      use entity lib.use_name;
    end for;
  end for;
end configuration cfg;
",
        );
        assert_eq!(
            code.with_stream_no_diagnostics(parse_configuration_declaration),
            ConfigurationDeclaration {
                context_clause: ContextClause::default(),
                ident: code.s1("cfg").ident(),
                entity_name: code.s1("entity_name").selected_name(),
                decl: vec![],
                vunit_bind_inds: Vec::new(),
                block_config: BlockConfiguration {
                    block_spec: code.s1("rtl(0)").name(),
                    use_clauses: vec![],
                    items: vec![ConfigurationItem::Component(ComponentConfiguration {
                        spec: ComponentSpecification {
                            instantiation_list: InstantiationList::Labels(vec![code
                                .s1("inst")
                                .ident()]),
                            component_name: code.s1("lib.pkg.comp").selected_name()
                        },
                        bind_ind: Some(BindingIndication {
                            entity_aspect: Some(EntityAspect::Entity(
                                code.s1("lib.use_name").selected_name(),
                                None
                            )),
                            generic_map: None,
                            port_map: None,
                        }),
                        vunit_bind_inds: Vec::new(),
                        block_config: None,
                    }),],
                },
                configuration_token: code.keyword_token(Configuration, 1),
                is_token: code.keyword_token(Is, 1),
                end_token: code.keyword_token(End, -1),
                semi_token: code.keyword_token(SemiColon, -1),
            }
        );
    }

    #[test]
    fn configuration_component_configuration() {
        let code = Code::new(
            "\
configuration cfg of entity_name is
  for rtl(0)
    for inst : lib.pkg.comp
    end for;
    for inst1, inst2, inst3 : lib2.pkg.comp
    end for;
    for all : lib3.pkg.comp
    end for;
    for others : lib4.pkg.comp
    end for;
  end for;
end configuration cfg;
",
        );
        assert_eq!(
            code.with_stream_no_diagnostics(parse_configuration_declaration),
            ConfigurationDeclaration {
                context_clause: ContextClause::default(),
                ident: code.s1("cfg").ident(),
                entity_name: code.s1("entity_name").selected_name(),
                decl: vec![],
                vunit_bind_inds: Vec::new(),
                block_config: BlockConfiguration {
                    block_spec: code.s1("rtl(0)").name(),
                    use_clauses: vec![],
                    items: vec![
                        ConfigurationItem::Component(ComponentConfiguration {
                            spec: ComponentSpecification {
                                instantiation_list: InstantiationList::Labels(vec![code
                                    .s1("inst")
                                    .ident()]),
                                component_name: code.s1("lib.pkg.comp").selected_name()
                            },
                            bind_ind: None,
                            vunit_bind_inds: Vec::new(),
                            block_config: None,
                        }),
                        ConfigurationItem::Component(ComponentConfiguration {
                            spec: ComponentSpecification {
                                instantiation_list: InstantiationList::Labels(vec![
                                    code.s1("inst1").ident(),
                                    code.s1("inst2").ident(),
                                    code.s1("inst3").ident()
                                ]),
                                component_name: code.s1("lib2.pkg.comp").selected_name()
                            },
                            bind_ind: None,
                            vunit_bind_inds: Vec::new(),
                            block_config: None,
                        }),
                        ConfigurationItem::Component(ComponentConfiguration {
                            spec: ComponentSpecification {
                                instantiation_list: InstantiationList::All,
                                component_name: code.s1("lib3.pkg.comp").selected_name()
                            },
                            bind_ind: None,
                            vunit_bind_inds: Vec::new(),
                            block_config: None,
                        }),
                        ConfigurationItem::Component(ComponentConfiguration {
                            spec: ComponentSpecification {
                                instantiation_list: InstantiationList::Others,
                                component_name: code.s1("lib4.pkg.comp").selected_name()
                            },
                            bind_ind: None,
                            vunit_bind_inds: Vec::new(),
                            block_config: None,
                        })
                    ],
                },
                configuration_token: code.keyword_token(Configuration, 1),
                is_token: code.keyword_token(Is, 1),
                end_token: code.keyword_token(End, -1),
                semi_token: code.keyword_token(SemiColon, -1),
            }
        );
    }

    #[test]
    fn entity_entity_aspect_entity() {
        let code = Code::new("entity lib.foo.name");
        assert_eq!(
            code.with_stream(parse_entity_aspect),
            EntityAspect::Entity(code.s1("lib.foo.name").selected_name(), None)
        );
    }

    #[test]
    fn entity_entity_aspect_entity_arch() {
        let code = Code::new("entity lib.foo.name(arch)");
        assert_eq!(
            code.with_stream(parse_entity_aspect),
            EntityAspect::Entity(
                code.s1("lib.foo.name").selected_name(),
                Some(code.s1("arch").ident())
            )
        );
    }

    #[test]
    fn entity_entity_aspect_configuration() {
        let code = Code::new("configuration lib.foo.name");
        assert_eq!(
            code.with_stream(parse_entity_aspect),
            EntityAspect::Configuration(code.s1("lib.foo.name").selected_name())
        );
    }

    #[test]
    fn entity_entity_aspect_open() {
        let code = Code::new("open");
        assert_eq!(code.with_stream(parse_entity_aspect), EntityAspect::Open);
    }

    #[test]
    fn simple_configuration_specification() {
        let code = Code::new("for all : lib.pkg.comp use entity work.foo(rtl);");

        assert_eq!(
            code.with_stream(parse_configuration_specification),
            ConfigurationSpecification {
                spec: ComponentSpecification {
                    instantiation_list: InstantiationList::All,
                    component_name: code.s1("lib.pkg.comp").selected_name(),
                },
                bind_ind: BindingIndication {
                    entity_aspect: Some(EntityAspect::Entity(
                        code.s1("work.foo").selected_name(),
                        Some(code.s1("rtl").ident())
                    )),
                    generic_map: None,
                    port_map: None
                },
                vunit_bind_inds: Vec::new()
            }
        );
    }

    #[test]
    fn simple_configuration_specification_end_for() {
        let code = Code::new("for all : lib.pkg.comp use entity work.foo(rtl); end for;");

        assert_eq!(
            code.with_stream(parse_configuration_specification),
            ConfigurationSpecification {
                spec: ComponentSpecification {
                    instantiation_list: InstantiationList::All,
                    component_name: code.s1("lib.pkg.comp").selected_name(),
                },
                bind_ind: BindingIndication {
                    entity_aspect: Some(EntityAspect::Entity(
                        code.s1("work.foo").selected_name(),
                        Some(code.s1("rtl").ident())
                    )),
                    generic_map: None,
                    port_map: None
                },
                vunit_bind_inds: Vec::new()
            }
        );
    }

    #[test]
    fn compound_configuration_specification() {
        let code = Code::new(
            "for all : lib.pkg.comp use entity work.foo(rtl); use vunit bar, baz; end for;",
        );

        assert_eq!(
            code.with_stream(parse_configuration_specification),
            ConfigurationSpecification {
                spec: ComponentSpecification {
                    instantiation_list: InstantiationList::All,
                    component_name: code.s1("lib.pkg.comp").selected_name(),
                },
                bind_ind: BindingIndication {
                    entity_aspect: Some(EntityAspect::Entity(
                        code.s1("work.foo").selected_name(),
                        Some(code.s1("rtl").ident())
                    )),
                    generic_map: None,
                    port_map: None
                },
                vunit_bind_inds: vec![VUnitBindingIndication {
                    vunit_list: vec![code.s1("bar").name(), code.s1("baz").name()]
                }],
            }
        );
    }
}
