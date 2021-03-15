// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2018, Olof Kraigher olof.kraigher@gmail.com

use super::analyze::*;
use super::region::*;
use super::target::AssignmentType;
use crate::ast::Range;
use crate::ast::*;
use crate::data::*;
use std::sync::Arc;

impl<'a> AnalyzeContext<'a> {
    pub fn lookup_selected(
        &self,
        prefix_pos: &SrcPos,
        prefix: &NamedEntity,
        suffix: &WithPos<WithRef<Designator>>,
    ) -> AnalysisResult<Option<NamedEntities>> {
        match prefix.actual_kind() {
            NamedEntityKind::Library => {
                let library_name = prefix.designator().expect_identifier();
                let named_entity =
                    self.lookup_in_library(library_name, &suffix.pos, suffix.designator())?;

                Ok(Some(NamedEntities::new(named_entity)))
            }

            NamedEntityKind::UninstPackage(..) => Err(AnalysisError::NotFatal(
                invalid_selected_name_prefix(prefix, prefix_pos),
            )),
            NamedEntityKind::Object(ref object) => {
                self.lookup_type_selected(prefix_pos, &object.subtype.type_mark(), suffix)
            }
            NamedEntityKind::ElementDeclaration(ref subtype) => {
                self.lookup_type_selected(prefix_pos, subtype.type_mark(), suffix)
            }
            NamedEntityKind::Package(ref region)
            | NamedEntityKind::PackageInstance(ref region)
            | NamedEntityKind::LocalPackageInstance(ref region) => {
                if let Some(decl) = region.lookup_selected(suffix.designator()) {
                    Ok(Some(decl.clone()))
                } else {
                    Err(no_declaration_within(prefix, suffix).into())
                }
            }
            NamedEntityKind::UnknownAlias => Ok(None),
            _ => Err(invalid_selected_name_prefix(prefix, prefix_pos).into()),
        }
    }

    /// Lookup a selected name when the prefix has type
    pub fn lookup_type_selected(
        &self,
        prefix_pos: &SrcPos,
        prefix_type: &NamedEntity,
        suffix: &WithPos<WithRef<Designator>>,
    ) -> AnalysisResult<Option<NamedEntities>> {
        match prefix_type.actual_kind() {
            NamedEntityKind::RecordType(ref region) => {
                if let Some(decl) = region.lookup_selected(suffix.designator()) {
                    Ok(Some(decl.clone()))
                } else {
                    Err(no_declaration_within(prefix_type, suffix).into())
                }
            }
            NamedEntityKind::UnknownAlias => {
                // @TODO forbid prefix
                Ok(None)
            }
            NamedEntityKind::ProtectedType(region) => {
                if let Some(decl) = region.lookup_selected(suffix.designator()) {
                    Ok(Some(decl.clone()))
                } else {
                    Err(no_declaration_within(prefix_type, suffix).into())
                }
            }
            NamedEntityKind::IncompleteType(full_type_ref) => {
                let full_type = full_type_ref.load();
                if let Some(full_type) = full_type.upgrade() {
                    self.lookup_type_selected(prefix_pos, &full_type, suffix)
                } else {
                    Ok(None)
                }
            }
            NamedEntityKind::Subtype(subtype) => {
                self.lookup_type_selected(prefix_pos, subtype.type_mark(), suffix)
            }
            NamedEntityKind::AccessType(subtype) => {
                self.lookup_type_selected(prefix_pos, subtype.type_mark(), suffix)
            }
            _ => Err(invalid_selected_name_prefix(prefix_type, prefix_pos).into()),
        }
    }

