// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2020, Olof Kraigher olof.kraigher@gmail.com

use lsp_types::{DocumentSymbol, DocumentSymbolResponse, SymbolKind, Url};
use vhdl_lang::ast::*;
use vhdl_lang::Latin1String;
use vhdl_lang::{Source, VHDLParser};

pub fn nested_document_symbol_response_from_file(uri: &Url) -> Option<DocumentSymbolResponse> {
    match uri.to_file_path() {
        Ok(path) => {
            let mut diagnostics = vec![];
            match VHDLParser::default().parse_design_file(&path, &mut diagnostics) {
                Ok((_, design_file)) => Some(nested_document_symbol_response(&design_file)),
                Err(_) => None,
            }
        }
        _ => None,
    }
}

pub fn nested_document_symbol_response_from_source(source: &Source) -> DocumentSymbolResponse {
    let mut diagnostics = vec![];
    let design_file = VHDLParser::default().parse_design_source(&source, &mut diagnostics);
    nested_document_symbol_response(&design_file)
}

pub fn nested_document_symbol_response(design_file: &DesignFile) -> DocumentSymbolResponse {
    let mut response = vec![];
    for design_unit in design_file.design_units.iter() {
        response.push(design_unit.document_symbol());
    }
    DocumentSymbolResponse::from(response)
}

pub trait HasDocumentSymbol {
    fn document_symbol(&self) -> DocumentSymbol;
}

impl HasDocumentSymbol for AnyDesignUnit {
    fn document_symbol(&self) -> DocumentSymbol {
        match self {
            AnyDesignUnit::Primary(ref unit) => unit.document_symbol(),
            AnyDesignUnit::Secondary(ref unit) => unit.document_symbol(),
        }
    }
}

impl HasDocumentSymbol for AnyPrimaryUnit {
    fn document_symbol(&self) -> DocumentSymbol {
        match self {
            AnyPrimaryUnit::Entity(ref unit) => unit.document_symbol(),
            AnyPrimaryUnit::Configuration(ref unit) => unit.document_symbol(),
            AnyPrimaryUnit::Package(ref unit) => unit.document_symbol(),
            AnyPrimaryUnit::PackageInstance(ref unit) => unit.document_symbol(),
            AnyPrimaryUnit::Context(ref unit) => unit.document_symbol(),
        }
    }
}

impl HasDocumentSymbol for EntityDeclaration {
    fn document_symbol(&self) -> DocumentSymbol {
        let mut children = vec![];
        push_context_clause(self.context_clause.clone(), &mut children);
        push_generic_interface_list(self.generic_clause.as_ref(), &mut children);
        push_port_interface_list(self.port_clause.as_ref(), &mut children);
        let decl_start = if let Some(ref ports) = self.port_clause {
            ports.semi_token.clone()
        } else if let Some(ref generics) = self.generic_clause {
            generics.semi_token.clone()
        } else {
            self.is_token.clone()
        };
        let decl_end = if let Some(ref begin) = self.begin_token {
            begin.clone()
        } else {
            self.end_token.clone()
        };
        if !self.decl.is_empty() {
            push_declarations(&decl_start, &decl_end, &self.decl, &mut children);
        }
        if let Some(ref begin) = self.begin_token {
            push_concurrent_statement_part(begin, &self.end_token, &self.statements, &mut children);
        }
        DocumentSymbol {
            name: self.ident.item.name_utf8(),
            detail: Some(String::from("entity")),
            kind: SymbolKind::Interface,
            deprecated: None,
            range: lsp_types::Range {
                start: to_lsp_pos(
                    self.context_clause
                        .first()
                        .map_or(self.entity_token.pos.start(), |first| first.pos.start()),
                ),
                end: to_lsp_pos(self.semi_token.pos.end()),
            },
            selection_range: to_lsp_range(self.ident.pos.range()),
            children: none_if_empty(children),
        }
    }
}

impl HasDocumentSymbol for ConfigurationDeclaration {
    fn document_symbol(&self) -> DocumentSymbol {
        let mut children = vec![];
        push_context_clause(self.context_clause.clone(), &mut children);
        DocumentSymbol {
            name: self.ident.item.name_utf8(),
            detail: Some(String::from("configuration")),
            kind: SymbolKind::Constructor,
            deprecated: None,
            range: lsp_types::Range {
                start: to_lsp_pos(
                    self.context_clause
                        .first()
                        .map_or(self.configuration_token.pos.start(), |first| {
                            first.pos.start()
                        }),
                ),
                end: to_lsp_pos(self.semi_token.pos.end()),
            },
            selection_range: to_lsp_range(self.ident.pos.range()),
            children: none_if_empty(children),
        }
    }
}

impl HasDocumentSymbol for PackageDeclaration {
    fn document_symbol(&self) -> DocumentSymbol {
        let mut children = vec![];
        push_context_clause(self.context_clause.clone(), &mut children);
        push_generic_interface_list(self.generic_clause.as_ref(), &mut children);
        push_declarations(
            self.generic_clause
                .as_ref()
                .map_or(&self.is_token, |generics| &generics.semi_token),
            &self.end_token,
            &self.decl,
            &mut children,
        );
        DocumentSymbol {
            name: self.ident.item.name_utf8(),
            detail: Some(String::from("package")),
            kind: SymbolKind::Package,
            deprecated: None,
            range: lsp_types::Range {
                start: to_lsp_pos(
                    self.context_clause
                        .first()
                        .map_or(self.package_token.pos.start(), |first| first.pos.start()),
                ),
                end: to_lsp_pos(self.semi_token.pos.end()),
            },
            selection_range: to_lsp_range(self.ident.pos.range()),
            children: none_if_empty(children),
        }
    }
}

