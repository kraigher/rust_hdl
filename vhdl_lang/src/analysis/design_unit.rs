// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2019, Olof Kraigher olof.kraigher@gmail.com

use super::*;
use crate::ast::*;
use crate::data::*;
use analyze::*;
use region::*;
use root::*;
use semantic::{uninstantiated_package_prefix_error, ResolvedName};
use std::sync::Arc;

impl<'a> AnalyzeContext<'a> {
    pub fn analyze_design_unit(
        &self,
        id: EntityId,
        unit: &mut AnyDesignUnit,
        root_region: &mut Region<'_>,
        region: &mut Region<'_>,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        match unit {
            AnyDesignUnit::Primary(unit) => match unit {
                AnyPrimaryUnit::Entity(unit) => {
                    self.analyze_entity(id, unit, root_region, region, diagnostics)
                }
                AnyPrimaryUnit::Configuration(unit) => {
                    self.analyze_configuration(unit, diagnostics)
                }
                AnyPrimaryUnit::Package(unit) => {
                    self.analyze_package(id, unit, root_region, region, diagnostics)
                }
                AnyPrimaryUnit::PackageInstance(unit) => {
                    self.analyze_package_instance(unit, root_region, region, diagnostics)
                }
                AnyPrimaryUnit::Context(unit) => {
                    self.analyze_context(unit, root_region, region, diagnostics)
                }
            },
            AnyDesignUnit::Secondary(unit) => match unit {
                AnySecondaryUnit::Architecture(unit) => {
                    self.analyze_architecture(id, unit, diagnostics)
                }
                AnySecondaryUnit::PackageBody(unit) => self.analyze_package_body(unit, diagnostics),
            },
        }
    }

    fn analyze_entity(
        &self,
        id: EntityId,
        unit: &mut EntityDeclaration,
        root_region: &mut Region<'_>,
        region: &mut Region<'_>,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        *root_region = Region::default();
        self.add_implicit_context_clause(root_region)?;
        self.analyze_context_clause(root_region, &mut unit.context_clause, diagnostics)?;

        let mut primary_region = root_region.nested();

        // Entity name is visible
        primary_region.make_potentially_visible(
            Some(unit.pos()),
            Arc::new(NamedEntity::new_with_id(
                id,
                unit.name(),
                NamedEntityKind::Label,
                Some(unit.pos()),
            )),
        );

        if let Some(ref mut list) = unit.generic_clause {
            self.analyze_interface_list(&mut primary_region, &mut list.items, diagnostics)?;
        }
        if let Some(ref mut list) = unit.port_clause {
            self.analyze_interface_list(&mut primary_region, &mut list.items, diagnostics)?;
        }
        self.analyze_declarative_part(&mut primary_region, &mut unit.decl, diagnostics)?;
        self.analyze_concurrent_part(&mut primary_region, &mut unit.statements, diagnostics)?;

        *region = primary_region.without_parent();

        Ok(())
    }

    fn analyze_configuration(
        &self,
        unit: &mut ConfigurationDeclaration,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        let mut root_region = Region::default();
        self.add_implicit_context_clause(&mut root_region)?;
        self.analyze_context_clause(&mut root_region, &mut unit.context_clause, diagnostics)?;

        match self.lookup_entity_for_configuration(&root_region, unit) {
            Ok(named_entity) => {
                if let Some(primary_pos) = named_entity.decl_pos() {
                    let secondary_pos = unit.pos();
                    if primary_pos.source == secondary_pos.source
                        && primary_pos.start() > secondary_pos.start()
                    {
                        diagnostics.push(Diagnostic::error(
                            secondary_pos,
                            capitalize(&format!(
                                "{} declared before {}",
                                self.current_unit_id().describe(),
                                named_entity.describe()
                            )),
                        ));
                    }
                }
            }
            Err(err) => {
                err.add_to(diagnostics)?;
            }
        };
        Ok(())
    }

    fn analyze_package(
        &self,
        id: EntityId,
        unit: &mut PackageDeclaration,
        root_region: &mut Region<'_>,
        region: &mut Region<'_>,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        *root_region = Region::default();
        self.add_implicit_context_clause(root_region)?;
        self.analyze_context_clause(root_region, &mut unit.context_clause, diagnostics)?;

        let mut primary_region = root_region.nested().in_package_declaration();

        // Package name is visible
        primary_region.make_potentially_visible(
            Some(unit.pos()),
            Arc::new(NamedEntity::new_with_id(
                id,
                unit.name(),
                NamedEntityKind::Label,
                Some(unit.pos()),
            )),
        );

        if let Some(ref mut list) = unit.generic_clause {
            self.analyze_interface_list(&mut primary_region, &mut list.items, diagnostics)?;
        }
        self.analyze_declarative_part(&mut primary_region, &mut unit.decl, diagnostics)?;

        if !self.has_package_body() {
            primary_region.close(diagnostics);
        }

        *region = primary_region.without_parent();

        Ok(())
    }

    fn analyze_package_instance(
        &self,
        unit: &mut PackageInstantiation,
        root_region: &mut Region<'_>,
        region: &mut Region<'_>,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        *root_region = Region::default();
        self.add_implicit_context_clause(root_region)?;
        self.analyze_context_clause(root_region, &mut unit.context_clause, diagnostics)?;

        match self.analyze_package_instance_name(root_region, &mut unit.package_name) {
            Ok(package_region) => {
                *region = (*package_region).clone();
                Ok(())
            }
            Err(AnalysisError::NotFatal(diagnostic)) => {
                diagnostics.push(diagnostic);
                Ok(())
            }
            Err(AnalysisError::Fatal(err)) => Err(err),
        }
    }

    fn analyze_context(
        &self,
        unit: &mut ContextDeclaration,
        root_region: &mut Region<'_>,
        region: &mut Region<'_>,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        *root_region = Region::default();
        self.add_implicit_context_clause(root_region)?;
        let mut primary_region = root_region.nested();
        self.analyze_context_clause(&mut primary_region, &mut unit.items, diagnostics)?;
        *region = primary_region.without_parent();
        Ok(())
    }

    fn analyze_architecture(
        &self,
        id: EntityId,
        unit: &mut ArchitectureBody,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        unit.entity_name.clear_reference();

        let entity = self.lookup_primary_unit(
            unit.primary_ident(),
            PrimaryKind::Entity,
            unit.pos(),
            diagnostics,
        )?;
        // @TODO maybe add more fatal results
        let entity = if let Some(entity) = entity {
            entity
        } else {
            return Ok(());
        };

        unit.entity_name.set_reference_pos(Some(entity.pos()));

        let mut root_region = Region::default().with_parent(&entity.result().root_region);
        self.analyze_context_clause(&mut root_region, &mut unit.context_clause, diagnostics)?;
        let mut region = Region::extend(&entity.result().region, Some(&root_region));

        // Architecture name is visible
        region.make_potentially_visible(
            Some(unit.pos()),
            Arc::new(NamedEntity::new_with_id(
                id,
                unit.name(),
                NamedEntityKind::Label,
                Some(unit.pos()),
            )),
        );

        self.analyze_declarative_part(&mut region, &mut unit.decl, diagnostics)?;
        self.analyze_concurrent_part(&mut region, &mut unit.statements, diagnostics)?;
        region.close(diagnostics);
        Ok(())
    }

    fn analyze_package_body(
        &self,
        unit: &mut PackageBody,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        unit.ident.clear_reference();

        let package = self.lookup_primary_unit(
            unit.primary_ident(),
            PrimaryKind::Package,
            unit.pos(),
            diagnostics,
        )?;
        // @TODO maybe add more fatal results
        let package = if let Some(package) = package {
            package
        } else {
            return Ok(());
        };

        unit.ident.set_reference_pos(Some(package.pos()));
        // @TODO make pattern of primary/secondary extension
        let mut root_region = Region::default().with_parent(&package.result().root_region);
        self.analyze_context_clause(&mut root_region, &mut unit.context_clause, diagnostics)?;

        let mut region = Region::extend(&package.result().region, Some(&root_region));

        self.analyze_declarative_part(&mut region, &mut unit.decl, diagnostics)?;
        region.close(diagnostics);
        Ok(())
    }

    fn lookup_primary_unit(
        &self,
        primary_ident: &Ident,
        primary_kind: PrimaryKind,
        secondary_pos: &SrcPos,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalResult<Option<UnitReadGuard<'a>>> {
        let unit = {
            if let Some(data) = self.get_primary_analysis(
                primary_ident.pos(),
                self.work_library_name(),
                primary_ident.name(),
                primary_kind,
            ) {
                data?
            } else {
                diagnostics.push(Diagnostic::error(
                    primary_ident.pos(),
                    format!(
                        "No {} '{}' within library '{}'",
                        primary_kind.describe(),
                        primary_ident.name(),
                        self.work_library_name()
                    ),
                ));
                return Ok(None);
            }
        };

        let primary_pos = unit.pos();
        if primary_pos.source == secondary_pos.source && primary_pos.start() > secondary_pos.start()
        {
            diagnostics.push(Diagnostic::error(
                secondary_pos,
                format!(
                    "{} declared before {} '{}'",
                    capitalize(&self.current_unit_id().describe()),
                    primary_kind.describe(),
                    unit.name()
                ),
            ));
        }

        Ok(Some(unit))
    }

    fn lookup_entity_for_configuration(
        &self,
        region: &Region<'_>,
        config: &mut ConfigurationDeclaration,
    ) -> AnalysisResult<Arc<NamedEntity>> {
        let ent_name = &mut config.entity_name;

        match ent_name.item {
            // Entitities are implicitly defined for configurations
            // configuration cfg of ent
            SelectedName::Designator(ref mut designator) => {
                match self.lookup_in_library(
                    self.work_library_name(),
                    &ent_name.pos,
                    &designator.item,
                ) {
                    Ok(ent) => {
                        designator.set_unique_reference(&ent);
                        Ok(ent)
                    }
                    Err(err) => {
                        designator.clear_reference();
                        Err(err)
                    }
                }
            }

            // configuration cfg of lib.ent
            SelectedName::Selected(ref mut prefix, ref mut designator) => {
                designator.clear_reference();

                self.resolve_selected_name(&region, prefix)?
                    .into_non_overloaded()
                    .map_err(|ent|
                             AnalysisError::not_fatal_error(
                                 &prefix.pos,
                                 format!("{} does not denote a library", ent.first().describe()),
                             )
                    )
                    .and_then(|library_ent| match library_ent.kind() {
                        NamedEntityKind::Library => {
                            let library_name = library_ent.designator().expect_identifier();
                            if library_name != self.work_library_name() {
                                Err(AnalysisError::not_fatal_error(
                                    &prefix.pos,
                                    format!("Configuration must be within the same library '{}' as the corresponding entity", self.work_library_name()),
                                ))
                            } else {
                                let primary_ent = self.lookup_in_library(library_name, &designator.pos, designator.designator())?;
                                match primary_ent.kind() {
                                    NamedEntityKind::Entity(..) => {
                                        designator.set_unique_reference(&primary_ent);
                                        Ok(
                                            primary_ent,
                                        )

                                    }
                                    _ => {
                                        Err(AnalysisError::not_fatal_error(
                                            designator,
                                            format!("{} does not denote an entity", primary_ent.describe()),
                                        ))
                                    }
                                }
                            }

                        }
                        _ => {
                            Err(AnalysisError::not_fatal_error(
                                &prefix.pos,
                                format!("{} does not denote a library", library_ent.describe())
                            ))
                        }
                    })
            }
        }
    }

    fn resolve_context_item_prefix(
        &self,
        region: &Region<'_>,
        prefix: &mut WithPos<Name>,
    ) -> AnalysisResult<Arc<NamedEntity>> {
        match self.resolve_context_item_name(region, prefix)? {
            UsedNames::Single(visible) => visible.into_non_overloaded().map_err(|_| {
                AnalysisError::not_fatal_error(&prefix, "Invalid prefix of a selected name")
            }),
            UsedNames::AllWithin(..) => Err(AnalysisError::not_fatal_error(
                &prefix,
                "'.all' may not be the prefix of a selected name",
            )),
        }
    }

    fn resolve_context_item_name(
        &self,
        region: &Region<'_>,
        name: &mut WithPos<Name>,
    ) -> AnalysisResult<UsedNames> {
        match &mut name.item {
            Name::Selected(ref mut prefix, ref mut suffix) => {
                suffix.clear_reference();
                let prefix_ent = self.resolve_context_item_prefix(region, prefix)?;

                match self.lookup_selected(&prefix.pos, &prefix_ent, suffix)? {
                    ResolvedName::Known(visible) => {
                        suffix.set_reference(&visible);
                        Ok(UsedNames::Single(visible))
                    }
                    ResolvedName::Unknown => Err(AnalysisError::not_fatal_error(
                        &prefix.pos,
                        "Invalid prefix for selected name",
                    )),
                }
            }

            Name::SelectedAll(prefix) => {
                let prefix_ent = self.resolve_context_item_prefix(region, prefix)?;
                Ok(UsedNames::AllWithin(prefix.pos.clone(), prefix_ent))
            }
            Name::Designator(designator) => {
                designator.clear_reference();
                let visible = region.lookup_within(&name.pos, designator.designator())?;
                designator.set_reference(&visible);
                Ok(UsedNames::Single(visible))
            }
            Name::Indexed(..)
            | Name::Slice(..)
            | Name::Attribute(..)
            | Name::FunctionCall(..)
            | Name::External(..) => Err(AnalysisError::not_fatal_error(
                &name.pos,
                "Invalid selected name",
            )),
        }
    }

    fn analyze_context_clause(
        &self,
        region: &mut Region<'_>,
        context_clause: &mut [WithPos<ContextItem>],
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        for context_item in context_clause.iter_mut() {
            match context_item.item {
                ContextItem::Library(LibraryClause { ref name_list }) => {
                    for library_name in name_list.iter() {
                        if self.work_sym == library_name.item {
                            diagnostics.push(Diagnostic::hint(
                                &library_name,
                                "Library clause not necessary for current working library",
                            ))
                        } else if let Some(library) = self.get_library(&library_name.item) {
                            region.make_potentially_visible(Some(&library_name.pos), library);
                        } else {
                            diagnostics.push(Diagnostic::error(
                                &library_name,
                                format!("No such library '{}'", library_name.item),
                            ));
                        }
                    }
                }
                ContextItem::Use(ref mut use_clause) => {
                    self.analyze_use_clause(region, use_clause, diagnostics)?;
                }
                ContextItem::Context(ContextReference { ref mut name_list }) => {
                    for name in name_list.iter_mut() {
                        match name.item {
                            Name::Selected(..) => {}
                            _ => {
                                diagnostics.push(Diagnostic::error(
                                    &name.pos,
                                    "Context reference must be a selected name",
                                ));
                                continue;
                            }
                        }

                        match self.resolve_context_item_name(&region, name) {
                            Ok(UsedNames::Single(visible)) => {
                                let ent = visible.first();
                                match ent.kind() {
                                    // OK
                                    NamedEntityKind::Context(ref context_region) => {
                                        region.add_context_visibility(
                                            Some(&name.pos),
                                            context_region,
                                        );
                                    }
                                    _ => {
                                        if let Name::Selected(_, ref suffix) = name.item {
                                            diagnostics.push(Diagnostic::error(
                                                &suffix,
                                                format!(
                                                    "{} does not denote a context declaration",
                                                    ent.describe()
                                                ),
                                            ));
                                        }
                                    }
                                }
                            }
                            Ok(UsedNames::AllWithin(..)) => {
                                // Handled above
                            }
                            Err(err) => {
                                err.add_to(diagnostics)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn analyze_use_clause(
        &self,
        region: &mut Region<'_>,
        use_clause: &mut UseClause,
        diagnostics: &mut dyn DiagnosticHandler,
    ) -> FatalNullResult {
        for name in use_clause.name_list.iter_mut() {
            match name.item {
                Name::Selected(..) => {}
                Name::SelectedAll(..) => {}
                _ => {
                    diagnostics.push(Diagnostic::error(
                        &name.pos,
                        "Use clause must be a selected name",
                    ));
                    continue;
                }
            }

            match self.resolve_context_item_name(&region, name) {
                Ok(UsedNames::Single(visible)) => {
                    visible.make_potentially_visible_in(Some(&name.pos), region);
                }
                Ok(UsedNames::AllWithin(visibility_pos, named_entity)) => {
                    match named_entity.kind() {
                        NamedEntityKind::Library => {
                            let library_name = named_entity.designator().expect_identifier();
                            self.use_all_in_library(&name.pos, library_name, region)?;
                        }
                        NamedEntityKind::UninstPackage(..) => {
                            diagnostics.push(uninstantiated_package_prefix_error(
                                &named_entity,
                                &visibility_pos,
                            ));
                        }
                        NamedEntityKind::Package(ref primary_region)
                        | NamedEntityKind::PackageInstance(ref primary_region)
                        | NamedEntityKind::LocalPackageInstance(ref primary_region) => {
                            region.make_all_potentially_visible(Some(&name.pos), primary_region);
                        }
                        _ => {
                            diagnostics.error(visibility_pos, "Invalid prefix for selected name");
                        }
                    }
                }
                Err(err) => {
                    err.add_to(diagnostics)?;
                }
            }
        }

        Ok(())
    }

    /// Returns a reference to the the uninstantiated package
    pub fn analyze_package_instance_name(
        &self,
        region: &Region<'_>,
        package_name: &mut WithPos<SelectedName>,
    ) -> AnalysisResult<Arc<Region<'static>>> {
        let decl = self.resolve_selected_name(region, package_name)?;

        if let NamedEntityKind::UninstPackage(ref package_region) = decl.first_kind() {
            Ok(package_region.clone())
        } else {
            Err(AnalysisError::not_fatal_error(
                &package_name.pos,
                format!(
                    "'{}' is not an uninstantiated generic package",
                    package_name
                ),
            ))
        }
    }
}

pub enum UsedNames {
    /// A single name was used selected
    Single(NamedEntities),
    /// All names within was selected
    /// @TODO add pos for where declaration was made visible into VisibleDeclaration
    AllWithin(SrcPos, Arc<NamedEntity>),
}