    pub fn resolve_selected_name(
        &self,
        region: &Region<'_>,
        name: &mut WithPos<SelectedName>,
    ) -> AnalysisResult<NamedEntities> {
        match name.item {
            SelectedName::Selected(ref mut prefix, ref mut suffix) => {
                suffix.clear_reference();

                let prefix_ent = self
                    .resolve_selected_name(region, prefix)?
                    .into_non_overloaded();
                if let Ok(prefix_ent) = prefix_ent {
                    if let Some(visible) = self.lookup_selected(&prefix.pos, &prefix_ent, suffix)? {
                        suffix.set_reference(&visible);
                        return Ok(visible);
                    };
                };

                Err(AnalysisError::NotFatal(Diagnostic::error(
                    &prefix.pos,
                    "Invalid prefix for selected name",
                )))
            }
            SelectedName::Designator(ref mut designator) => {
                designator.clear_reference();
                let visible = region.lookup_within(&name.pos, designator.designator())?;
                designator.set_reference(&visible);
                Ok(visible)
            }
        }
    }

    pub fn resolve_name(
        &self,
        region: &Region<'_>,
        name_pos: &SrcPos,
        name: &mut Name,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalResult<Option<NamedEntities>> {
        match name {
            Name::Selected(prefix, suffix) => {
                suffix.clear_reference();

                match self.resolve_name(region, &prefix.pos, &mut prefix.item, diagnostics)? {
                    Some(NamedEntities::Single(ref named_entity)) => {
                        match self.lookup_selected(&prefix.pos, named_entity, suffix) {
                            Ok(Some(visible)) => {
                                suffix.set_reference(&visible);
                                Ok(Some(visible))
                            }
                            Ok(None) => Ok(None),
                            Err(err) => {
                                err.add_to(diagnostics)?;
                                Ok(None)
                            }
                        }
                    }
                    Some(NamedEntities::Overloaded(..)) => Ok(None),
                    None => Ok(None),
                }
            }

            Name::SelectedAll(prefix) => {
                self.resolve_name(region, &prefix.pos, &mut prefix.item, diagnostics)?;

                Ok(None)
            }
            Name::Designator(designator) => {
                designator.clear_reference();
                match region.lookup_within(name_pos, designator.designator()) {
                    Ok(visible) => {
                        designator.set_reference(&visible);
                        Ok(Some(visible))
                    }
                    Err(diagnostic) => {
                        diagnostics.push(diagnostic);
                        Ok(None)
                    }
                }
            }
            Name::Indexed(ref mut prefix, ref mut exprs) => {
                self.resolve_name(region, &prefix.pos, &mut prefix.item, diagnostics)?;
                for expr in exprs.iter_mut() {
                    self.analyze_expression(region, expr, diagnostics)?;
                }
                Ok(None)
            }

            Name::Slice(ref mut prefix, ref mut drange) => {
                self.resolve_name(region, &prefix.pos, &mut prefix.item, diagnostics)?;
                self.analyze_discrete_range(region, drange.as_mut(), diagnostics)?;
                Ok(None)
            }
            Name::Attribute(ref mut attr) => {
                self.analyze_attribute_name(region, attr, diagnostics)?;
                Ok(None)
            }
            Name::FunctionCall(ref mut fcall) => {
                self.analyze_function_call(region, fcall, diagnostics)?;
                Ok(None)
            }
            Name::External(ref mut ename) => {
                let ExternalName { subtype, .. } = ename.as_mut();
                self.analyze_subtype_indication(region, subtype, diagnostics)?;
                Ok(None)
            }
        }
    }

    pub fn resolve_non_overloaded(
        &self,
        region: &Region<'_>,
        name: &mut WithPos<SelectedName>,
        kind_ok: &impl Fn(&NamedEntityKind) -> bool,
        expected: &str,
    ) -> AnalysisResult<Arc<NamedEntity>> {
        match self
            .resolve_selected_name(region, name)?
            .into_non_overloaded()
        {
            Ok(ent) => {
                if kind_ok(ent.actual_kind()) {
                    Ok(ent)
                } else {
                    let mut error = Diagnostic::error(
                        name.suffix_pos(),
                        format!("Expected {}, got {}", expected, ent.describe()),
                    );
                    if let Some(pos) = ent.decl_pos() {
                        error.add_related(pos, "Defined here");
                    }
                    Err(AnalysisError::NotFatal(error))
                }
            }
            Err(overloaded) => {
                let mut error = Diagnostic::error(
                    name.suffix_pos(),
                    format!("Expected {}, got overloaded name", expected),
                );
                for ent in overloaded.entities() {
                    if let Some(pos) = ent.decl_pos() {
                        error.add_related(pos, "Defined here");
                    }
                }
                Err(AnalysisError::NotFatal(error))
            }
        }
    }

    pub fn resolve_type_mark(
        &self,
        region: &Region<'_>,
        type_mark: &mut WithPos<SelectedName>,
    ) -> AnalysisResult<Arc<NamedEntity>> {
        self.resolve_non_overloaded(region, type_mark, &NamedEntityKind::is_type, "type")
    }

    fn analyze_attribute_name(
        &self,
        region: &Region<'_>,
        attr: &mut AttributeName,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        // @TODO more, attr must be checked inside the region of attributes of prefix
        let AttributeName {
            name,
            signature,
            expr,
            ..
        } = attr;

        self.resolve_name(region, &name.pos, &mut name.item, diagnostics)?;

        if let Some(ref mut signature) = signature {
            if let Err(err) = self.resolve_signature(region, signature) {
                err.add_to(diagnostics)?;
            }
        }
        if let Some(ref mut expr) = expr {
            self.analyze_expression(region, expr, diagnostics)?;
        }
        Ok(())
    }

    pub fn analyze_range(
        &self,
        region: &Region<'_>,
        range: &mut Range,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        match range {
            Range::Range(ref mut constraint) => {
                self.analyze_expression(region, &mut constraint.left_expr, diagnostics)?;
                self.analyze_expression(region, &mut constraint.right_expr, diagnostics)?;
            }
            Range::Attribute(ref mut attr) => {
                self.analyze_attribute_name(region, attr, diagnostics)?
            }
        }
        Ok(())
    }

    pub fn analyze_discrete_range(
        &self,
        region: &Region<'_>,
        drange: &mut DiscreteRange,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        match drange {
            DiscreteRange::Discrete(ref mut type_mark, ref mut range) => {
                if let Err(err) = self.resolve_type_mark(region, type_mark) {
                    err.add_to(diagnostics)?;
                }
                if let Some(ref mut range) = range {
                    self.analyze_range(region, range, diagnostics)?;
                }
            }
            DiscreteRange::Range(ref mut range) => {
                self.analyze_range(region, range, diagnostics)?;
            }
        }
        Ok(())
    }

    pub fn analyze_choices(
        &self,
        region: &Region<'_>,
        choices: &mut Vec<Choice>,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        for choice in choices.iter_mut() {
            match choice {
                Choice::Expression(ref mut expr) => {
                    self.analyze_expression(region, expr, diagnostics)?;
                }
                Choice::DiscreteRange(ref mut drange) => {
                    self.analyze_discrete_range(region, drange, diagnostics)?;
                }
                Choice::Others => {}
            }
        }
        Ok(())
    }

    pub fn analyze_expression(
        &self,
        region: &Region<'_>,
        expr: &mut WithPos<Expression>,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        self.analyze_expression_pos(region, &expr.pos, &mut expr.item, diagnostics)
    }

    pub fn analyze_waveform(
        &self,
        region: &Region<'_>,
        wavf: &mut Waveform,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        match wavf {
            Waveform::Elements(ref mut elems) => {
                for elem in elems.iter_mut() {
                    let WaveformElement { value, after } = elem;
                    self.analyze_expression(region, value, diagnostics)?;
                    if let Some(expr) = after {
                        self.analyze_expression(region, expr, diagnostics)?;
                    }
                }
            }
            Waveform::Unaffected => {}
        }
        Ok(())
    }

    pub fn resolve_formal(
        &self,
        region: &Region<'_>,
        list_region: &Region<'_>,
        formal_pos: &SrcPos,
        formal: &mut Name,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalResult<Option<NamedEntities>> {
        // LRM 6.5.7 Association lists
        // @TODO Mode in does not allow conversion function nor type conversion
        match formal {
            Name::Designator(designator) => {
                designator.clear_reference();
                if let Some(visible) = list_region.lookup_immediate(designator.designator()) {
                    designator.set_reference(visible);
                    Ok(Some(visible.clone()))
                } else {
                    diagnostics.push(Diagnostic::error(
                        formal_pos,
                        format!("No declaration of '{}'", designator),
                    ));
                    Ok(None)
                }
            }
            Name::Selected(..) => self.resolve_name(list_region, formal_pos, formal, diagnostics),
            Name::Indexed(ref mut prefix, ref mut exprs) => {
                self.resolve_name(list_region, &prefix.pos, &mut prefix.item, diagnostics)?;
                for expr in exprs.iter_mut() {
                    self.analyze_expression(region, expr, diagnostics)?;
                }
                Ok(None)
            }
            Name::Slice(ref mut prefix, ref mut drange) => {
                self.resolve_name(list_region, &prefix.pos, &mut prefix.item, diagnostics)?;
                self.analyze_discrete_range(region, drange.as_mut(), diagnostics)?;
                Ok(None)
            }
            Name::FunctionCall(ref mut fcall) => {
                let FunctionCall { name, parameters } = fcall.as_mut();
                // Disambiguate between indexed name and function call
                if let Name::Designator(designator) = &mut name.item {
                    if let Some(visible) = list_region.lookup_immediate(designator.designator()) {
                        designator.set_reference(visible);
                        self.analyze_assoc_elems(region, None, parameters, diagnostics)?;
                        return Ok(Some(visible.clone()));
                    }
                }

                // @TODO Verify single formal argument
                self.resolve_name(region, &name.pos, &mut name.item, diagnostics)?;
                self.analyze_assoc_elems(list_region, None, parameters, diagnostics)?;
                Ok(None)
            }
            _ => {
                diagnostics.push(Diagnostic::error(formal_pos, "Invalid formal designator"));
                Ok(None)
            }
        }
    }

    pub fn analyze_assoc_elems(
        &self,
        region: &Region<'_>,
        list_region: Option<&Region>,
        elems: &mut Vec<AssociationElement>,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        for AssociationElement { formal, actual } in elems.iter_mut() {
            if let Some(formal) = formal {
                if let Some(list_region) = list_region {
                    self.resolve_formal(
                        region,
                        list_region,
                        &formal.pos,
                        &mut formal.item,
                        diagnostics,
                    )?;
                }
            }
            match actual.item {
                ActualPart::Expression(ref mut expr) => {
                    self.analyze_expression_pos(region, &actual.pos, expr, diagnostics)?;
                }
                ActualPart::Open => {}
            }
        }
        Ok(())
    }

    pub fn analyze_function_call(
        &self,
        region: &Region<'_>,
        fcall: &mut FunctionCall,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        let FunctionCall { name, parameters } = fcall;
        self.resolve_name(region, &name.pos, &mut name.item, diagnostics)?;
        self.analyze_assoc_elems(region, None, parameters, diagnostics)
    }

    pub fn analyze_aggregate(
        &self,
        region: &Region<'_>,
        assocs: &mut Vec<ElementAssociation>,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        for assoc in assocs.iter_mut() {
            match assoc {
                ElementAssociation::Named(ref mut choices, ref mut expr) => {
                    for choice in choices.iter_mut() {
                        match choice {
                            Choice::Expression(..) => {
                                // @TODO could be record field so we cannot do more now
                            }
                            Choice::DiscreteRange(ref mut drange) => {
                                self.analyze_discrete_range(region, drange, diagnostics)?;
                            }
                            Choice::Others => {}
                        }
                    }
                    self.analyze_expression(region, expr, diagnostics)?;
                }
                ElementAssociation::Positional(ref mut expr) => {
                    self.analyze_expression(region, expr, diagnostics)?;
                }
            }
        }
        Ok(())
    }

    fn analyze_qualified_expression(
        &self,
        region: &Region<'_>,
        qexpr: &mut QualifiedExpression,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        let QualifiedExpression { name, expr } = qexpr;
        self.resolve_name(region, &name.pos, &mut name.item, diagnostics)?;
        self.analyze_expression(region, expr, diagnostics)?;
        Ok(())
    }

    fn analyze_expression_pos(
        &self,
        region: &Region<'_>,
        pos: &SrcPos,
        expr: &mut Expression,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        match expr {
            Expression::Binary(_, ref mut left, ref mut right) => {
                self.analyze_expression(region, left, diagnostics)?;
                self.analyze_expression(region, right, diagnostics)
            }
            Expression::Unary(_, ref mut inner) => {
                self.analyze_expression(region, inner, diagnostics)
            }
            Expression::Name(ref mut name) => {
                self.resolve_name(region, pos, name, diagnostics)?;
                Ok(())
            }
            Expression::Aggregate(ref mut assocs) => {
                self.analyze_aggregate(region, assocs, diagnostics)
            }
            Expression::Qualified(ref mut qexpr) => {
                self.analyze_qualified_expression(region, qexpr, diagnostics)
            }

            Expression::New(ref mut alloc) => match alloc.item {
                Allocator::Qualified(ref mut qexpr) => {
                    self.analyze_qualified_expression(region, qexpr, diagnostics)
                }
                Allocator::Subtype(ref mut subtype) => {
                    self.analyze_subtype_indication(region, subtype, diagnostics)
                }
            },
            Expression::Literal(_) => Ok(()),
        }
    }

    pub fn analyze_name_with_target_type(
        &self,
        region: &Region<'_>,
        target_type: &NamedEntity,
        name_pos: &SrcPos,
        name: &mut Name,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        match name {
            Name::Designator(designator) => {
                designator.clear_reference();

                match region.lookup_within(name_pos, designator.designator()) {
                    Ok(entities) => {
                        // If the name is unique it is more helpful to get a reference
                        // Even if the type has a mismatch
                        designator.set_reference(&entities);

                        match entities {
                            NamedEntities::Single(ent) => {
                                designator.set_unique_reference(&ent);
                                if !match_non_overloaded_types(&ent, target_type) {
                                    diagnostics.push(type_mismatch(name_pos, &ent, target_type));
                                }
                            }
                            NamedEntities::Overloaded(overloaded) => {
                                match match_overloaded_types(&overloaded, target_type).as_ref() {
                                    Ok(ent) => {
                                        designator.set_unique_reference(ent);
                                    }
                                    Err(candidates) => {
                                        if candidates.is_empty() {
                                            diagnostics.push(overloaded_type_mismatch(
                                                name_pos,
                                                name,
                                                target_type,
                                            ));
                                        } else {
                                            diagnostics.push(ambiguous_overloaded_type(
                                                name_pos,
                                                name,
                                                candidates.as_ref(),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Err(diagnostic) => {
                        diagnostics.push(diagnostic);
                    }
                }
            }
            Name::Selected(prefix, designator) => {
                designator.clear_reference();

                if let Some(NamedEntities::Single(ref named_entity)) =
                    self.resolve_name(region, &prefix.pos, &mut prefix.item, diagnostics)?
                {
                    match self.lookup_selected(&prefix.pos, named_entity, designator) {
                        Ok(Some(entities)) => {
                            // If the name is unique it is more helpful to get a reference
                            // Even if the type has a mismatch
                            designator.set_reference(&entities);

                            match entities {
                                NamedEntities::Single(ent) => {
                                    designator.set_unique_reference(&ent);
                                    if !match_non_overloaded_types(&ent, target_type) {
                                        diagnostics.push(type_mismatch(
                                            name_pos,
                                            &ent,
                                            target_type,
                                        ));
                                    }
                                }
                                NamedEntities::Overloaded(overloaded) => {
                                    match match_overloaded_types(&overloaded, target_type).as_ref()
                                    {
                                        Ok(ent) => {
                                            designator.set_unique_reference(ent);
                                        }
                                        Err(candidates) => {
                                            if candidates.is_empty() {
                                                diagnostics.push(overloaded_type_mismatch(
                                                    name_pos,
                                                    name,
                                                    target_type,
                                                ));
                                            } else {
                                                diagnostics.push(ambiguous_overloaded_type(
                                                    name_pos,
                                                    name,
                                                    candidates.as_ref(),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Ok(None) => {}
                        Err(err) => {
                            err.add_to(diagnostics)?;
                        }
                    }
                }
            }
            _ => {
                self.resolve_name(region, name_pos, name, diagnostics)?;
            }
        }

        Ok(())
    }

    pub fn analyze_expression_with_target_type(
        &self,
        region: &Region<'_>,
        target_type: &NamedEntity,
        expr: &mut WithPos<Expression>,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        let target_base = target_type.base_type();

        match expr.item {
            Expression::Literal(Literal::AbstractLiteral(AbstractLiteral::Integer(_))) => {
                if !matches!(target_base.kind(), NamedEntityKind::IntegerType(..)) {
                    diagnostics.push(Diagnostic::error(
                        expr,
                        format!("integer literal does not match {}", target_type.describe()),
                    ));
                }
                Ok(())
            }
            Expression::Name(ref mut name) => self.analyze_name_with_target_type(
                region,
                target_type,
                &expr.pos,
                name,
                diagnostics,
            ),
            _ => self.analyze_expression(region, expr, diagnostics),
        }
    }

    // @TODO maybe make generic function for expression/waveform.
    // wait until type checking to see if it makes sense
    pub fn analyze_expr_assignment(
        &self,
        region: &Region<'_>,
        target: &mut WithPos<Target>,
        assignment_type: AssignmentType,
        rhs: &mut AssignmentRightHand<WithPos<Expression>>,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        match rhs {
            AssignmentRightHand::Simple(expr) => {
                self.analyze_target(region, target, assignment_type, diagnostics)?;
                self.analyze_expression(region, expr, diagnostics)?;
            }
            AssignmentRightHand::Conditional(conditionals) => {
                let Conditionals {
                    conditionals,
                    else_item,
                } = conditionals;
                self.analyze_target(region, target, assignment_type, diagnostics)?;
                for conditional in conditionals {
                    let Conditional { condition, item } = conditional;
                    self.analyze_expression(region, item, diagnostics)?;
                    self.analyze_expression(region, condition, diagnostics)?;
                }
                if let Some(expr) = else_item {
                    self.analyze_expression(region, expr, diagnostics)?;
                }
            }
            AssignmentRightHand::Selected(selection) => {
                let Selection {
                    expression,
                    alternatives,
                } = selection;
                self.analyze_expression(region, expression, diagnostics)?;
                // target is located after expression
                self.analyze_target(region, target, assignment_type, diagnostics)?;
                for Alternative { choices, item } in alternatives.iter_mut() {
                    self.analyze_expression(region, item, diagnostics)?;
                    self.analyze_choices(region, choices, diagnostics)?;
                }
            }
        }
        Ok(())
    }

    pub fn analyze_waveform_assignment(
        &self,
        region: &Region<'_>,
        target: &mut WithPos<Target>,
        assignment_type: AssignmentType,
        rhs: &mut AssignmentRightHand<Waveform>,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        match rhs {
            AssignmentRightHand::Simple(wavf) => {
                self.analyze_target(region, target, assignment_type, diagnostics)?;
                self.analyze_waveform(region, wavf, diagnostics)?;
            }
            AssignmentRightHand::Conditional(conditionals) => {
                let Conditionals {
                    conditionals,
                    else_item,
                } = conditionals;
                self.analyze_target(region, target, assignment_type, diagnostics)?;
                for conditional in conditionals {
                    let Conditional { condition, item } = conditional;
                    self.analyze_waveform(region, item, diagnostics)?;
                    self.analyze_expression(region, condition, diagnostics)?;
                }
                if let Some(wavf) = else_item {
                    self.analyze_waveform(region, wavf, diagnostics)?;
                }
            }
            AssignmentRightHand::Selected(selection) => {
                let Selection {
                    expression,
                    alternatives,
                } = selection;
                self.analyze_expression(region, expression, diagnostics)?;
                // target is located after expression
                self.analyze_target(region, target, assignment_type, diagnostics)?;
                for Alternative { choices, item } in alternatives.iter_mut() {
                    self.analyze_waveform(region, item, diagnostics)?;
                    self.analyze_choices(region, choices, diagnostics)?;
                }
            }
        }
        Ok(())
    }
}

/// Match a named entity with a target type
/// Returns a diagnostic in case of mismatch
fn match_non_overloaded_types(ent: &NamedEntity, target_type: &NamedEntity) -> bool {
    if let NamedEntityKind::LoopParameter = ent.actual_kind() {
        // Ignore now to avoid false positives
        true
    } else {
        ent.base_type() == target_type.base_type()
    }
}

/// Match the type of overloaded name with the target type
/// Returns an unique match if found
///  ... or return a vector of ambiguous candidates
//   ... or an empty array if no match
/// An overloaded name is a simple or selected name without any arguments
/// such as an enumeration literal or function call without any arguments
fn match_overloaded_types<'e>(
    overloaded: &'e OverloadedName,
    target_type: &NamedEntity,
) -> Result<&'e Arc<NamedEntity>, Vec<&'e Arc<NamedEntity>>> {
    let target_base = target_type.base_type();
    let mut candidates = Vec::new();

    for ent in overloaded.entities() {
        if let Some(signature) = ent.signature() {
            if signature.match_return_type(target_base)
                && signature.can_be_called_without_parameters()
            {
                candidates.push(ent);
            }
        }
    }

    if candidates.len() == 1 {
        Ok(candidates[0])
    } else {
        candidates.sort_by_key(|ent| ent.decl_pos());
        Err(candidates)
    }
}

fn ambiguous_overloaded_type(
    pos: &SrcPos,
    name: &Name,
    candidates: &[&Arc<NamedEntity>],
) -> Diagnostic {
    let mut diagnostic = Diagnostic::error(pos, format!("ambiguous use of '{}'", name));

    for ent in candidates {
        if let Some(signature) = ent.signature() {
            if let Some(decl_pos) = ent.decl_pos() {
                diagnostic.add_related(
                    decl_pos,
                    format!("migth be {}{}", ent.designator(), signature.describe()),
                )
            }
        }
    }

    diagnostic
}

fn overloaded_type_mismatch(pos: &SrcPos, name: &Name, expected_type: &NamedEntity) -> Diagnostic {
    Diagnostic::error(
        pos,
        format!("'{}' does not match {}", name, expected_type.describe()),
    )
}

fn type_mismatch(pos: &SrcPos, ent: &NamedEntity, expected_type: &NamedEntity) -> Diagnostic {
    Diagnostic::error(
        pos,
        format!(
            "{} does not match {}",
            ent.describe(),
            expected_type.describe()
        ),
    )
}

pub fn invalid_selected_name_prefix(named_entity: &NamedEntity, prefix: &SrcPos) -> Diagnostic {
    Diagnostic::error(
        prefix,
        capitalize(&format!(
            "{} may not be the prefix of a selected name",
            named_entity.describe(),
        )),
    )
}

pub fn no_declaration_within(
    named_entity: &NamedEntity,
    suffix: &WithPos<WithRef<Designator>>,
) -> Diagnostic {
    Diagnostic::error(
        suffix.as_ref(),
        format!(
            "No declaration of '{}' within {}",
            suffix.item,
            named_entity.describe(),
        ),
    )
}