impl HasDocumentSymbol for PackageInstantiation {
    fn document_symbol(&self) -> DocumentSymbol {
        let mut children = vec![];
        push_context_clause(self.context_clause.clone(), &mut children);
        DocumentSymbol {
            name: self.ident.item.name_utf8(),
            detail: Some(String::from("package instance")),
            kind: SymbolKind::Package,
            deprecated: None,
            range: lsp_types::Range {
                start: to_lsp_pos(
                    self.context_clause
                        .first()
                        .map_or(self.package_token.pos.start(), |first| first.pos.start()),
                ),
                end: to_lsp_pos(self.semi_token.pos.end()),
            },
            selection_range: to_lsp_range(self.ident.pos.range()),
            children: none_if_empty(children),
        }
    }
}

impl HasDocumentSymbol for ContextDeclaration {
    fn document_symbol(&self) -> DocumentSymbol {
        let mut children = vec![];
        push_context_clause(self.items.clone(), &mut children);
        DocumentSymbol {
            name: self.ident.item.name_utf8(),
            detail: Some(String::from("context")),
            kind: SymbolKind::Namespace,
            deprecated: None,
            range: lsp_range(&self.context_token, &self.semi_token),
            selection_range: to_lsp_range(self.ident.pos.range()),
            children: none_if_empty(children),
        }
    }
}

impl HasDocumentSymbol for AnySecondaryUnit {
    fn document_symbol(&self) -> DocumentSymbol {
        match self {
            AnySecondaryUnit::PackageBody(ref unit) => unit.document_symbol(),
            AnySecondaryUnit::Architecture(ref unit) => unit.document_symbol(),
        }
    }
}

impl HasDocumentSymbol for PackageBody {
    fn document_symbol(&self) -> DocumentSymbol {
        let mut children = vec![];
        push_context_clause(self.context_clause.clone(), &mut children);
        push_declarations(&self.is_token, &self.end_token, &self.decl, &mut children);
        DocumentSymbol {
            name: self.ident.item.item.name_utf8(),
            detail: Some(String::from("package body")),
            kind: SymbolKind::Package,
            deprecated: None,
            range: lsp_types::Range {
                start: to_lsp_pos(
                    self.context_clause
                        .first()
                        .map_or(self.package_token.pos.start(), |first| first.pos.start()),
                ),
                end: to_lsp_pos(self.semi_token.pos.end()),
            },
            selection_range: to_lsp_range(self.ident.item.pos.range()),
            children: none_if_empty(children),
        }
    }
}

impl HasDocumentSymbol for ArchitectureBody {
    fn document_symbol(&self) -> DocumentSymbol {
        let mut children = vec![];
        push_context_clause(self.context_clause.clone(), &mut children);
        push_declarations(&self.is_token, &self.begin_token, &self.decl, &mut children);
        push_concurrent_statement_part(
            &self.begin_token,
            &self.end_token,
            &self.statements,
            &mut children,
        );
        DocumentSymbol {
            name: self.ident.item.name_utf8(),
            detail: Some(format!(
                "architecture of {}",
                self.entity_name.item.item.name().to_string()
            )),
            kind: SymbolKind::Class,
            deprecated: None,
            range: lsp_types::Range {
                start: to_lsp_pos(
                    self.context_clause
                        .first()
                        .map_or(self.architecture_token.pos.start(), |first| {
                            first.pos.start()
                        }),
                ),
                end: to_lsp_pos(self.semi_token.pos.end()),
            },
            selection_range: to_lsp_range(self.ident.pos.range()),
            children: none_if_empty(children),
        }
    }
}

impl HasDocumentSymbol for ContextClause {
    fn document_symbol(&self) -> DocumentSymbol {
        let (range, selection_range, children) = {
            if let Some(first) = self.first() {
                (
                    to_lsp_range(first.pos.combine(&self.last().unwrap().pos).range()),
                    to_lsp_range(first.pos.range()),
                    {
                        let mut children = vec![];
                        for child in self.iter() {
                            children.push(child.item.document_symbol());
                        }
                        Some(children)
                    },
                )
            } else {
                (NULL_RANGE, NULL_RANGE, None)
            }
        };
        DocumentSymbol {
            name: String::from("context"),
            detail: None,
            kind: SymbolKind::Namespace,
            deprecated: None,
            range,
            selection_range,
            children,
        }
    }
}

impl HasDocumentSymbol for ContextItem {
    fn document_symbol(&self) -> DocumentSymbol {
        match self {
            ContextItem::Use(use_clause) => use_clause.document_symbol(),
            ContextItem::Library(library_clause) => library_clause.document_symbol(),
            ContextItem::Context(context_reference) => context_reference.document_symbol(),
        }
    }
}

impl HasDocumentSymbol for UseClause {
    fn document_symbol(&self) -> DocumentSymbol {
        if self.name_list.is_empty() {
            DocumentSymbol {
                name: String::from("use"),
                detail: None,
                kind: SymbolKind::Namespace,
                deprecated: None,
                range: NULL_RANGE,
                selection_range: NULL_RANGE,
                children: None,
            }
        } else if self.name_list.len() == 1 {
            let name = self.name_list.first().unwrap();
            DocumentSymbol {
                name: name_to_string(&name.item),
                detail: None,
                kind: SymbolKind::Namespace,
                deprecated: None,
                range: to_lsp_range(name.pos.range()),
                selection_range: to_lsp_range(name.pos.range()),
                children: None,
            }
        } else {
            let first = self.name_list.first().unwrap();
            let last = self.name_list.last().unwrap();
            DocumentSymbol {
                name: String::from("use"),
                detail: None,
                kind: SymbolKind::Namespace,
                deprecated: None,
                range: to_lsp_range(first.pos.combine(&last.pos).range()),
                selection_range: to_lsp_range(first.pos.range()),
                children: Some(
                    self.name_list
                        .iter()
                        .map(|name| DocumentSymbol {
                            name: name_to_string(&name.item),
                            detail: None,
                            kind: SymbolKind::Namespace,
                            deprecated: None,
                            range: to_lsp_range(name.pos.range()),
                            selection_range: to_lsp_range(name.pos.range()),
                            children: None,
                        })
                        .collect(),
                ),
            }
        }
    }
}

