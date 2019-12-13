// This Source Code Form is subject to the terms of the Mozilla Public
// Lic// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// This Source Code Form is subject to the terms of the Mozilla Public
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2019, Olof Kraigher olof.kraigher@gmail.com

use super::*;

#[must_use]
pub enum SearchResult<T> {
    Found(T),
    NotFound,
}

impl<T> Into<Option<T>> for SearchResult<T> {
    fn into(self) -> Option<T> {
        match self {
            Found(value) => Some(value),
            NotFound => None,
        }
    }
}

#[must_use]
pub enum SearchState<T> {
    Finished(SearchResult<T>),
    NotFinished,
}

pub use SearchResult::*;
pub use SearchState::*;

impl<T> SearchState<T> {
    fn or_else(self, nested_fun: impl FnOnce() -> SearchResult<T>) -> SearchResult<T> {
        match self {
            Finished(result) => result,
            NotFinished => nested_fun(),
        }
    }

    fn or_not_found(self) -> SearchResult<T> {
        self.or_else(|| NotFound)
    }
}

pub trait Searcher<T> {
    fn search_labeled_concurrent_statement(
        &mut self,
        _stmt: &LabeledConcurrentStatement,
    ) -> SearchState<T> {
        NotFinished
    }
    fn search_labeled_sequential_statement(
        &mut self,
        _stmt: &LabeledSequentialStatement,
    ) -> SearchState<T> {
        NotFinished
    }
    fn search_declaration(&mut self, _decl: &Declaration) -> SearchState<T> {
        NotFinished
    }
    fn search_type_declaration(&mut self, _decl: &TypeDeclaration) -> SearchState<T> {
        NotFinished
    }
    fn search_interface_declaration(&mut self, _decl: &InterfaceDeclaration) -> SearchState<T> {
        NotFinished
    }
    fn search_subtype_indication(&mut self, _decl: &SubtypeIndication) -> SearchState<T> {
        NotFinished
    }
    fn search_entity(&mut self, _ent: &EntityUnit) -> SearchState<T> {
        NotFinished
    }
    fn search_designator_ref(
        &mut self,
        _pos: &SrcPos,
        _designator: &WithRef<Designator>,
    ) -> SearchState<T> {
        NotFinished
    }
    fn search_with_pos(&mut self, _pos: &SrcPos) -> SearchState<T> {
        NotFinished
    }
    fn search_source(&mut self, _source: &Source) -> SearchState<T> {
        NotFinished
    }
}

pub trait Search<T> {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T>;
}

#[macro_export]
macro_rules! return_if {
    ($result:expr) => {
        match $result {
            result @ Found(_) => {
                return result;
            }
            _ => {}
        };
    };
}

impl<T, V: Search<T>> Search<T> for Vec<V> {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        for decl in self.iter() {
            return_if!(decl.search(searcher));
        }
        NotFound
    }
}

impl<T, V: Search<T>> Search<T> for Option<V> {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        for decl in self.iter() {
            return_if!(decl.search(searcher));
        }
        NotFound
    }
}

impl<T> Search<T> for LabeledSequentialStatement {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher
            .search_labeled_sequential_statement(self)
            .or_else(|| NotFound)
    }
}

impl<T> Search<T> for GenerateBody {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        return_if!(self.decl.search(searcher));
        self.statements.search(searcher)
    }
}
impl<T> Search<T> for LabeledConcurrentStatement {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher
            .search_labeled_concurrent_statement(self)
            .or_else(|| match self.statement {
                ConcurrentStatement::Block(ref block) => {
                    return_if!(block.decl.search(searcher));
                    block.statements.search(searcher)
                }
                ConcurrentStatement::Process(ref process) => {
                    return_if!(process.decl.search(searcher));
                    process.statements.search(searcher)
                }
                ConcurrentStatement::ForGenerate(ref gen) => gen.body.search(searcher),
                ConcurrentStatement::IfGenerate(ref gen) => {
                    for conditional in gen.conditionals.iter() {
                        return_if!(conditional.item.search(searcher));
                    }
                    gen.else_item.search(searcher)
                }
                ConcurrentStatement::CaseGenerate(ref gen) => {
                    for alternative in gen.alternatives.iter() {
                        return_if!(alternative.item.search(searcher))
                    }
                    NotFound
                }
                // @TODO not searched
                _ => NotFound,
            })
    }
}

