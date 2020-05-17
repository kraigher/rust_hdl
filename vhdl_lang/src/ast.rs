// This Source Code Form is subject to the terms of the Mozilla Public
// Lic// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// This Source Code Form is subject to the terms of the Mozilla Public
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2020, Olof Kraigher olof.kraigher@gmail.com

// Allowing this, since box_patterns are feature gated: https://github.com/rust-lang/rfcs/pull/469
// Track here: https://github.com/rust-lang/rust/issues/29641
#![allow(clippy::large_enum_variant)]

mod display;
mod name_util;

#[macro_use]
mod any_design_unit;

#[macro_use]
pub mod search;

pub use self::display::*;
pub use self::name_util::*;
pub use any_design_unit::*;

use crate::data::*;

pub use crate::data::KeyWordToken;

/// LRM 15.8 Bit string literals
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum BaseSpecifier {
    B,
    O,
    X,
    UB,
    UO,
    UX,
    SB,
    SO,
    SX,
    D,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Unary {
    And,
    Or,
    Nand,
    Nor,
    Xor,
    Xnor,
    Abs,
    Not,
    Minus,
    Plus,
    QueQue, // ?? conditional operator
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Binary {
    And,
    Or,
    Nand,
    Nor,
    Xor,
    Xnor,

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

    SLL,
    SRL,
    SLA,
    SRA,
    ROL,
    ROR,

    Plus,
    Minus,
    Concat,

    Times,
    Div,
    Mod,
    Rem,

    Pow,
}

/// LRM 8.6 Attribute names
#[derive(PartialEq, Debug, Clone)]
pub struct AttributeName {
    pub name: WithPos<Name>,
    pub signature: Option<WithPos<Signature>>,
    pub attr: Ident,
    pub expr: Option<Box<WithPos<Expression>>>,
}

/// LRM 8.7 External names
#[derive(PartialEq, Debug, Clone)]
pub enum ExternalObjectClass {
    Constant,
    Signal,
    Variable,
}

/// LRM 8.7 External names
#[derive(PartialEq, Debug, Clone)]
pub enum ExternalPath {
    Package(WithPos<Name>),
    Absolute(WithPos<Name>),

    // usize field indicates the number of up-levels ('^')
    Relative(WithPos<Name>, usize),
}

/// LRM 8.7 External names
#[derive(PartialEq, Debug, Clone)]
pub struct ExternalName {
    pub class: ExternalObjectClass,
    pub path: WithPos<ExternalPath>,
    pub subtype: SubtypeIndication,
}

/// LRM 8. Names
#[derive(PartialEq, Debug, Clone)]
pub enum Name {
    Designator(WithRef<Designator>),
    Selected(Box<WithPos<Name>>, WithPos<WithRef<Designator>>),
    SelectedAll(Box<WithPos<Name>>),
    Indexed(Box<WithPos<Name>>, Vec<WithPos<Expression>>),
    Slice(Box<WithPos<Name>>, Box<DiscreteRange>),
    Attribute(Box<AttributeName>),
    FunctionCall(Box<FunctionCall>),
    External(Box<ExternalName>),
}

/// LRM 8. Names
/// A subset of a full name allowing only selected name
#[derive(PartialEq, Debug, Clone)]
pub enum SelectedName {
    Designator(WithRef<Designator>),
    Selected(Box<WithPos<SelectedName>>, WithPos<WithRef<Designator>>),
}

/// LRM 9.3.4 Function calls
#[derive(PartialEq, Debug, Clone)]
pub struct FunctionCall {
    pub name: WithPos<Name>,
    pub parameters: Vec<AssociationElement>,
}

/// LRM 9.3.3 Aggregates
#[derive(PartialEq, Debug, Clone)]
pub enum Choice {
    Expression(WithPos<Expression>),
    DiscreteRange(DiscreteRange),
    Others,
}

/// LRM 9.3.3 Aggregates
#[derive(PartialEq, Debug, Clone)]
pub enum ElementAssociation {
    Positional(WithPos<Expression>),
    Named(Vec<Choice>, WithPos<Expression>),
}

/// LRM 6.5.7 Association Lists
#[derive(PartialEq, Debug, Clone)]
pub enum ActualPart {
    Expression(Expression),
    Open,
}

/// LRM 6.5.7 Association Lists
#[derive(PartialEq, Debug, Clone)]
pub struct AssociationElement {
    pub formal: Option<WithPos<Name>>,
    pub actual: WithPos<ActualPart>,
}

/// LRM 15.5 Abstract literals
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum AbstractLiteral {
    Integer(u64),
    Real(f64),
}

/// LRM 15.8 Bit string literals
#[derive(PartialEq, Debug, Clone)]
pub struct BitString {
    pub length: Option<u32>,
    pub base: BaseSpecifier,
    pub value: Latin1String,
}

/// LRM 9.3.2 Literals
#[derive(PartialEq, Debug, Clone)]
pub enum Literal {
    String(Latin1String),
    BitString(BitString),
    Character(u8),
    AbstractLiteral(AbstractLiteral),
    Physical(AbstractLiteral, Symbol),
    Null,
}

/// LRM 9.3.7 Allocators
#[derive(PartialEq, Debug, Clone)]
pub enum Allocator {
    Qualified(QualifiedExpression),
    Subtype(SubtypeIndication),
}

/// LRM 9.3.5 Qualified expressions
#[derive(PartialEq, Debug, Clone)]
pub struct QualifiedExpression {
    pub name: Box<WithPos<Name>>,
    pub expr: Box<WithPos<Expression>>,
}

/// LRM 9. Expressions
#[derive(PartialEq, Debug, Clone)]
pub enum Expression {
    Binary(Binary, Box<WithPos<Expression>>, Box<WithPos<Expression>>),
    Unary(Unary, Box<WithPos<Expression>>),

    /// LRM 9.3.3 Aggregates
    Aggregate(Vec<ElementAssociation>),

    /// LRM 9.3.5 Qualified expressions
    Qualified(Box<QualifiedExpression>),

    /// LRM 8 Names
    Name(Box<Name>),

    /// LRM 9.3.2 Literals
    Literal(Literal),

    /// LRM 9.3.7 Allocators
    New(Box<WithPos<Allocator>>),
}

/// An identifier together with the lexical source location it occurs in.
pub type Ident = WithPos<Symbol>;

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Direction {
    Ascending,
    Descending,
}

/// LRM discrete_range
/// discrete_range ::= discrete_subtype_indication | range
/// range ::=
///     range_attribute_name
///   | simple_expression direction simple_expression

#[derive(PartialEq, Debug, Clone)]
pub enum DiscreteRange {
    Discrete(WithPos<SelectedName>, Option<Range>),
    Range(Range),
}

#[derive(PartialEq, Debug, Clone)]
pub struct RangeConstraint {
    pub direction: Direction,
    pub left_expr: Box<WithPos<Expression>>,
    pub right_expr: Box<WithPos<Expression>>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Range {
    Range(RangeConstraint),
    Attribute(Box<AttributeName>),
}

/// LRM: record_element_constraint
#[derive(PartialEq, Debug, Clone)]
pub struct ElementConstraint {
    pub ident: Ident,
    pub constraint: Box<WithPos<SubtypeConstraint>>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum SubtypeConstraint {
    Range(Range),
    /// Empty Vec means Open
    Array(Vec<DiscreteRange>, Option<Box<WithPos<SubtypeConstraint>>>),
    Record(Vec<ElementConstraint>),
}

/// LRM 6.3 Subtype declarations
#[derive(PartialEq, Debug, Clone)]
pub struct RecordElementResolution {
    pub ident: Ident,
    pub resolution: Box<ResolutionIndication>,
}

/// LRM 6.3 Subtype declarations
#[derive(PartialEq, Debug, Clone)]
pub enum ResolutionIndication {
    FunctionName(WithPos<SelectedName>),
    ArrayElement(WithPos<SelectedName>),
    Record(Vec<RecordElementResolution>),
    Unresolved,
}

/// LRM 6.3 Subtype declarations
#[derive(PartialEq, Debug, Clone)]
pub struct SubtypeIndication {
    pub resolution: ResolutionIndication,
    pub type_mark: WithPos<SelectedName>,
    pub constraint: Option<WithPos<SubtypeConstraint>>,
}

/// LRM 5.3 Array Types
#[derive(PartialEq, Debug, Clone)]
pub enum ArrayIndex {
    /// Unbounded
    /// {identifier} range <>
    IndexSubtypeDefintion(WithPos<SelectedName>),

    /// Constraint
    Discrete(DiscreteRange),
}

/// LRM 5.3.3 Record types
#[derive(PartialEq, Debug, Clone)]
pub struct ElementDeclaration {
    pub ident: Ident,
    pub subtype: SubtypeIndication,
}

/// LRM 5.6.2 Protected type declarations
#[derive(PartialEq, Debug, Clone)]
pub enum ProtectedTypeDeclarativeItem {
    Subprogram(SubprogramDeclaration),
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum Designator {
    Identifier(Symbol),
    OperatorSymbol(Latin1String),
    Character(u8),
}

pub type Reference = Option<SrcPos>;

/// An item which has a reference to a declaration
#[derive(PartialEq, Debug, Clone)]
pub struct WithRef<T> {
    pub item: T,
    // @TODO just store the positions of the reference for now
    pub reference: Reference,
}

impl<T> WithRef<T> {
    pub fn new(item: T) -> WithRef<T> {
        WithRef {
            item,
            reference: None,
        }
    }

    pub fn reference_positions(&self) -> impl Iterator<Item = &SrcPos> {
        self.reference.iter()
    }
}

/// LRM 6.6 Alias declarations
#[derive(PartialEq, Debug, Clone)]
pub struct AliasDeclaration {
    pub designator: WithPos<Designator>,
    pub subtype_indication: Option<SubtypeIndication>,
    pub name: WithPos<Name>,
    pub signature: Option<WithPos<Signature>>,
}

/// LRM 6.7 Attribute declarations
#[derive(PartialEq, Debug, Clone)]
pub struct AttributeDeclaration {
    pub ident: Ident,
    pub type_mark: WithPos<SelectedName>,
}

/// LRM 7.2 Attribute specification
#[derive(PartialEq, Debug, Clone)]
pub struct EntityTag {
    pub designator: WithPos<Designator>,
    pub signature: Option<WithPos<Signature>>,
}

/// LRM 7.2 Attribute specification
#[derive(PartialEq, Debug, Clone)]
pub enum EntityName {
    Name(EntityTag),
    All,
    Others,
}

/// LRM 7.2 Attribute specification
// @TODO there are more classes
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum EntityClass {
    Entity,
    Architecture,
    Configuration,
    Package,
    Signal,
    Variable,
    Procedure,
    Function,
    Component,
    Constant,
    Type,
    Label,
}

/// LRM 7.2 Attribute specification
#[derive(PartialEq, Debug, Clone)]
pub struct AttributeSpecification {
    pub ident: Ident,
    pub entity_name: EntityName,
    pub entity_class: EntityClass,
    pub expr: WithPos<Expression>,
}

/// LRM 7.2 Attribute specification
#[derive(PartialEq, Debug, Clone)]
pub enum Attribute {
    Specification(AttributeSpecification),
    Declaration(AttributeDeclaration),
}

/// LRM 5.6.2 Protected type declarations
#[derive(PartialEq, Debug, Clone)]
pub struct ProtectedTypeDeclaration {
    pub items: Vec<ProtectedTypeDeclarativeItem>,
}

/// LRM 5.6.3 Protected type bodies
#[derive(PartialEq, Debug, Clone)]
pub struct ProtectedTypeBody {
    pub type_reference: Reference,
    pub decl: Vec<Declaration>,
}

/// LRM 5.4.2 Physical type declaration
#[derive(PartialEq, Debug, Clone)]
pub struct PhysicalTypeDeclaration {
    pub range: Range,
    pub primary_unit: Ident,
    pub secondary_units: Vec<(Ident, Literal)>,
}

/// LRM 5.2.2 Enumeration types
#[derive(PartialEq, Debug, Clone)]
pub enum EnumerationLiteral {
    Identifier(Symbol),
    Character(u8),
}

/// LRM 5 Types
#[derive(PartialEq, Debug, Clone)]
pub enum TypeDefinition {
    /// LRM 5.2 Scalar Types
    /// LRM 5.2.2 Enumeration types
    Enumeration(Vec<WithPos<EnumerationLiteral>>),
    /// LRM 5.2.3 Integer types
    Integer(Range),
    /// LRM 5.2.4 Physical types
    Physical(PhysicalTypeDeclaration),
    // @TODO floating
    /// LRM 5.3 Composite Types
    /// LRM 5.3.2 Array types
    Array(Vec<ArrayIndex>, SubtypeIndication),
    /// LRM 5.3.3 Record types
    Record(Vec<ElementDeclaration>),
    /// LRM 5.4 Access types
    Access(SubtypeIndication),
    /// LRM 5.4.2 Incomplete type declarations
    Incomplete(Reference),
    /// LRM 5.5 File types
    File(WithPos<SelectedName>),
    /// LRM 5.6 Protected types
    Protected(ProtectedTypeDeclaration),
    ProtectedBody(ProtectedTypeBody),
    /// LRM 6.3 Subtype declarations
    Subtype(SubtypeIndication),
}

/// LRM 6.2 Type declarations
#[derive(PartialEq, Debug, Clone)]
pub struct TypeDeclaration {
    pub ident: Ident,
    pub def: TypeDefinition,
}

/// LRM 6.4.2 Object Declarations
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum ObjectClass {
    Signal,
    // @TODO signal_kind
    Constant,
    Variable,
    SharedVariable,
}

#[derive(PartialEq, Debug, Clone)]
pub struct ObjectDeclaration {
    pub class: ObjectClass,
    pub ident: Ident,
    pub subtype_indication: SubtypeIndication,
    pub expression: Option<WithPos<Expression>>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct FileDeclaration {
    pub ident: Ident,
    pub subtype_indication: SubtypeIndication,
    pub open_info: Option<WithPos<Expression>>,
    pub file_name: Option<WithPos<Expression>>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum SubprogramDesignator {
    Identifier(Symbol),
    OperatorSymbol(Latin1String),
}

/// LRM 4.2 Subprogram declaration
#[derive(PartialEq, Debug, Clone)]
pub struct ProcedureSpecification {
    pub designator: WithPos<SubprogramDesignator>,
    pub parameter_list: Vec<InterfaceDeclaration>,
}

/// LRM 4.2 Subprogram declaration
#[derive(PartialEq, Debug, Clone)]
pub struct FunctionSpecification {
    pub pure: bool,
    pub designator: WithPos<SubprogramDesignator>,
    pub parameter_list: Vec<InterfaceDeclaration>,
    pub return_type: WithPos<SelectedName>,
}

/// LRM 4.3 Subprogram bodies
#[derive(PartialEq, Debug, Clone)]
pub struct SubprogramBody {
    pub specification: SubprogramDeclaration,
    pub declarations: Vec<Declaration>,
    pub statements: Vec<LabeledSequentialStatement>,
}

/// LRM 4.5.3 Signatures
#[derive(PartialEq, Debug, Clone)]
pub enum Signature {
    Function(Vec<WithPos<SelectedName>>, WithPos<SelectedName>),
    Procedure(Vec<WithPos<SelectedName>>),
}

#[derive(PartialEq, Debug, Clone)]
pub enum SubprogramDeclaration {
    Procedure(ProcedureSpecification),
    Function(FunctionSpecification),
}

#[derive(PartialEq, Debug, Clone)]
pub struct InterfaceFileDeclaration {
    pub ident: Ident,
    pub subtype_indication: SubtypeIndication,
}

/// LRM 6.5.2 Interface object declarations
#[derive(PartialEq, Debug, Clone)]
pub struct InterfaceObjectDeclaration {
    pub class: ObjectClass,
    pub ident: Ident,
    pub mode: Mode,
    pub subtype_indication: SubtypeIndication,
    pub expression: Option<WithPos<Expression>>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum SubprogramDefault {
    Name(WithPos<SelectedName>),
    Box,
}
/// LRM 6.5.5 Interface package declaration
#[derive(PartialEq, Debug, Clone)]
pub enum InterfacePackageGenericMapAspect {
    Map(Vec<AssociationElement>),
    Box,
    Default,
}

/// LRM 6.5.5 Interface package declaration
#[derive(PartialEq, Debug, Clone)]
pub struct InterfacePackageDeclaration {
    pub ident: Ident,
    pub package_name: WithPos<SelectedName>,
    pub generic_map: InterfacePackageGenericMapAspect,
}

#[derive(PartialEq, Debug, Clone)]
pub enum InterfaceDeclaration {
    Object(InterfaceObjectDeclaration),
    File(InterfaceFileDeclaration),
    Type(Ident),
    /// LRM 6.5.4 Interface subprogram declarations
    Subprogram(SubprogramDeclaration, Option<SubprogramDefault>),
    /// LRM 6.5.5 Interface package declaration
    Package(InterfacePackageDeclaration),
}

#[derive(PartialEq, Debug, Clone)]
pub struct InterfaceList {
    pub items: Vec<InterfaceDeclaration>,

    // Tokens
    pub start_token: KeyWordToken, // port/generiv/parameter
    pub semi_token: KeyWordToken,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Mode {
    In,
    Out,
    InOut,
    Buffer,
    Linkage,
}

/// LRM 6.8 Component declarations
#[derive(PartialEq, Debug, Clone)]
pub struct ComponentDeclaration {
    pub ident: Ident,
    pub generic_list: Vec<InterfaceDeclaration>,
    pub port_list: Vec<InterfaceDeclaration>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Declaration {
    Object(ObjectDeclaration),
    File(FileDeclaration),
    Type(TypeDeclaration),
    Component(ComponentDeclaration),
    Attribute(Attribute),
    Alias(AliasDeclaration),
    SubprogramDeclaration(SubprogramDeclaration),
    SubprogramBody(SubprogramBody),
    Use(WithPos<UseClause>),
    Package(PackageInstantiation),
    Configuration(ConfigurationSpecification),
}

/// LRM 10.2 Wait statement
#[derive(PartialEq, Debug, Clone)]
pub struct WaitStatement {
    pub sensitivity_clause: Vec<WithPos<Name>>,
    pub condition_clause: Option<WithPos<Expression>>,
    pub timeout_clause: Option<WithPos<Expression>>,
}

/// LRM 10.3 Assertion statement
#[derive(PartialEq, Debug, Clone)]
pub struct AssertStatement {
    pub condition: WithPos<Expression>,
    pub report: Option<WithPos<Expression>>,
    pub severity: Option<WithPos<Expression>>,
}

/// LRM 10.4 Report statement
#[derive(PartialEq, Debug, Clone)]
pub struct ReportStatement {
    pub report: WithPos<Expression>,
    pub severity: Option<WithPos<Expression>>,
}

/// LRM 10.5 Signal assignment statement
#[derive(PartialEq, Debug, Clone)]
pub enum Target {
    Name(Name),
    Aggregate(Vec<ElementAssociation>),
}

/// LRM 10.5 Signal assignment statement
#[derive(PartialEq, Debug, Clone)]
pub struct WaveformElement {
    pub value: WithPos<Expression>,
    pub after: Option<WithPos<Expression>>,
}

/// LRM 10.5 Signal assignment statement
#[derive(PartialEq, Debug, Clone)]
pub enum Waveform {
    Elements(Vec<WaveformElement>),
    Unaffected,
}

/// LRM 10.5 Signal assignment statement
#[derive(PartialEq, Debug, Clone)]
pub enum DelayMechanism {
    Transport,
    Inertial { reject: Option<WithPos<Expression>> },
}

/// LRM 10.5 Signal assignment statement
#[derive(PartialEq, Debug, Clone)]
pub struct SignalAssignment {
    pub target: WithPos<Target>,
    pub delay_mechanism: Option<DelayMechanism>,
    pub rhs: AssignmentRightHand<Waveform>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum ForceMode {
    In,
    Out,
}

#[derive(PartialEq, Debug, Clone)]
pub struct SignalForceAssignment {
    pub target: WithPos<Target>,
    pub force_mode: Option<ForceMode>,
    pub rhs: AssignmentRightHand<WithPos<Expression>>,
}

#[derive(PartialEq, Debug, Clone)]
pub struct SignalReleaseAssignment {
    pub target: WithPos<Target>,
    pub force_mode: Option<ForceMode>,
}

/// LRM 10.6 Variable assignment statement
#[derive(PartialEq, Debug, Clone)]
pub struct VariableAssignment {
    pub target: WithPos<Target>,
    pub rhs: AssignmentRightHand<WithPos<Expression>>,
}

/// LRM 10.5 Signal assignment statement
/// LRM 10.6 Variable assignment statement
#[derive(PartialEq, Debug, Clone)]
pub enum AssignmentRightHand<T> {
    Simple(T),
    Conditional(Conditionals<T>),
    Selected(Selection<T>),
}

#[derive(PartialEq, Debug, Clone)]
pub struct Conditional<T> {
    pub condition: WithPos<Expression>,
    pub item: T,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Conditionals<T> {
    pub conditionals: Vec<Conditional<T>>,
    pub else_item: Option<T>,
}

pub type ConditionalExpression = Conditional<WithPos<Expression>>;
pub type ConditionalExpressions = Conditionals<WithPos<Expression>>;

/// LRM 10.8 If statement
pub type IfStatement = Conditionals<Vec<LabeledSequentialStatement>>;

#[derive(PartialEq, Debug, Clone)]
pub struct Alternative<T> {
    pub choices: Vec<Choice>,
    pub item: T,
}

#[derive(PartialEq, Debug, Clone)]
pub struct Selection<T> {
    pub expression: WithPos<Expression>,
    pub alternatives: Vec<Alternative<T>>,
}

/// LRM 10.9 Case statement
#[derive(PartialEq, Debug, Clone)]
pub struct CaseStatement {
    pub is_matching: bool,
    pub expression: WithPos<Expression>,
    pub alternatives: Vec<Alternative<Vec<LabeledSequentialStatement>>>,
}

/// LRM 10.10 Loop statement
#[derive(PartialEq, Debug, Clone)]
pub enum IterationScheme {
    While(WithPos<Expression>),
    For(Ident, DiscreteRange),
}

/// LRM 10.10 Loop statement
#[derive(PartialEq, Debug, Clone)]
pub struct LoopStatement {
    pub iteration_scheme: Option<IterationScheme>,
    pub statements: Vec<LabeledSequentialStatement>,
}

/// LRM 10.11 Next statement
#[derive(PartialEq, Debug, Clone)]
pub struct NextStatement {
    pub loop_label: Option<Ident>,
    pub condition: Option<WithPos<Expression>>,
}

/// LRM 10.12 Exit statement
#[derive(PartialEq, Debug, Clone)]
pub struct ExitStatement {
    pub loop_label: Option<Ident>,
    pub condition: Option<WithPos<Expression>>,
}

/// LRM 10.13 Return statement
#[derive(PartialEq, Debug, Clone)]
pub struct ReturnStatement {
    pub expression: Option<WithPos<Expression>>,
}

/// LRM 10. Sequential statements
#[derive(PartialEq, Debug, Clone)]
pub enum SequentialStatement {
    Wait(WaitStatement),
    Assert(AssertStatement),
    Report(ReportStatement),
    VariableAssignment(VariableAssignment),
    SignalAssignment(SignalAssignment),
    SignalForceAssignment(SignalForceAssignment),
    SignalReleaseAssignment(SignalReleaseAssignment),
    ProcedureCall(FunctionCall),
    If(IfStatement),
    Case(CaseStatement),
    Loop(LoopStatement),
    Next(NextStatement),
    Exit(ExitStatement),
    Return(ReturnStatement),
    Null,
}

/// LRM 10. Sequential statements
#[derive(PartialEq, Debug, Clone)]
pub struct LabeledSequentialStatement {
    pub label: Option<Ident>,
    pub statement: SequentialStatement,
}

/// LRM 11.2 Block statement
#[derive(PartialEq, Debug, Clone)]
pub struct BlockStatement {
    pub guard_condition: Option<WithPos<Expression>>,
    pub header: BlockHeader,
    pub decl: Vec<Declaration>,
    pub statements: Vec<LabeledConcurrentStatement>,

    // Tokens
    pub block_token: KeyWordToken,
    pub semi_token: KeyWordToken,
}

/// LRM 11.2 Block statement
#[derive(PartialEq, Debug, Clone)]
pub struct BlockHeader {
    pub generic_clause: Option<Vec<InterfaceDeclaration>>,
    pub generic_map: Option<Vec<AssociationElement>>,
    pub port_clause: Option<Vec<InterfaceDeclaration>>,
    pub port_map: Option<Vec<AssociationElement>>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum SensitivityList {
    Names(Vec<WithPos<Name>>),
    All,
}

/// LRM 11.3 Process statement
#[derive(PartialEq, Debug, Clone)]
pub struct ProcessStatement {
    pub postponed: bool,
    pub sensitivity_list: Option<SensitivityList>,
    pub decl: Vec<Declaration>,
    pub statements: Vec<LabeledSequentialStatement>,

    // Tokens
    pub start_token: KeyWordToken, // postponed/process
    pub semi_token: KeyWordToken,
}

/// LRM 11.4 Concurrent procedure call statements
#[derive(PartialEq, Debug, Clone)]
pub struct ConcurrentProcedureCall {
    pub postponed: bool,
    pub call: FunctionCall,
}

/// LRM 11.5 Concurrent assertion statements
#[derive(PartialEq, Debug, Clone)]
pub struct ConcurrentAssertStatement {
    pub postponed: bool,
    pub statement: AssertStatement,
}

/// 11.6 Concurrent signal assignment statements
#[derive(PartialEq, Debug, Clone)]
pub struct ConcurrentSignalAssignment {
    pub postponed: bool,
    pub guarded: bool,
    pub target: WithPos<Target>,
    pub delay_mechanism: Option<DelayMechanism>,
    pub rhs: AssignmentRightHand<Waveform>,
}

/// 11.7 Component instantiation statements
#[derive(PartialEq, Debug, Clone)]
pub enum InstantiatedUnit {
    Component(WithPos<SelectedName>),
    Entity(WithPos<SelectedName>, Option<Ident>),
    Configuration(WithPos<SelectedName>),
}

/// 11.7 Component instantiation statements
#[derive(PartialEq, Debug, Clone)]
pub struct InstantiationStatement {
    pub unit: InstantiatedUnit,
    pub generic_map: Vec<AssociationElement>,
    pub port_map: Vec<AssociationElement>,
}

/// 11.8 Generate statements
#[derive(PartialEq, Debug, Clone)]
pub struct GenerateBody {
    pub alternative_label: Option<Ident>,
    pub decl: Option<Vec<Declaration>>,
    pub statements: Vec<LabeledConcurrentStatement>,
}

/// 11.8 Generate statements
#[derive(PartialEq, Debug, Clone)]
pub struct ForGenerateStatement {
    pub index_name: Ident,
    pub discrete_range: DiscreteRange,
    pub body: GenerateBody,
}

/// 11.8 Generate statements
pub type IfGenerateStatement = Conditionals<GenerateBody>;
pub type CaseGenerateStatement = Selection<GenerateBody>;

/// LRM 11. Concurrent statements
#[derive(PartialEq, Debug, Clone)]
pub enum ConcurrentStatement {
    ProcedureCall(ConcurrentProcedureCall),
    Block(BlockStatement),
    Process(ProcessStatement),
    Assert(ConcurrentAssertStatement),
    Assignment(ConcurrentSignalAssignment),
    Instance(InstantiationStatement),
    ForGenerate(ForGenerateStatement),
    IfGenerate(IfGenerateStatement),
    CaseGenerate(CaseGenerateStatement),
}

/// LRM 11. Concurrent statements
#[derive(PartialEq, Debug, Clone)]
pub struct LabeledConcurrentStatement {
    pub label: Option<Ident>,
    pub statement: ConcurrentStatement,
}

/// LRM 13. Design units and their analysis
#[derive(PartialEq, Debug, Clone)]
pub struct LibraryClause {
    pub name_list: Vec<Ident>,
}

/// LRM 12.4. Use clauses
#[derive(PartialEq, Debug, Clone)]
pub struct UseClause {
    pub name_list: Vec<WithPos<Name>>,
}

/// LRM 13.4 Context clauses
#[derive(PartialEq, Debug, Clone)]
pub struct ContextReference {
    pub name_list: Vec<WithPos<Name>>,
}

/// LRM 13.4 Context clauses
#[derive(PartialEq, Debug, Clone)]
pub enum ContextItem {
    Use(UseClause),
    Library(LibraryClause),
    Context(ContextReference),
}

/// LRM 13.4 Context clauses
#[derive(PartialEq, Debug, Clone)]
pub struct ContextDeclaration {
    pub ident: Ident,
    pub items: ContextClause,

    // Tokens
    pub context_token: KeyWordToken,
    pub is_token: KeyWordToken,
    pub end_token: KeyWordToken,
    pub semi_token: KeyWordToken,
}

/// LRM 4.9 Package instatiation declaration
#[derive(PartialEq, Debug, Clone)]
pub struct PackageInstantiation {
    pub context_clause: ContextClause,
    pub ident: Ident,
    pub package_name: WithPos<SelectedName>,
    pub generic_map: Option<Vec<AssociationElement>>,

    // Tokens
    pub package_token: KeyWordToken,
    pub semi_token: KeyWordToken,
}

/// LRM 7.3 Configuration specification
#[derive(PartialEq, Debug, Clone)]
pub enum InstantiationList {
    Labels(Vec<Ident>),
    Others,
    All,
}

/// LRM 7.3.2 Binding indication
#[derive(PartialEq, Debug, Clone)]
pub enum EntityAspect {
    Entity(WithPos<SelectedName>, Option<Ident>),
    Configuration(WithPos<SelectedName>),
    Open,
}

/// LRM 7.3.2 Binding indication
#[derive(PartialEq, Debug, Clone)]
pub struct BindingIndication {
    pub entity_aspect: Option<EntityAspect>,
    pub generic_map: Option<Vec<AssociationElement>>,
    pub port_map: Option<Vec<AssociationElement>>,
}

/// LRM 7.3 Configuration specification
#[derive(PartialEq, Debug, Clone)]
pub struct ComponentSpecification {
    pub instantiation_list: InstantiationList,
    pub component_name: WithPos<SelectedName>,
}

/// LRM 7.3.4 Verification unit binding indication
#[derive(PartialEq, Debug, Clone)]
pub struct VUnitBindingIndication {
    pub vunit_list: Vec<WithPos<Name>>,
}

/// LRM 7.3 Configuration specification
#[derive(PartialEq, Debug, Clone)]
pub struct ConfigurationSpecification {
    pub spec: ComponentSpecification,
    pub bind_ind: BindingIndication,
    pub vunit_bind_inds: Vec<VUnitBindingIndication>,
}

/// LRM 3.4 Configuration declarations
#[derive(PartialEq, Debug, Clone)]
pub enum ConfigurationDeclarativeItem {
    Use(WithPos<UseClause>),
    // @TODO attribute
    // @TODO group
}
/// LRM 3.4 Configuration declarations
#[derive(PartialEq, Debug, Clone)]
pub struct ComponentConfiguration {
    pub spec: ComponentSpecification,
    pub bind_ind: Option<BindingIndication>,
    pub vunit_bind_inds: Vec<VUnitBindingIndication>,
    pub block_config: Option<BlockConfiguration>,
}

/// LRM 3.4 Configuration declarations
#[derive(PartialEq, Debug, Clone)]
pub enum ConfigurationItem {
    Block(BlockConfiguration),
    Component(ComponentConfiguration),
}

/// LRM 3.4 Configuration declarations
#[derive(PartialEq, Debug, Clone)]
pub struct BlockConfiguration {
    pub block_spec: WithPos<Name>,
    pub use_clauses: Vec<UseClause>,
    pub items: Vec<ConfigurationItem>,
}

/// LRM 3.4 Configuration declarations
#[derive(PartialEq, Debug, Clone)]
pub struct ConfigurationDeclaration {
    pub context_clause: ContextClause,
    pub ident: Ident,
    pub entity_name: WithPos<SelectedName>,
    pub decl: Vec<ConfigurationDeclarativeItem>,
    pub vunit_bind_inds: Vec<VUnitBindingIndication>,
    pub block_config: BlockConfiguration,

    // Tokens
    pub configuration_token: KeyWordToken,
    pub is_token: KeyWordToken,
    pub end_token: KeyWordToken,
    pub semi_token: KeyWordToken,
}

/// LRM 3.2 Entity declarations
#[derive(PartialEq, Debug, Clone)]
pub struct EntityDeclaration {
    pub context_clause: ContextClause,
    pub ident: Ident,
    pub generic_clause: Option<InterfaceList>,
    pub port_clause: Option<InterfaceList>,
    pub decl: Vec<Declaration>,
    pub statements: Vec<LabeledConcurrentStatement>,

    // Tokens
    pub entity_token: KeyWordToken,
    pub is_token: KeyWordToken,
    pub begin_token: Option<KeyWordToken>,
    pub end_token: KeyWordToken,
    pub semi_token: KeyWordToken,
}
/// LRM 3.3 Architecture bodies
#[derive(PartialEq, Debug, Clone)]
pub struct ArchitectureBody {
    pub context_clause: ContextClause,
    pub ident: Ident,
    pub entity_name: WithRef<Ident>,
    pub decl: Vec<Declaration>,
    pub statements: Vec<LabeledConcurrentStatement>,

    // Tokens
    pub architecture_token: KeyWordToken,
    pub is_token: KeyWordToken,
    pub begin_token: KeyWordToken,
    pub end_token: KeyWordToken,
    pub semi_token: KeyWordToken,
}

/// LRM 4.7 Package declarations
#[derive(PartialEq, Debug, Clone)]
pub struct PackageDeclaration {
    pub context_clause: ContextClause,
    pub ident: Ident,
    pub generic_clause: Option<InterfaceList>,
    pub decl: Vec<Declaration>,

    // Tokens
    pub package_token: KeyWordToken,
    pub is_token: KeyWordToken,
    pub end_token: KeyWordToken,
    pub semi_token: KeyWordToken,
}

/// LRM 4.8 Package bodies
#[derive(PartialEq, Debug, Clone)]
pub struct PackageBody {
    pub context_clause: ContextClause,
    pub ident: WithRef<Ident>,
    pub decl: Vec<Declaration>,

    // Tokens
    pub package_token: KeyWordToken,
    pub is_token: KeyWordToken,
    pub end_token: KeyWordToken,
    pub semi_token: KeyWordToken,
}

/// LRM 13.1 Design units
#[derive(PartialEq, Debug, Clone)]
pub enum AnyPrimaryUnit {
    /// LRM 3.2 Entity declaration
    Entity(EntityDeclaration),

    /// LRM 3.4 Configuration declarations
    Configuration(ConfigurationDeclaration),

    /// LRM 4.7 Package declarations
    Package(PackageDeclaration),

    /// LRM 4.9 Package instatiation declaration
    PackageInstance(PackageInstantiation),

    /// LRM 13.4 Context clauses
    Context(ContextDeclaration),
}

/// LRM 13.1 Design units
#[derive(PartialEq, Debug, Clone)]
pub enum AnySecondaryUnit {
    /// LRM 3.3 Architecture bodies
    Architecture(ArchitectureBody),

    /// LRM 4.8 Package bodies
    PackageBody(PackageBody),
}

pub type ContextClause = Vec<WithPos<ContextItem>>;

/// LRM 13.1 Design units
#[derive(PartialEq, Debug, Clone)]
pub enum AnyDesignUnit {
    Primary(AnyPrimaryUnit),
    Secondary(AnySecondaryUnit),
}

#[derive(PartialEq, Debug, Clone, Default)]
pub struct DesignFile {
    pub design_units: Vec<AnyDesignUnit>,
}