impl HasDocumentSymbol for LibraryClause {
    fn document_symbol(&self) -> DocumentSymbol {
        if self.name_list.is_empty() {
            DocumentSymbol {
                name: String::from("library"),
                detail: None,
                kind: SymbolKind::Namespace,
                deprecated: None,
                range: NULL_RANGE,
                selection_range: NULL_RANGE,
                children: None,
            }
        } else if self.name_list.len() == 1 {
            let name = self.name_list.first().unwrap();
            DocumentSymbol {
                name: name.item.name_utf8(),
                detail: None,
                kind: SymbolKind::Namespace,
                deprecated: None,
                range: to_lsp_range(name.pos.range()),
                selection_range: to_lsp_range(name.pos.range()),
                children: None,
            }
        } else {
            let first = self.name_list.first().unwrap();
            let last = self.name_list.last().unwrap();
            DocumentSymbol {
                name: String::from("library"),
                detail: None,
                kind: SymbolKind::Namespace,
                deprecated: None,
                range: to_lsp_range(first.pos.combine(&last.pos).range()),
                selection_range: to_lsp_range(first.pos.range()),
                children: Some(
                    self.name_list
                        .iter()
                        .map(|name| DocumentSymbol {
                            name: name.item.name_utf8(),
                            detail: None,
                            kind: SymbolKind::Namespace,
                            deprecated: None,
                            range: to_lsp_range(name.pos.range()),
                            selection_range: to_lsp_range(name.pos.range()),
                            children: None,
                        })
                        .collect(),
                ),
            }
        }
    }
}

impl HasDocumentSymbol for ContextReference {
    fn document_symbol(&self) -> DocumentSymbol {
        if self.name_list.is_empty() {
            DocumentSymbol {
                name: String::from("context"),
                detail: None,
                kind: SymbolKind::Namespace,
                deprecated: None,
                range: NULL_RANGE,
                selection_range: NULL_RANGE,
                children: None,
            }
        } else if self.name_list.len() == 1 {
            let name = self.name_list.first().unwrap();
            DocumentSymbol {
                name: name_to_string(&name.item),
                detail: None,
                kind: SymbolKind::Namespace,
                deprecated: None,
                range: to_lsp_range(name.pos.range()),
                selection_range: to_lsp_range(name.pos.range()),
                children: None,
            }
        } else {
            let first = self.name_list.first().unwrap();
            let last = self.name_list.last().unwrap();
            DocumentSymbol {
                name: String::from("context"),
                detail: None,
                kind: SymbolKind::Namespace,
                deprecated: None,
                range: to_lsp_range(first.pos.combine(&last.pos).range()),
                selection_range: to_lsp_range(first.pos.range()),
                children: Some(
                    self.name_list
                        .iter()
                        .map(|name| DocumentSymbol {
                            name: name_to_string(&name.item),
                            detail: None,
                            kind: SymbolKind::Namespace,
                            deprecated: None,
                            range: to_lsp_range(name.pos.range()),
                            selection_range: to_lsp_range(name.pos.range()),
                            children: None,
                        })
                        .collect(),
                ),
            }
        }
    }
}

impl HasDocumentSymbol for InterfaceList {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: String::from("interface list"),
            detail: None,
            kind: SymbolKind::Unknown,
            deprecated: None,
            range: lsp_range(&self.start_token, &self.semi_token),
            selection_range: lsp_token_range(&self.start_token),
            children: Some(
                self.items
                    .iter()
                    .map(|item| item.document_symbol())
                    .collect(),
            ),
        }
    }
}

impl HasDocumentSymbol for InterfaceDeclaration {
    fn document_symbol(&self) -> DocumentSymbol {
        match self {
            InterfaceDeclaration::Object(ref object) => object.document_symbol(),
            InterfaceDeclaration::File(ref file) => file.document_symbol(),
            InterfaceDeclaration::Type(ref ident) => DocumentSymbol {
                name: ident.item.name_utf8(),
                detail: Some(String::from("Type")),
                kind: SymbolKind::TypeParameter,
                deprecated: None,
                range: to_lsp_range(ident.pos.range()),
                selection_range: to_lsp_range(ident.pos.range()),
                children: None,
            },
            InterfaceDeclaration::Subprogram(ref subprogram_declaration, _) => {
                subprogram_declaration.document_symbol()
            }
            InterfaceDeclaration::Package(ref package) => package.document_symbol(),
        }
    }
}

impl HasDocumentSymbol for InterfaceObjectDeclaration {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: self.ident.item.name_utf8(),
            detail: {
                match self.class {
                    ObjectClass::Constant => None,
                    _ => Some(format!(": {}", mode_to_string(self.mode))),
                }
            },
            kind: symbol_kind_from_object_class(self.class),
            deprecated: None,
            range: to_lsp_range(self.ident.pos.range()),
            selection_range: to_lsp_range(self.ident.pos.range()),
            children: None,
        }
    }
}