impl<T> Search<T> for WithPos<WithRef<Designator>> {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher.search_with_pos(&self.pos).or_else(|| {
            searcher
                .search_designator_ref(&self.pos, &self.item)
                .or_not_found()
        })
    }
}

impl<T> Search<T> for WithPos<SelectedName> {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        match self.item {
            SelectedName::Selected(ref prefix, ref designator) => {
                return_if!(prefix.search(searcher));
                return_if!(designator.search(searcher));
                NotFound
            }
            SelectedName::Designator(ref designator) => searcher
                .search_designator_ref(&self.pos, designator)
                .or_not_found(),
        }
    }
}

impl<T> Search<T> for SubtypeIndication {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher.search_subtype_indication(&self).or_else(|| {
            return_if!(self.type_mark.search(searcher));
            NotFound
        })
    }
}

impl<T> Search<T> for TypeDeclaration {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher.search_type_declaration(self).or_else(|| {
            match self.def {
                TypeDefinition::ProtectedBody(ref body) => {
                    return_if!(body.decl.search(searcher));
                }
                TypeDefinition::Protected(ref prot_decl) => {
                    for item in prot_decl.items.iter() {
                        match item {
                            ProtectedTypeDeclarativeItem::Subprogram(ref subprogram) => {
                                return_if!(subprogram.search(searcher));
                            }
                        }
                    }
                }
                TypeDefinition::Record(ref element_decls) => {
                    for elem in element_decls {
                        return_if!(elem.subtype.search(searcher));
                    }
                }
                TypeDefinition::Access(ref subtype_indication) => {
                    return_if!(subtype_indication.search(searcher));
                }
                TypeDefinition::Array(.., ref subtype_indication) => {
                    return_if!(subtype_indication.search(searcher));
                }
                TypeDefinition::Subtype(ref subtype_indication) => {
                    return_if!(subtype_indication.search(searcher));
                }

                _ => {}
            }
            NotFound
        })
    }
}

impl<T> Search<T> for Declaration {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher.search_declaration(self).or_else(|| {
            match self {
                Declaration::Object(object) => {
                    return_if!(object.subtype_indication.search(searcher))
                }
                Declaration::Type(typ) => return_if!(typ.search(searcher)),
                Declaration::SubprogramBody(body) => {
                    return_if!(body.specification.search(searcher));
                    return_if!(body.declarations.search(searcher));
                }
                Declaration::SubprogramDeclaration(decl) => {
                    return_if!(decl.search(searcher));
                }
                Declaration::Attribute(Attribute::Declaration(decl)) => {
                    return_if!(decl.type_mark.search(searcher));
                }
                Declaration::Alias(decl) => {
                    return_if!(decl.subtype_indication.search(searcher));
                }
                _ => {}
            }
            NotFound
        })
    }
}

impl<T> Search<T> for InterfaceDeclaration {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher.search_interface_declaration(self).or_else(|| {
            match self {
                InterfaceDeclaration::Object(ref decl) => {
                    return_if!(decl.subtype_indication.search(searcher));
                }
                InterfaceDeclaration::Subprogram(ref decl, _) => {
                    return_if!(decl.search(searcher));
                }
                _ => {}
            };
            NotFound
        })
    }
}

impl<T> Search<T> for SubprogramDeclaration {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        match self {
            SubprogramDeclaration::Function(ref decl) => return_if!(decl.search(searcher)),
            SubprogramDeclaration::Procedure(ref decl) => return_if!(decl.search(searcher)),
        }
        NotFound
    }
}

impl<T> Search<T> for ProcedureSpecification {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        self.parameter_list.search(searcher)
    }
}

impl<T> Search<T> for FunctionSpecification {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        return_if!(self.parameter_list.search(searcher));
        self.return_type.search(searcher)
    }
}

impl<T> Search<T> for EntityUnit {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher.search_source(self.source()).or_else(|| {
            searcher.search_entity(self).or_else(|| {
                return_if!(self.unit.generic_clause.search(searcher));
                return_if!(self.unit.port_clause.search(searcher));
                return_if!(self.unit.decl.search(searcher));
                self.unit.statements.search(searcher)
            })
        })
    }
}

