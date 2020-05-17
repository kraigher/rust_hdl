// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2020, Olof Kraigher olof.kraigher@gmail.com

#[macro_use]
extern crate log;

mod document_symbol;
mod rpc_channel;
mod stdio_server;
mod vhdl_server;
pub use crate::stdio_server::start;