impl HasDocumentSymbol for InterfaceFileDeclaration {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: self.ident.item.name_utf8(),
            detail: Some(String::from("File")),
            kind: SymbolKind::File,
            deprecated: None,
            range: to_lsp_range(self.ident.pos.range()),
            selection_range: to_lsp_range(self.ident.pos.range()),
            children: None,
        }
    }
}

impl HasDocumentSymbol for SubprogramDeclaration {
    fn document_symbol(&self) -> DocumentSymbol {
        match self {
            SubprogramDeclaration::Procedure(procedure) => DocumentSymbol {
                name: match procedure.designator.item {
                    SubprogramDesignator::Identifier(ref symbol) => symbol.name_utf8(),
                    SubprogramDesignator::OperatorSymbol(ref latin1string) => {
                        format!("\"{}\"", latin1string.to_string())
                    }
                },
                detail: Some(String::from("Procedure")),
                kind: SymbolKind::Method,
                deprecated: None,
                range: to_lsp_range(procedure.designator.pos.range()),
                selection_range: to_lsp_range(procedure.designator.pos.range()),
                children: None,
            },
            SubprogramDeclaration::Function(function) => DocumentSymbol {
                name: match function.designator.item {
                    SubprogramDesignator::Identifier(ref symbol) => symbol.name_utf8(),
                    SubprogramDesignator::OperatorSymbol(ref latin1string) => {
                        format!("\"{}\"", latin1string.to_string())
                    }
                },
                detail: Some(String::from("Function")),
                kind: SymbolKind::Function,
                deprecated: None,
                range: to_lsp_range(function.designator.pos.range()),
                selection_range: to_lsp_range(function.designator.pos.range()),
                children: None,
            },
        }
    }
}

impl HasDocumentSymbol for InterfacePackageDeclaration {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: self.ident.item.name_utf8(),
            detail: Some(String::from("Package")),
            kind: SymbolKind::Package,
            deprecated: None,
            range: to_lsp_range(self.ident.pos.range()),
            selection_range: to_lsp_range(self.ident.pos.range()),
            children: None,
        }
    }
}

impl HasDocumentSymbol for Declaration {
    fn document_symbol(&self) -> DocumentSymbol {
        match self {
            Declaration::Object(object) => object.document_symbol(),
            Declaration::File(file) => file.document_symbol(),
            Declaration::Type(type_decl) => type_decl.document_symbol(),
            Declaration::Component(component) => component.document_symbol(),
            Declaration::Attribute(attribute) => attribute.document_symbol(),
            Declaration::Alias(alias) => alias.document_symbol(),
            Declaration::SubprogramDeclaration(subprogram) => subprogram.document_symbol(),
            Declaration::SubprogramBody(subprogram) => subprogram.specification.document_symbol(),
            Declaration::Use(use_decl) => use_decl.item.document_symbol(), //WithPos<UseClause>
            Declaration::Package(package) => package.document_symbol(),
            Declaration::Configuration(configuration) => configuration.document_symbol(),
        }
    }
}

impl HasDocumentSymbol for ObjectDeclaration {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: self.ident.item.name_utf8(),
            detail: None,
            kind: symbol_kind_from_object_class(self.class),
            deprecated: None,
            range: to_lsp_range(self.ident.pos.range()),
            selection_range: to_lsp_range(self.ident.pos.range()),
            children: None,
        }
    }
}

impl HasDocumentSymbol for FileDeclaration {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: self.ident.item.name_utf8(),
            detail: None,
            kind: SymbolKind::File,
            deprecated: None,
            range: to_lsp_range(self.ident.pos.range()),
            selection_range: to_lsp_range(self.ident.pos.range()),
            children: None,
        }
    }
}

impl HasDocumentSymbol for TypeDeclaration {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: self.ident.item.name_utf8(),
            detail: Some(String::from("type")),
            kind: symbol_kind_from_type_definition(&self.def),
            deprecated: None,
            range: to_lsp_range(self.ident.pos.range()),
            selection_range: to_lsp_range(self.ident.pos.range()),
            children: None,
        }
    }
}

impl HasDocumentSymbol for ComponentDeclaration {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: self.ident.item.name_utf8(),
            detail: Some(String::from("component")),
            kind: SymbolKind::Interface,
            deprecated: None,
            range: to_lsp_range(self.ident.pos.range()),
            selection_range: to_lsp_range(self.ident.pos.range()),
            children: None,
        }
    }
}

impl HasDocumentSymbol for Attribute {
    fn document_symbol(&self) -> DocumentSymbol {
        match self {
            Attribute::Specification(spec) => DocumentSymbol {
                name: spec.ident.item.name_utf8(),
                detail: Some(String::from("attribute")),
                kind: SymbolKind::Property,
                deprecated: None,
                range: to_lsp_range(spec.ident.pos.range()),
                selection_range: to_lsp_range(spec.ident.pos.range()),
                children: None,
            },
            Attribute::Declaration(decl) => DocumentSymbol {
                name: decl.ident.item.name_utf8(),
                detail: Some(String::from("attribute")),
                kind: SymbolKind::Property,
                deprecated: None,
                range: to_lsp_range(decl.ident.pos.range()),
                selection_range: to_lsp_range(decl.ident.pos.range()),
                children: None,
            },
        }
    }
}

impl HasDocumentSymbol for AliasDeclaration {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: designator_name_to_string(&self.designator.item),
            detail: Some(String::from("alias")),
            kind: SymbolKind::Interface,
            deprecated: None,
            range: to_lsp_range(self.designator.pos.range()),
            selection_range: to_lsp_range(self.designator.pos.range()),
            children: None,
        }
    }
}