impl<T> Search<T> for ArchitectureUnit {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher.search_source(self.source()).or_else(|| {
            return_if!(self.unit.decl.search(searcher));
            self.unit.statements.search(searcher)
        })
    }
}

impl<T> Search<T> for PackageUnit {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher.search_source(self.source()).or_else(|| {
            return_if!(self.unit.generic_clause.search(searcher));
            self.unit.decl.search(searcher)
        })
    }
}

impl<T> Search<T> for PackageBodyUnit {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher
            .search_source(self.source())
            .or_else(|| self.unit.decl.search(searcher))
    }
}

impl<T> Search<T> for PackageInstanceUnit {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher
            .search_source(self.source())
            .or_else(|| self.unit.package_name.search(searcher))
    }
}

impl<T> Search<T> for ConfigurationUnit {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher
            .search_source(self.source())
            .or_else(|| self.unit.entity_name.search(searcher))
    }
}

// Search for reference to declaration/definition at cursor
pub struct ItemAtCursor {
    source: Source,
    cursor: Position,
}

impl ItemAtCursor {
    pub fn new(source: &Source, cursor: Position) -> ItemAtCursor {
        ItemAtCursor {
            source: source.clone(),
            cursor,
        }
    }

    pub fn is_inside(&self, pos: &SrcPos) -> bool {
        // cursor is the gap between character cursor and cursor + 1
        // Thus cursor will match character cursor and cursor + 1
        pos.range.start <= self.cursor && self.cursor <= pos.range.end
    }
}

impl Searcher<SrcPos> for ItemAtCursor {
    fn search_with_pos(&mut self, pos: &SrcPos) -> SearchState<SrcPos> {
        // cursor is the gap between character cursor and cursor + 1
        // Thus cursor will match character cursor and cursor + 1
        if self.is_inside(pos) {
            NotFinished
        } else {
            Finished(NotFound)
        }
    }

    fn search_entity(&mut self, ent: &EntityUnit) -> SearchState<SrcPos> {
        let pos = ent.ident().pos();
        if self.is_inside(pos) {
            Finished(Found(pos.clone()))
        } else {
            NotFinished
        }
    }

    fn search_designator_ref(
        &mut self,
        pos: &SrcPos,
        designator: &WithRef<Designator>,
    ) -> SearchState<SrcPos> {
        if !self.is_inside(pos) {
            Finished(NotFound)
        } else if let Some(ref reference) = designator.reference {
            Finished(Found(reference.clone()))
        } else {
            Finished(NotFound)
        }
    }

    fn search_type_declaration(&mut self, decl: &TypeDeclaration) -> SearchState<SrcPos> {
        if self.is_inside(decl.ident.pos()) {
            Finished(Found(decl.ident.pos().clone()))
        } else {
            Finished(NotFound)
        }
    }

    fn search_source(&mut self, source: &Source) -> SearchState<SrcPos> {
        if source == &self.source {
            NotFinished
        } else {
            Finished(NotFound)
        }
    }
}

// Search for all reference to declaration/defintion
pub struct FindAllReferences {
    decl_pos: SrcPos,
    references: Vec<SrcPos>,
}

impl FindAllReferences {
    pub fn new(decl_pos: &SrcPos) -> FindAllReferences {
        FindAllReferences {
            decl_pos: decl_pos.clone(),
            references: Vec::new(),
        }
    }

    pub fn search(mut self, searchable: &impl Search<()>) -> Vec<SrcPos> {
        let _unnused = searchable.search(&mut self);
        self.references
    }

    fn search_decl_pos(&mut self, decl_pos: &SrcPos) {
        if decl_pos == &self.decl_pos {
            self.references.push(decl_pos.clone());
        }
    }
}

impl Searcher<()> for FindAllReferences {
    fn search_entity(&mut self, ent: &EntityUnit) -> SearchState<()> {
        self.search_decl_pos(ent.ident().pos());
        NotFinished
    }

    fn search_designator_ref(
        &mut self,
        pos: &SrcPos,
        designator: &WithRef<Designator>,
    ) -> SearchState<()> {
        if let Some(ref reference) = designator.reference {
            if reference == &self.decl_pos {
                self.references.push(pos.clone());
            }
        };
        NotFinished
    }
}