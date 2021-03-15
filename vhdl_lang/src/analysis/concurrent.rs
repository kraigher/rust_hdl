// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2019, Olof Kraigher olof.kraigher@gmail.com

// These fields are better explicit than .. since we are forced to consider if new fields should be searched
#![allow(clippy::unneeded_field_pattern)]

use super::*;
use crate::ast::*;
use crate::data::*;
use analyze::*;
use region::*;
use target::AssignmentType;

impl<'a> AnalyzeContext<'a> {
    pub fn analyze_concurrent_part(
        &self,
        parent: &mut Region<'_>,
        statements: &mut [LabeledConcurrentStatement],
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        for statement in statements.iter_mut() {
            self.analyze_concurrent_statement(parent, statement, diagnostics)?;
        }

        Ok(())
    }

    fn analyze_concurrent_statement(
        &self,
        parent: &mut Region<'_>,
        statement: &mut LabeledConcurrentStatement,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        if let Some(ref label) = statement.label {
            parent.add(label.clone(), NamedEntityKind::Label, diagnostics);
        }

        match statement.statement {
            ConcurrentStatement::Block(ref mut block) => {
                if let Some(ref mut guard_condition) = block.guard_condition {
                    self.analyze_expression(parent, guard_condition, diagnostics)?;
                }
                let mut region = parent.nested();
                if let Some(ref mut list) = block.header.generic_clause {
                    self.analyze_interface_list(&mut region, list, diagnostics)?;
                }
                if let Some(ref mut list) = block.header.generic_map {
                    self.analyze_assoc_elems(parent, Some(&region), list, diagnostics)?;
                }
                if let Some(ref mut list) = block.header.port_clause {
                    self.analyze_interface_list(&mut region, list, diagnostics)?;
                }
                if let Some(ref mut list) = block.header.port_map {
                    self.analyze_assoc_elems(parent, Some(&region), list, diagnostics)?;
                }
                self.analyze_declarative_part(&mut region, &mut block.decl, diagnostics)?;
                self.analyze_concurrent_part(&mut region, &mut block.statements, diagnostics)?;
            }
            ConcurrentStatement::Process(ref mut process) => {
                let ProcessStatement {
                    postponed: _,
                    sensitivity_list,
                    decl,
                    statements,
                } = process;
                if let Some(sensitivity_list) = sensitivity_list {
                    match sensitivity_list {
                        SensitivityList::Names(names) => {
                            for name in names.iter_mut() {
                                self.resolve_name(parent, &name.pos, &mut name.item, diagnostics)?;
                            }
                        }
                        SensitivityList::All => {}
                    }
                }
                let mut region = parent.nested();
                self.analyze_declarative_part(&mut region, decl, diagnostics)?;
                self.analyze_sequential_part(&mut region, statements, diagnostics)?;
            }
            ConcurrentStatement::ForGenerate(ref mut gen) => {
                let ForGenerateStatement {
                    index_name,
                    discrete_range,
                    body,
                } = gen;
                self.analyze_discrete_range(parent, discrete_range, diagnostics)?;
                let mut region = parent.nested();
                region.add(
                    index_name.clone(),
                    NamedEntityKind::LoopParameter,
                    diagnostics,
                );
                self.analyze_generate_body(&mut region, body, diagnostics)?;
            }
            ConcurrentStatement::IfGenerate(ref mut gen) => {
                let Conditionals {
                    conditionals,
                    else_item,
                } = gen;
                for conditional in conditionals.iter_mut() {
                    let Conditional { condition, item } = conditional;
                    self.analyze_expression(parent, condition, diagnostics)?;
                    let mut region = parent.nested();
                    self.analyze_generate_body(&mut region, item, diagnostics)?;
                }
                if let Some(ref mut else_item) = else_item {
                    let mut region = parent.nested();
                    self.analyze_generate_body(&mut region, else_item, diagnostics)?;
                }
            }
            ConcurrentStatement::CaseGenerate(ref mut gen) => {
                for alternative in gen.alternatives.iter_mut() {
                    let mut region = parent.nested();
                    self.analyze_generate_body(&mut region, &mut alternative.item, diagnostics)?;
                }
            }
            ConcurrentStatement::Instance(ref mut instance) => {
                self.analyze_instance(parent, instance, diagnostics)?;
            }
            ConcurrentStatement::Assignment(ref mut assign) => {
                // @TODO more delaymechanism
                let ConcurrentSignalAssignment { target, rhs, .. } = assign;
                self.analyze_waveform_assignment(
                    parent,
                    target,
                    AssignmentType::Signal,
                    rhs,
                    diagnostics,
                )?;
            }
            ConcurrentStatement::ProcedureCall(ref mut pcall) => {
                let ConcurrentProcedureCall {
                    call,
                    postponed: _postponed,
                } = pcall;
                self.analyze_function_call(parent, call, diagnostics)?;
            }
            ConcurrentStatement::Assert(ref mut assert) => {
                let ConcurrentAssertStatement {
                    postponed: _postponed,
                    statement:
                        AssertStatement {
                            condition,
                            report,
                            severity,
                        },
                } = assert;
                self.analyze_expression(parent, condition, diagnostics)?;
                if let Some(expr) = report {
                    self.analyze_expression(parent, expr, diagnostics)?;
                }
                if let Some(expr) = severity {
                    self.analyze_expression(parent, expr, diagnostics)?;
                }
            }
        };
        Ok(())
    }

    fn analyze_generate_body(
        &self,
        region: &mut Region<'_>,
        body: &mut GenerateBody,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        let GenerateBody {
            alternative_label,
            decl,
            statements,
        } = body;
        if let Some(label) = alternative_label {
            region.add(label.clone(), NamedEntityKind::Label, diagnostics);
        }
        if let Some(ref mut decl) = decl {
            self.analyze_declarative_part(region, decl, diagnostics)?;
        }
        self.analyze_concurrent_part(region, statements, diagnostics)?;

        Ok(())
    }

    fn analyze_instance(
        &self,
        parent: &Region<'_>,
        instance: &mut InstantiationStatement,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        let unit = match instance.unit {
            // @TODO architecture
            InstantiatedUnit::Entity(ref mut entity_name, ..) => {
                fn is_entity(kind: &NamedEntityKind) -> bool {
                    matches!(kind, NamedEntityKind::Entity(..))
                }

                self.resolve_non_overloaded(parent, entity_name, &is_entity, "entity")
            }
            InstantiatedUnit::Component(ref mut component_name) => {
                fn is_component(kind: &NamedEntityKind) -> bool {
                    matches!(kind, NamedEntityKind::Component)
                }

                self.resolve_non_overloaded(parent, component_name, &is_component, "component")
            }
            InstantiatedUnit::Configuration(ref mut config_name) => {
                fn is_configuration(kind: &NamedEntityKind) -> bool {
                    matches!(kind, NamedEntityKind::Configuration(..))
                }

                self.resolve_non_overloaded(parent, config_name, &is_configuration, "configuration")
            }
        };

        let list_region = match unit {
            Ok(ref unit) => match unit.kind() {
                NamedEntityKind::Entity(list_region) => Some(list_region.as_ref()),
                // @TODO add interface list region to component
                NamedEntityKind::Component => None,
                NamedEntityKind::Configuration(list_region) => Some(list_region.as_ref()),
                _ => None,
            },
            Err(err) => {
                err.add_to(diagnostics)?;
                None
            }
        };

        self.analyze_assoc_elems(parent, list_region, &mut instance.generic_map, diagnostics)?;
        self.analyze_assoc_elems(parent, list_region, &mut instance.port_map, diagnostics)?;

        Ok(())
    }
}