impl HasDocumentSymbol for ConfigurationSpecification {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: String::from("configuration"),
            detail: None,
            kind: SymbolKind::Constructor,
            deprecated: None,
            range: to_lsp_range(self.spec.component_name.pos.range()),
            selection_range: to_lsp_range(self.spec.component_name.pos.range()),
            children: None,
        }
    }
}

impl HasDocumentSymbol for LabeledConcurrentStatement {
    fn document_symbol(&self) -> DocumentSymbol {
        let mut symbol = match &self.statement {
            ConcurrentStatement::ProcedureCall(stmt) => stmt.document_symbol(),
            ConcurrentStatement::Block(stmt) => stmt.document_symbol(),
            ConcurrentStatement::Process(stmt) => stmt.document_symbol(),
            ConcurrentStatement::Assert(stmt) => stmt.document_symbol(),
            ConcurrentStatement::Assignment(stmt) => stmt.document_symbol(),
            ConcurrentStatement::Instance(stmt) => stmt.document_symbol(),
            ConcurrentStatement::ForGenerate(stmt) => stmt.document_symbol(),
            ConcurrentStatement::IfGenerate(stmt) => stmt.document_symbol(),
            ConcurrentStatement::CaseGenerate(stmt) => stmt.document_symbol(),
        };
        if let Some(ref label) = self.label {
            symbol.detail = Some(symbol.name);
            symbol.name = label.item.name_utf8();
            symbol.range = lsp_types::Range::new(to_lsp_pos(label.pos.start()), symbol.range.end);
            symbol.selection_range = to_lsp_range(label.pos.range());
        }
        symbol
    }
}

impl HasDocumentSymbol for ConcurrentProcedureCall {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: String::from("procedure"),
            detail: None,
            kind: SymbolKind::Method,
            deprecated: None,
            range: to_lsp_range(self.call.name.pos.range()),
            selection_range: to_lsp_range(self.call.name.pos.range()),
            children: None,
        }
    }
}

impl HasDocumentSymbol for BlockStatement {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: String::from("block"),
            detail: None,
            kind: SymbolKind::Module,
            deprecated: None,
            range: lsp_range(&self.block_token, &self.semi_token),
            selection_range: lsp_token_range(&self.block_token),
            children: None,
        }
    }
}

impl HasDocumentSymbol for ProcessStatement {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: String::from("process"),
            detail: None,
            kind: SymbolKind::Event,
            deprecated: None,
            range: lsp_range(&self.start_token, &self.semi_token),
            selection_range: lsp_token_range(&self.start_token),
            children: None,
        }
    }
}

impl HasDocumentSymbol for ConcurrentAssertStatement {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: String::from("assertion"),
            detail: None,
            kind: SymbolKind::Field,
            deprecated: None,
            range: to_lsp_range(self.statement.condition.pos.range()),
            selection_range: to_lsp_range(self.statement.condition.pos.range()),
            children: None,
        }
    }
}

impl HasDocumentSymbol for ConcurrentSignalAssignment {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: String::from("assignment"),
            detail: None,
            kind: SymbolKind::Field,
            deprecated: None,
            range: to_lsp_range(self.target.pos.range()),
            selection_range: to_lsp_range(self.target.pos.range()),
            children: None,
        }
    }
}

impl HasDocumentSymbol for InstantiationStatement {
    fn document_symbol(&self) -> DocumentSymbol {
        match &self.unit {
            InstantiatedUnit::Component(selected_name) => DocumentSymbol {
                name: String::from("component"),
                detail: None,
                kind: SymbolKind::Class,
                deprecated: None,
                range: to_lsp_range(selected_name.pos.range()),
                selection_range: to_lsp_range(selected_name.pos.range()),
                children: None,
            },
            InstantiatedUnit::Entity(selected_name, _) => DocumentSymbol {
                name: String::from("entity"),
                detail: None,
                kind: SymbolKind::Class,
                deprecated: None,
                range: to_lsp_range(selected_name.pos.range()),
                selection_range: to_lsp_range(selected_name.pos.range()),
                children: None,
            },
            InstantiatedUnit::Configuration(selected_name) => DocumentSymbol {
                name: String::from("configuration"),
                detail: None,
                kind: SymbolKind::Class,
                deprecated: None,
                range: to_lsp_range(selected_name.pos.range()),
                selection_range: to_lsp_range(selected_name.pos.range()),
                children: None,
            },
        }
    }
}

impl HasDocumentSymbol for ForGenerateStatement {
    fn document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: String::from("generate"),
            detail: None,
            kind: SymbolKind::Field,
            deprecated: None,
            range: to_lsp_range(self.index_name.pos.range()),
            selection_range: to_lsp_range(self.index_name.pos.range()),
            children: None,
        }
    }
}

impl HasDocumentSymbol for IfGenerateStatement {
    fn document_symbol(&self) -> DocumentSymbol {
        let range = if let Some(cond) = self.conditionals.first() {
            to_lsp_range(cond.condition.pos.range())
        } else {
            NULL_RANGE
        };
        DocumentSymbol {
            name: String::from("generate"),
            detail: None,
            kind: SymbolKind::Field,
            deprecated: None,
            range,
            selection_range: range,
            children: None,
        }
    }
}

impl HasDocumentSymbol for CaseGenerateStatement {
    fn document_symbol(&self) -> DocumentSymbol {
        let range = to_lsp_range(self.expression.pos.range());
        DocumentSymbol {
            name: String::from("generate"),
            detail: None,
            kind: SymbolKind::Field,
            deprecated: None,
            range,
            selection_range: range,
            children: None,
        }
    }
}

