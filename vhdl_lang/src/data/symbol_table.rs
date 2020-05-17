// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2020, Olof Kraigher olof.kraigher@gmail.com

use super::latin_1::Latin1String;
use parking_lot::RwLock;
use std::sync::Arc;

use fnv::FnvHashMap;

/// Represents a unique string symbol.
///
/// The `id` field can be used as a fast comparison key for symbols.
/// Two symbols are compared for equality based on VHDL's rules for identifiers:
/// * basic identifiers are compared case-insensitive (LRM 15.4.2)
/// * extended identifiers are compared case-sensitive (LRM 15.4.3)
#[derive(Clone, Debug, Eq)]
pub struct Symbol {
    /// The unique ID of the symbol.
    ///
    /// Note: IDs are not necessarily contiguous.
    pub(crate) id: usize,

    /// The name of the symbol
    name: Arc<Latin1String>,
}

impl Symbol {
    /// Creates a new symbol. The `id` parameter is assumed to be a valid
    /// [`SymbolTable`](struct.SymbolTable.html) ID.
    fn new(id: usize, name: &Arc<Latin1String>) -> Symbol {
        Symbol {
            id,
            name: Arc::clone(name),
        }
    }

    /// Returns the name of the symbol.
    pub fn name(self: &Self) -> &Latin1String {
        self.name.as_ref()
    }

    /// Returns the name of the symbol as a UTF-8 string.
    pub fn name_utf8(self: &Self) -> String {
        self.name.to_string()
    }
}

impl PartialEq for Symbol {
    /// Symbols are compared just based on the `id` field.
    fn eq(self: &Self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name_utf8())
    }
}

impl std::hash::Hash for Symbol {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        self.id.hash(hasher);
    }
}

/// A thread-safe symbol table to keep track of identifiers.
///
/// This table maintains a mapping from symbol strings to
/// [`Symbol`](struct.Symbol.html) objects.
/// Equivalent identifiers get identical IDs.
#[derive(Default)]
pub struct SymbolTable {
    /// Symbol mapping.
    ///
    /// Basic identifiers containing upper-case letters are stored in verbatim
    /// and normalized forms, with distinct [`Symbol`](struct.Symbol.html) objects,
    /// but these objects contain the same ID.
    name_to_symbol: RwLock<FnvHashMap<Arc<Latin1String>, Symbol>>,
}

impl SymbolTable {
    pub fn insert_utf8(&self, name: &str) -> Symbol {
        let name = Latin1String::from_utf8(name).unwrap();
        self.insert(&name)
    }

    #[cfg(test)]
    pub fn insert_extended_utf8(&self, name: &str) -> Symbol {
        let name = Latin1String::from_utf8_unchecked(name);
        self.insert_extended(&name)
    }

    /// Looks up an identifier (basic or extended).
    ///
    /// Returns the corresponding `Symbol` instance if the identifier already exists,
    /// and `None` otherwise.
    pub fn lookup(&self, name: &Latin1String) -> Option<Symbol> {
        let name_to_symbol = self.name_to_symbol.read();
        if let Some(sym) = name_to_symbol.get(name) {
            // Symbol already exists with identical case
            Some(sym.clone())
        } else {
            None
        }
    }

    /// Inserts a basic identifier and returns a corresponding `Symbol` instance.
    pub fn insert(&self, name: &Latin1String) -> Symbol {
        if let Some(symbol) = self.lookup(name) {
            symbol
        } else {
            self.insert_new(name, false)
        }
    }

    /// Inserts an extended identifier and returns a corresponding `Symbol` instance.
    pub fn insert_extended(&self, name: &Latin1String) -> Symbol {
        if let Some(symbol) = self.lookup(name) {
            symbol
        } else {
            self.insert_new(name, true)
        }
    }

    fn insert_new(&self, name: &Latin1String, is_extended: bool) -> Symbol {
        let mut name_to_symbol = self.name_to_symbol.write();

        // Lookup again after taking lock to avoid race-condition where new symbols are created in parallel
        if let Some(sym) = name_to_symbol.get(name) {
            // Symbol already exists with identical case
            return sym.clone();
        }

        debug_assert_eq!(name.bytes.get(0) == Some(&b'\\'), is_extended);
        let name = Arc::from(name.clone());
        if is_extended {
            let id = name_to_symbol.len();
            let sym = Symbol::new(id, &name);
            name_to_symbol.insert(name, sym.clone());
            return sym;
        }

        // Symbol does not exists with the given case, try normalizing case
        let normal_name = Arc::from(name.to_lowercase());

        match name_to_symbol.get(&normal_name).cloned() {
            // Symbol exists in normalized case
            Some(normal_sym) => {
                // Copy id from previous symbol
                // Insert new symbol with given case and return it
                let id = normal_sym.id;
                let sym = Symbol::new(id, &name);
                name_to_symbol.insert(name, sym.clone());
                sym
            }

            // Symbol does not exist
            None => {
                // Create new id
                let id = name_to_symbol.len();

                if normal_name != name {
                    // If symbol is not already normalized case insert it
                    let sym = Symbol::new(id, &normal_name);
                    name_to_symbol.insert(normal_name, sym);
                }

                let sym = Symbol::new(id, &name);
                name_to_symbol.insert(name, sym.clone());
                sym
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_table_insert() {
        let symtab = SymbolTable::default();
        let sym = symtab.insert_utf8("hello");
        assert_eq!(sym.name_utf8(), "hello");
    }

    #[test]
    fn symbols_are_equal() {
        let symtab = SymbolTable::default();
        let sym0 = symtab.insert_utf8("hello");
        let sym1 = symtab.insert_utf8("hello");
        assert_eq!(sym0, sym1);
        assert_eq!(sym0.name_utf8(), "hello");
        assert_eq!(sym1.name_utf8(), "hello");

        let sym0 = symtab.insert_utf8("Hello");
        let sym1 = symtab.insert_utf8("hello");
        assert_eq!(sym0, sym1);
        assert_eq!(sym0.name_utf8(), "Hello");
        assert_eq!(sym1.name_utf8(), "hello");
    }

    #[test]
    fn symbols_are_case_insensitive() {
        let symtab = SymbolTable::default();
        let sym0 = symtab.insert_utf8("Hello");
        let sym1 = symtab.insert_utf8("hello");
        let sym2 = symtab.insert_utf8("heLLo");
        assert_eq!(sym0, sym1);
        assert_eq!(sym0, sym2);
        assert_eq!(sym1, sym2);
        assert_eq!(sym0.name_utf8(), "Hello");
        assert_eq!(sym1.name_utf8(), "hello");
        assert_eq!(sym2.name_utf8(), "heLLo");
    }

    #[test]
    fn extended_identifiers_symbols_are_case_sensitive() {
        let symtab = SymbolTable::default();
        let sym0 = symtab.insert_extended_utf8("\\hello\\");
        let sym1 = symtab.insert_extended_utf8("\\HELLO\\");
        let sym2 = symtab.insert_extended_utf8("\\hello\\");
        assert_ne!(sym0, sym1);
        assert_eq!(sym0, sym2);
        assert_ne!(sym1, sym2);
        assert_eq!(sym0.name_utf8(), "\\hello\\");
        assert_eq!(sym1.name_utf8(), "\\HELLO\\");
        assert_eq!(sym2.name_utf8(), "\\hello\\");
    }

    #[test]
    fn symbols_are_not_equal() {
        let symtab = SymbolTable::default();
        let sym0 = symtab.insert_utf8("hello");
        let sym1 = symtab.insert_utf8("abc");
        assert_ne!(sym0, sym1);
    }
}
