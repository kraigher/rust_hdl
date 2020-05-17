// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2019, Olof Kraigher olof.kraigher@gmail.com

#[macro_use]
mod tokens;

mod alias_declaration;
mod attributes;
mod common;
mod component_declaration;
mod concurrent_statement;
mod configuration;
mod context;
mod declarative_part;
mod design_unit;
mod expression;
mod interface_declaration;
mod names;
mod object_declaration;
mod parser;
mod range;
mod sequential_statement;
mod subprogram;
mod subtype_indication;
mod type_declaration;
mod waveform;

#[cfg(test)]
pub mod test;

pub use parser::{ParserResult, VHDLParser};
pub use tokens::{KeyWordToken, Symbols};