fn symbol_kind_from_type_definition(type_definition: &TypeDefinition) -> SymbolKind {
    match type_definition {
        TypeDefinition::Enumeration(_) => SymbolKind::Enum,
        TypeDefinition::Integer(_) => SymbolKind::TypeParameter,
        TypeDefinition::Physical(_) => SymbolKind::TypeParameter,
        TypeDefinition::Array(_, _) => SymbolKind::Array,
        TypeDefinition::Record(_) => SymbolKind::Struct,
        TypeDefinition::Access(_) => SymbolKind::Key,
        TypeDefinition::Incomplete(_) => SymbolKind::TypeParameter,
        TypeDefinition::File(_) => SymbolKind::File,
        TypeDefinition::Protected(_) => SymbolKind::Object,
        TypeDefinition::ProtectedBody(_) => SymbolKind::Object,
        TypeDefinition::Subtype(_) => SymbolKind::TypeParameter,
    }
}

fn name_to_string(name: &Name) -> String {
    match name {
        Name::Designator(designator) => designator_name_to_string(&designator.item),
        Name::Selected(name, designator) => format!(
            "{}.{}",
            name_to_string(&name.item),
            designator_name_to_string(&designator.item.item)
        ),
        Name::SelectedAll(name) => format!("{}.{}", name_to_string(&name.item), "all"),
        _ => String::from(" "),
    }
}

fn designator_name_to_string(designator: &Designator) -> String {
    match designator {
        Designator::Identifier(symbol) => symbol.name_utf8(),
        Designator::OperatorSymbol(symbol) => format!("\"{}\"", symbol.to_string()),
        Designator::Character(character) => {
            format!("'{}'", Latin1String::new(&[character.to_owned()]))
        }
    }
}

fn mode_to_string(mode: Mode) -> String {
    String::from(match mode {
        Mode::In => "in",
        Mode::Out => "out",
        Mode::InOut => "inout",
        Mode::Buffer => "buffer",
        Mode::Linkage => "linkage",
    })
}

const NULL_RANGE: lsp_types::Range = lsp_types::Range {
    start: lsp_types::Position {
        line: 0,
        character: 0,
    },
    end: lsp_types::Position {
        line: 0,
        character: 0,
    },
};

fn to_lsp_pos(position: vhdl_lang::Position) -> lsp_types::Position {
    lsp_types::Position {
        line: position.line as u64,
        character: position.character as u64,
    }
}

fn to_lsp_range(range: vhdl_lang::Range) -> lsp_types::Range {
    lsp_types::Range {
        start: to_lsp_pos(range.start),
        end: to_lsp_pos(range.end),
    }
}

fn lsp_range(from: &KeyWordToken, to: &KeyWordToken) -> lsp_types::Range {
    lsp_types::Range {
        start: to_lsp_pos(from.pos.start()),
        end: to_lsp_pos(to.pos.end()),
    }
}

fn lsp_token_range(token: &KeyWordToken) -> lsp_types::Range {
    lsp_types::Range {
        start: to_lsp_pos(token.pos.start()),
        end: to_lsp_pos(token.pos.end()),
    }
}

fn lsp_range_between(from: &KeyWordToken, to: &KeyWordToken) -> lsp_types::Range {
    lsp_types::Range {
        start: to_lsp_pos(from.pos.end()),
        end: to_lsp_pos(to.pos.start()),
    }
}

fn symbol_kind_from_object_class(object_class: ObjectClass) -> SymbolKind {
    match object_class {
        ObjectClass::Signal => SymbolKind::Field,
        ObjectClass::Constant => SymbolKind::Constant,
        ObjectClass::Variable => SymbolKind::Variable,
        ObjectClass::SharedVariable => SymbolKind::Variable,
    }
}

fn none_if_empty<T>(vec: Vec<T>) -> Option<Vec<T>> {
    if vec.is_empty() {
        None
    } else {
        Some(vec)
    }
}

fn push_context_clause(context_clause: ContextClause, symbols: &mut Vec<DocumentSymbol>) {
    if !context_clause.is_empty() {
        symbols.push(context_clause.document_symbol());
    }
}

fn push_generic_interface_list(
    interface_list: Option<&InterfaceList>,
    symbols: &mut Vec<DocumentSymbol>,
) {
    push_interface_list(interface_list, symbols, "generics", SymbolKind::Constant);
}

fn push_port_interface_list(
    interface_list: Option<&InterfaceList>,
    symbols: &mut Vec<DocumentSymbol>,
) {
    push_interface_list(interface_list, symbols, "ports", SymbolKind::Field);
}

fn push_interface_list(
    interface_list: Option<&InterfaceList>,
    symbols: &mut Vec<DocumentSymbol>,
    name: &str,
    symbol_kind: SymbolKind,
) {
    if let Some(ref list) = interface_list {
        let mut symbol = list.document_symbol();
        symbol.name = String::from(name);
        symbol.kind = symbol_kind;
        symbols.push(symbol);
    }
}

fn push_declarations(
    start_token: &KeyWordToken,
    end_token: &KeyWordToken,
    decl: &[Declaration],
    symbols: &mut Vec<DocumentSymbol>,
) {
    symbols.push(DocumentSymbol {
        name: String::from("declarations"),
        detail: None,
        kind: SymbolKind::Field,
        deprecated: None,
        range: lsp_range_between(start_token, end_token),
        selection_range: lsp_range_between(start_token, end_token),
        children: Some(decl.iter().map(|decl| decl.document_symbol()).collect()),
    });
}

fn push_concurrent_statement_part(
    start_token: &KeyWordToken,
    end_token: &KeyWordToken,
    statements: &[LabeledConcurrentStatement],
    symbols: &mut Vec<DocumentSymbol>,
) {
    symbols.push(DocumentSymbol {
        name: String::from("statements"),
        detail: None,
        kind: SymbolKind::Field,
        deprecated: None,
        range: lsp_range_between(start_token, end_token),
        selection_range: lsp_range_between(start_token, end_token),
        children: Some(
            statements
                .iter()
                .map(|decl| decl.document_symbol())
                .collect(),
        ),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types;
    use pretty_assertions::assert_eq;
    use std::convert::TryInto;
    use std::io::Write;
    use std::path::Path;
    use tempfile::NamedTempFile;
    use vhdl_lang::Source;

    fn parse_str(code: &str) -> DesignFile {
        let mut diagnostics = vec![];
        let source = Source::inline(Path::new("mockpath"), code);
        let parser = VHDLParser::default();
        let design_file = parser.parse_design_source(&source, &mut diagnostics);
        for err in diagnostics.iter() {
            println!("{}", err.show());
        }
        if diagnostics.len() > 0 {
            panic!("Found errors");
        }
        design_file
    }

    fn write_source_file(code: &str) -> (Url, NamedTempFile) {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(code.as_bytes()).unwrap();
        (
            Url::from_file_path(file.path().canonicalize().unwrap()).unwrap(),
            file,
        )
    }

    fn range(code: &str, start: &str, end: &str) -> lsp_types::Range {
        find_range(code, start, 1, end, true)
    }

    fn range_between(code: &str, start: &str, end: &str) -> lsp_types::Range {
        find_range(code, start, 1, end, false)
    }

    fn find_range(
        code: &str,
        start: &str,
        start_occurance: usize,
        end: &str,
        inclusive: bool,
    ) -> lsp_types::Range {
        let mut start_line = 0;
        let mut start_column = 0;
        let mut found_start = false;
        let mut end_line = 0;
        let mut end_column = 0;
        let mut line_number = 0;
        let mut occurance = 0;
        for line in code.lines() {
            if !found_start {
                if let Some(pos) = line.find(start) {
                    occurance += 1;
                    if occurance == start_occurance {
                        start_column = pos;
                        if !inclusive {
                            start_column = start_column + start.len();
                        }
                        start_line = line_number;
                        found_start = true;
                    }
                }
            }
            if found_start {
                if let Some(pos) = line.find(end) {
                    end_column = pos;
                    if inclusive {
                        end_column = end_column + end.len();
                    }
                    end_line = line_number;
                    break;
                }
            }
            line_number += 1;
        }

        lsp_types::Range {
            start: lsp_types::Position {
                line: start_line.try_into().unwrap(),
                character: start_column.try_into().unwrap(),
            },
            end: lsp_types::Position {
                line: end_line.try_into().unwrap(),
                character: end_column.try_into().unwrap(),
            },
        }
    }

    fn range1(code: &str, start: &str) -> lsp_types::Range {
        range(code, start, start)
    }

    fn ieee_context(code: &str) -> DocumentSymbol {
        DocumentSymbol {
            name: String::from("context"),
            detail: None,
            kind: SymbolKind::Namespace,
            deprecated: None,
            range: range(code, "library ieee", "ieee;"),
            selection_range: range(code, "library ieee", "ieee;"),
            children: Some(vec![DocumentSymbol {
                name: String::from("ieee"),
                detail: None,
                kind: SymbolKind::Namespace,
                deprecated: None,
                range: range1(code, "ieee"),
                selection_range: range1(code, "ieee"),
                children: None,
            }]),
        }
    }

    fn simple_generic(code: &str) -> DocumentSymbol {
        DocumentSymbol {
            name: String::from("generics"),
            detail: None,
            kind: SymbolKind::Constant,
            deprecated: None,
            range: range(code, "generic(", ");"),
            selection_range: range1(code, "generic"),
            children: Some(vec![DocumentSymbol {
                name: String::from("g1"),
                detail: None,
                kind: SymbolKind::Constant,
                deprecated: None,
                range: range1(code, "g1"),
                selection_range: range1(code, "g1"),
                children: None,
            }]),
        }
    }

    fn simple_port(code: &str) -> DocumentSymbol {
        DocumentSymbol {
            name: String::from("ports"),
            detail: None,
            kind: SymbolKind::Field,
            deprecated: None,
            range: range(code, "port(", ");"),
            selection_range: range1(code, "port"),
            children: Some(vec![DocumentSymbol {
                name: String::from("p1"),
                detail: Some(String::from(": in")),
                kind: SymbolKind::Field,
                deprecated: None,
                range: range1(code, "p1"),
                selection_range: range1(code, "p1"),
                children: None,
            }]),
        }
    }

    fn simple_declaration(
        code: &str,
        start: &str,
        start_occurance: usize,
        begin_is_end: bool,
    ) -> DocumentSymbol {
        let end = if begin_is_end { "begin" } else { "end" };
        DocumentSymbol {
            name: String::from("declarations"),
            detail: None,
            kind: SymbolKind::Field,
            deprecated: None,
            range: find_range(code, start, start_occurance, end, false),
            selection_range: find_range(code, start, start_occurance, end, false),
            children: Some(vec![DocumentSymbol {
                name: String::from("decl1"),
                detail: None,
                kind: SymbolKind::Field,
                deprecated: None,
                range: range1(code, "decl1"),
                selection_range: range1(code, "decl1"),
                children: None,
            }]),
        }
    }
    fn simple_statement(code: &str, starts_with_begin: bool) -> DocumentSymbol {
        let start = if starts_with_begin { "begin" } else { "end" };
        DocumentSymbol {
            name: String::from("statements"),
            detail: None,
            kind: SymbolKind::Field,
            deprecated: None,
            range: range_between(code, start, "end"),
            selection_range: range_between(code, start, "end"),
            children: Some(vec![DocumentSymbol {
                name: String::from("stmt1"),
                detail: Some(String::from("assignment")),
                kind: SymbolKind::Field,
                deprecated: None,
                range: range(code, "stmt1", "decl1"),
                selection_range: range1(code, "stmt1"),
                children: None,
            }]),
        }
    }

    #[test]
    fn entity_declaration() {
        let code = "
library ieee;

entity ent1 is
    generic(
        g1 : integer := 3
    );
    port(
        p1 : integer
    );
    signal decl1 : integer;
end;
";
        let design_file = parse_str(code);
        let unit = design_file.design_units.first().unwrap();
        assert_eq!(
            unit.document_symbol(),
            DocumentSymbol {
                name: String::from("ent1"),
                detail: Some(String::from("entity")),
                kind: SymbolKind::Interface,
                deprecated: None,
                range: range(code, "library", "end;"),
                selection_range: range1(code, "ent1"),
                children: Some(vec![
                    ieee_context(code),
                    simple_generic(code),
                    simple_port(code),
                    simple_declaration(code, ");", 2, false),
                ]),
            }
        );
    }

    #[test]
    fn configuration_declaration() {
        let code = "
library ieee;
configuration cfg of entity_name is
    for rtl(0)
    end for;
end;
";
        let design_file = parse_str(code);
        let unit = design_file.design_units.first().unwrap();
        assert_eq!(
            unit.document_symbol(),
            DocumentSymbol {
                name: String::from("cfg"),
                detail: Some(String::from("configuration")),
                kind: SymbolKind::Constructor,
                deprecated: None,
                range: range(code, "library", "end;"),
                selection_range: range1(code, "cfg"),
                children: Some(vec![ieee_context(code)]),
            }
        );
    }

    #[test]
    fn package_declaration() {
        let code = "
library ieee;
package pkg is
    generic(
        g1 : integer := 3
    );
    
    signal decl1 : integer;
end;
";
        let design_file = parse_str(code);
        let unit = design_file.design_units.first().unwrap();
        assert_eq!(
            unit.document_symbol(),
            DocumentSymbol {
                name: String::from("pkg"),
                detail: Some(String::from("package")),
                kind: SymbolKind::Package,
                deprecated: None,
                range: range(code, "library", "end;"),
                selection_range: range1(code, "pkg"),
                children: Some(vec![
                    ieee_context(code),
                    simple_generic(code),
                    simple_declaration(code, ");", 1, false),
                ]),
            }
        );
    }

    #[test]
    fn package_instantiation() {
        let code = "
library ieee;
package pkg_inst is new work.pkg
    generic map(
        gen => 1
    );
";
        let design_file = parse_str(code);
        let unit = design_file.design_units.first().unwrap();
        assert_eq!(
            unit.document_symbol(),
            DocumentSymbol {
                name: String::from("pkg_inst"),
                detail: Some(String::from("package instance")),
                kind: SymbolKind::Package,
                deprecated: None,
                range: range(code, "library", ");"),
                selection_range: range1(code, "pkg_inst"),
                children: Some(vec![ieee_context(code)]),
            }
        );
    }

    #[test]
    fn context_declaration() {
        let code = "
context ctx is
    library ieee;
end;
";
        let design_file = parse_str(code);
        let unit = design_file.design_units.first().unwrap();
        assert_eq!(
            unit.document_symbol(),
            DocumentSymbol {
                name: String::from("ctx"),
                detail: Some(String::from("context")),
                kind: SymbolKind::Namespace,
                deprecated: None,
                range: range(code, "context", "end;"),
                selection_range: range1(code, "ctx"),
                children: Some(vec![ieee_context(code)]),
            }
        );
    }

    #[test]
    fn package_body() {
        let code = "
library ieee;
package body pkg is
    signal decl1 : integer;
end;
";
        let design_file = parse_str(code);
        let unit = design_file.design_units.first().unwrap();
        assert_eq!(
            unit.document_symbol(),
            DocumentSymbol {
                name: String::from("pkg"),
                detail: Some(String::from("package body")),
                kind: SymbolKind::Package,
                deprecated: None,
                range: range(code, "library", "end;"),
                selection_range: range1(code, "pkg"),
                children: Some(vec![
                    ieee_context(code),
                    simple_declaration(code, "is", 1, false),
                ]),
            }
        );
    }

    #[test]
    fn architecture_body() {
        let code = "
library ieee;
architecture rtl of ent is
    signal decl1 : integer;
begin
    stmt1: decl1 <= 1;
end;
";
        let design_file = parse_str(code);
        let unit = design_file.design_units.first().unwrap();
        assert_eq!(
            unit.document_symbol(),
            DocumentSymbol {
                name: String::from("rtl"),
                detail: Some(String::from("architecture of ent")),
                kind: SymbolKind::Class,
                deprecated: None,
                range: range(code, "library", "end;"),
                selection_range: range1(code, "rtl"),
                children: Some(vec![
                    ieee_context(code),
                    simple_declaration(code, "is", 1, true),
                    simple_statement(code, true)
                ]),
            }
        );
    }

    #[test]
    fn test_nested_document_symbol_response_from_file() {
        let code = "
library ieee;

entity ent1 is
end entity;

";
        let (source_url, _file) = write_source_file(code);
        assert_eq!(
            nested_document_symbol_response_from_file(&source_url).unwrap(),
            DocumentSymbolResponse::from(vec![DocumentSymbol {
                name: String::from("ent1"),
                detail: Some(String::from("entity")),
                kind: SymbolKind::Interface,
                deprecated: None,
                range: range(code, "library", "end entity;"),
                selection_range: range1(code, "ent1"),
                children: Some(vec![ieee_context(code)]),
            }])
        );
    }
}
