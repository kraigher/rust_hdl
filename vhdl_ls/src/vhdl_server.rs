// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2020, Olof Kraigher olof.kraigher@gmail.com

use lsp_types::*;

use fnv::FnvHashMap;
use std::collections::hash_map::Entry;

use crate::document_symbol::{
    nested_document_symbol_response_from_file, nested_document_symbol_response_from_source,
};
use crate::rpc_channel::{MessageChannel, RpcChannel};
use std::io;
use std::path::{Path, PathBuf};
use vhdl_lang::{Config, Diagnostic, Message, Project, Severity, Source, SrcPos};

pub struct VHDLServer<T: RpcChannel + Clone> {
    rpc_channel: T,
    // To have well defined unit tests that are not affected by environment
    use_external_config: bool,
    server: Option<InitializedVHDLServer<T>>,
}

impl<T: RpcChannel + Clone> VHDLServer<T> {
    pub fn new(rpc_channel: T) -> VHDLServer<T> {
        Self::new_external_config(rpc_channel, true)
    }

    fn new_external_config(rpc_channel: T, use_external_config: bool) -> VHDLServer<T> {
        VHDLServer {
            rpc_channel,
            use_external_config,
            server: None,
        }
    }

    /// Load the vhdl_ls.toml config file from initalizeParams.rootUri
    fn load_root_uri_config(&self, init_params: &InitializeParams) -> io::Result<Config> {
        let root_uri = init_params.root_uri.as_ref().ok_or_else(|| {
            io::Error::new(io::ErrorKind::Other, "initializeParams.rootUri not set")
        })?;

        let root_path = root_uri.to_file_path().map_err(|_| {
            io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "initializeParams.rootUri {:?} not a valid file path",
                    root_uri
                ),
            )
        })?;

        let config_file = root_path.join("vhdl_ls.toml");
        let config = Config::read_file_path(&config_file)?;

        // Log which file was loaded
        self.rpc_channel.push_msg(Message::log(format!(
            "Loaded workspace root configuration file: {}",
            config_file.to_str().unwrap()
        )));

        Ok(config)
    }

    /// Load the configuration or use a default configuration if unsuccessful
    /// Log info/error messages to the client
    fn load_config(&self, init_params: &InitializeParams) -> Config {
        let mut config = Config::default();
        let mut message_chan = MessageChannel::new(&self.rpc_channel);

        if self.use_external_config {
            config.load_external_config(&mut message_chan);
        }

        match self.load_root_uri_config(&init_params) {
            Ok(root_config) => config.append(&root_config, &mut message_chan),
            Err(ref err) => {
                self.rpc_channel.push_msg(Message::error(format!(
                    "Found no vhdl_ls.toml config file in the workspace root path: {}",
                    err
                )));
                self.rpc_channel.push_msg(Message::warning(
                    "Found no library mapping, semantic analysis disabled, will perform syntax checking only",
                ));
            }
        };

        config
    }

    pub fn initialize_request(
        &mut self,
        params: InitializeParams,
    ) -> jsonrpc_core::Result<InitializeResult> {
        let config = self.load_config(&params);
        let (server, result) = InitializedVHDLServer::new(self.rpc_channel.clone(), config, params);
        self.server = Some(server);
        Ok(result)
    }

    pub fn shutdown_server(&mut self, _params: ()) -> jsonrpc_core::Result<()> {
        self.server = None;
        Ok(())
    }

    fn mut_server(&mut self) -> &mut InitializedVHDLServer<T> {
        self.server.as_mut().expect("Expected initialized server")
    }

    pub fn exit_notification(&mut self, _params: ()) {
        match self.server {
            Some(_) => ::std::process::exit(1),
            None => ::std::process::exit(0),
        }
    }

    pub fn initialized_notification(&mut self) {
        self.mut_server().initialized_notification();
    }

    pub fn text_document_did_change_notification(&mut self, params: &DidChangeTextDocumentParams) {
        self.mut_server()
            .text_document_did_change_notification(&params)
    }

    pub fn text_document_did_open_notification(&mut self, params: &DidOpenTextDocumentParams) {
        self.mut_server()
            .text_document_did_open_notification(&params)
    }

    // textDocument/declaration
    pub fn text_document_declaration(
        &mut self,
        params: &TextDocumentPositionParams,
    ) -> Option<Location> {
        self.mut_server().text_document_declaration(&params)
    }

    // textDocument/definition
    pub fn text_document_definition(
        &mut self,
        params: &TextDocumentPositionParams,
    ) -> Option<Location> {
        self.mut_server().text_document_definition(&params)
    }

    // textDocument/references
    pub fn text_document_references(&mut self, params: &ReferenceParams) -> Vec<Location> {
        self.mut_server().text_document_references(&params)
    }

    // textDocument/documentSymbol
    pub fn text_document_document_symbol(
        &mut self,
        params: &DocumentSymbolParams,
    ) -> Option<DocumentSymbolResponse> {
        self.mut_server().text_document_document_symbol(&params)
    }
}

struct InitializedVHDLServer<T: RpcChannel> {
    rpc_channel: T,
    init_params: InitializeParams,
    project: Project,
    files_with_notifications: FnvHashMap<Url, ()>,
}

/// Allow VHDL Server to act as an RpcChannel
impl<T: RpcChannel> RpcChannel for InitializedVHDLServer<T> {
    fn send_notification(
        &self,
        method: impl Into<String>,
        notification: impl serde::ser::Serialize,
    ) {
        self.rpc_channel.send_notification(method, notification);
    }
}

impl<T: RpcChannel + Clone> InitializedVHDLServer<T> {
    pub fn new(
        rpc_channel: T,
        config: Config,
        init_params: InitializeParams,
    ) -> (InitializedVHDLServer<T>, InitializeResult) {
        let project = Project::from_config(&config, &mut MessageChannel::new(&rpc_channel));

        let server = InitializedVHDLServer {
            rpc_channel,
            init_params,
            project,
            files_with_notifications: FnvHashMap::default(),
        };

        let mut capabilities = ServerCapabilities::default();
        capabilities.text_document_sync = Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::Incremental,
        ));
        capabilities.declaration_provider = Some(true);
        capabilities.definition_provider = Some(true);
        capabilities.references_provider = Some(true);
        capabilities.document_symbol_provider = Some(true);
        let result = InitializeResult {
            capabilities,
            server_info: None,
        };

        (server, result)
    }

    pub fn initialized_notification(&mut self) {
        self.publish_diagnostics();
    }

    fn client_supports_related_information(&self) -> bool {
        let try_fun = || {
            self.init_params
                .capabilities
                .text_document
                .as_ref()?
                .publish_diagnostics
                .as_ref()?
                .related_information
        };
        try_fun().unwrap_or(false)
    }

    fn publish_diagnostics(&mut self) {
        let supports_related_information = self.client_supports_related_information();
        let diagnostics = self.project.analyse();
        let diagnostics = {
            if supports_related_information {
                diagnostics
            } else {
                flatten_related(diagnostics)
            }
        };

        let mut files_with_notifications = std::mem::take(&mut self.files_with_notifications);
        for (file_uri, diagnostics) in diagnostics_by_uri(diagnostics).into_iter() {
            let mut lsp_diagnostics = Vec::new();
            for diagnostic in diagnostics {
                lsp_diagnostics.push(to_lsp_diagnostic(diagnostic));
            }

            let publish_diagnostics = PublishDiagnosticsParams {
                uri: file_uri.clone(),
                diagnostics: lsp_diagnostics,
                version: None,
            };

            self.send_notification("textDocument/publishDiagnostics", publish_diagnostics);

            self.files_with_notifications.insert(file_uri.clone(), ());
        }

        for (file_uri, _) in files_with_notifications.drain() {
            // File has no longer any diagnosics, publish empty notification to clear them
            if !self.files_with_notifications.contains_key(&file_uri) {
                let publish_diagnostics = PublishDiagnosticsParams {
                    uri: file_uri.clone(),
                    diagnostics: vec![],
                    version: None,
                };

                self.send_notification("textDocument/publishDiagnostics", publish_diagnostics);
            }
        }
    }

    fn open(&mut self, uri: &Url, code: &str) {
        let file_name = uri_to_file_name(uri);
        if let Some(source) = self.project.get_source(&file_name) {
            source.change(None, &code);
            self.project.update_source(&source);
            self.publish_diagnostics();
        } else {
            self.push_msg(Message::info(format!(
                "Opening file {} that is not part of the project",
                file_name.to_string_lossy()
            )));
            self.project
                .update_source(&Source::inline(&file_name, code));
            self.publish_diagnostics();
        }
    }

    pub fn text_document_did_change_notification(&mut self, params: &DidChangeTextDocumentParams) {
        let file_name = uri_to_file_name(&params.text_document.uri);
        if let Some(source) = self.project.get_source(&file_name) {
            for content_change in params.content_changes.iter() {
                let range = content_change.range.map(from_lsp_range);
                source.change(range.as_ref(), &content_change.text);
            }
            self.project.update_source(&source);
            self.publish_diagnostics();
        } else {
            self.push_msg(Message::error(format!(
                "Changing file {} that is not part of the project",
                file_name.to_string_lossy()
            )));
        }
    }

    pub fn text_document_did_open_notification(&mut self, params: &DidOpenTextDocumentParams) {
        self.open(&params.text_document.uri, &params.text_document.text);
    }

    pub fn text_document_declaration(
        &mut self,
        params: &TextDocumentPositionParams,
    ) -> Option<Location> {
        self.project
            .get_source(&uri_to_file_name(&params.text_document.uri))
            .and_then(|source| {
                self.project
                    .search_reference(&source, from_lsp_pos(params.position))
            })
            .map(|result| srcpos_to_location(&result))
    }

    // Copy goto-declaration for now
    pub fn text_document_definition(
        &mut self,
        params: &TextDocumentPositionParams,
    ) -> Option<Location> {
        self.text_document_declaration(params)
    }

    pub fn text_document_references(&mut self, params: &ReferenceParams) -> Vec<Location> {
        let decl_pos = self
            .project
            .get_source(&uri_to_file_name(
                &params.text_document_position.text_document.uri,
            ))
            .and_then(|source| {
                self.project.search_reference(
                    &source,
                    from_lsp_pos(params.text_document_position.position),
                )
            });

        if let Some(ref decl_pos) = decl_pos {
            self.project
                .find_all_references(decl_pos)
                .iter()
                .map(|pos| srcpos_to_location(pos))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn text_document_document_symbol(
        &mut self,
        params: &DocumentSymbolParams,
    ) -> Option<DocumentSymbolResponse> {
        self.project
            .get_source(&uri_to_file_name(&params.text_document.uri))
            .map(|source| nested_document_symbol_response_from_source(&source))
            .or_else(|| nested_document_symbol_response_from_file(&params.text_document.uri))
    }
}

fn srcpos_to_location(pos: &SrcPos) -> Location {
    let uri = file_name_to_uri(pos.source.file_name());
    Location {
        uri,
        range: to_lsp_range(pos.range()),
    }
}

fn from_lsp_pos(position: lsp_types::Position) -> vhdl_lang::Position {
    vhdl_lang::Position {
        line: position.line as u32,
        character: position.character as u32,
    }
}

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

fn from_lsp_range(range: lsp_types::Range) -> vhdl_lang::Range {
    vhdl_lang::Range {
        start: from_lsp_pos(range.start),
        end: from_lsp_pos(range.end),
    }
}

fn diagnostics_by_uri(diagnostics: Vec<Diagnostic>) -> FnvHashMap<Url, Vec<Diagnostic>> {
    let mut map: FnvHashMap<Url, Vec<Diagnostic>> = FnvHashMap::default();

    for diagnostic in diagnostics {
        let uri = file_name_to_uri(diagnostic.pos.source.file_name());
        match map.entry(uri) {
            Entry::Occupied(mut entry) => entry.get_mut().push(diagnostic),
            Entry::Vacant(entry) => {
                let vec = vec![diagnostic];
                entry.insert(vec);
            }
        }
    }

    map
}

fn flatten_related(diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let mut flat_diagnostics = Vec::new();
    for mut diagnostic in diagnostics {
        flat_diagnostics.extend(diagnostic.drain_related());
        flat_diagnostics.push(diagnostic);
    }
    flat_diagnostics
}

fn file_name_to_uri(file_name: &Path) -> Url {
    // @TODO return error to client
    Url::from_file_path(file_name).unwrap()
}

fn uri_to_file_name(uri: &Url) -> PathBuf {
    // @TODO return error to client
    uri.to_file_path().unwrap()
}

fn to_lsp_diagnostic(diagnostic: Diagnostic) -> lsp_types::Diagnostic {
    let severity = match diagnostic.severity {
        Severity::Error => DiagnosticSeverity::Error,
        Severity::Warning => DiagnosticSeverity::Warning,
        Severity::Info => DiagnosticSeverity::Information,
        Severity::Hint => DiagnosticSeverity::Hint,
    };

    let related_information = if !diagnostic.related.is_empty() {
        let mut related_information = Vec::new();
        for (pos, msg) in diagnostic.related {
            let uri = file_name_to_uri(pos.source.file_name());
            related_information.push(DiagnosticRelatedInformation {
                location: Location {
                    uri: uri.to_owned(),
                    range: to_lsp_range(pos.range()),
                },
                message: msg,
            })
        }
        Some(related_information)
    } else {
        None
    };

    lsp_types::Diagnostic {
        range: to_lsp_range(diagnostic.pos.range()),
        severity: Some(severity),
        code: None,
        source: Some("vhdl ls".to_owned()),
        message: diagnostic.message,
        related_information,
        tags: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc_channel::test_support::*;
    use tempfile;

    fn initialize_server(server: &mut VHDLServer<RpcMock>, root_uri: Url) {
        let capabilities = ClientCapabilities::default();

        #[allow(deprecated)]
        let initialize_params = InitializeParams {
            process_id: None,
            root_path: None,
            root_uri: Some(root_uri),
            initialization_options: None,
            capabilities,
            trace: None,
            workspace_folders: None,
            client_info: None,
        };

        server
            .initialize_request(initialize_params)
            .expect("Should not fail");
        server.initialized_notification();
    }

    fn temp_root_uri() -> (tempfile::TempDir, Url) {
        let tempdir = tempfile::tempdir().unwrap();
        let root_uri = Url::from_file_path(tempdir.path().canonicalize().unwrap()).unwrap();
        (tempdir, root_uri)
    }

    fn expect_loaded_config_messages(mock: &RpcMock, config_uri: &Url) {
        let file_name = config_uri
            .to_file_path()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        mock.expect_message_contains(format!(
            "Loaded workspace root configuration file: {}",
            file_name
        ));
    }

    fn expect_missing_config_messages(mock: &RpcMock) {
        mock.expect_error_contains("Found no vhdl_ls.toml config file in the workspace root path");
        mock.expect_message_contains(
            "Found no library mapping, semantic analysis disabled, will perform syntax checking only",
        );
    }

    /// Create RpcMock and VHDLServer
    fn setup_server() -> (RpcMock, VHDLServer<RpcMock>) {
        let mock = RpcMock::new();
        let server = VHDLServer::new_external_config(mock.clone(), false);
        (mock, server)
    }

    #[test]
    fn initialize() {
        let (mock, mut server) = setup_server();
        let (_tempdir, root_uri) = temp_root_uri();
        expect_missing_config_messages(&mock);
        initialize_server(&mut server, root_uri);
    }

    #[test]
    fn did_open_no_diagnostics() {
        let (mock, mut server) = setup_server();
        let (_tempdir, root_uri) = temp_root_uri();
        expect_missing_config_messages(&mock);
        initialize_server(&mut server, root_uri.clone());

        let file_url = root_uri.join("ent.vhd").unwrap();
        let code = "
entity ent is
end entity ent;
"
        .to_owned();

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: file_url.clone(),
                language_id: "vhdl".to_owned(),
                version: 0,
                text: code.to_owned(),
            },
        };

        mock.expect_message_contains("is not part of the project");

        server.text_document_did_open_notification(&did_open);
    }

    #[test]
    fn did_open_with_diagnostics_and_change_without() {
        let (mock, mut server) = setup_server();

        let (_tempdir, root_uri) = temp_root_uri();
        expect_missing_config_messages(&mock);
        initialize_server(&mut server, root_uri.clone());

        let file_url = root_uri.join("ent.vhd").unwrap();
        let code = "
entity ent is
end entity ent2;
"
        .to_owned();

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: file_url.clone(),
                language_id: "vhdl".to_owned(),
                version: 0,
                text: code.to_owned(),
            },
        };

        let publish_diagnostics = PublishDiagnosticsParams {
            uri: file_url.clone(),
            diagnostics: vec![lsp_types::Diagnostic {
                range: Range {
                    start: lsp_types::Position {
                        line: 2,
                        character: "end entity ".len() as u64,
                    },
                    end: lsp_types::Position {
                        line: 2,
                        character: "end entity ent2".len() as u64,
                    },
                },
                code: None,
                severity: Some(DiagnosticSeverity::Error),
                source: Some("vhdl ls".to_owned()),
                message: "End identifier mismatch, expected ent".to_owned(),
                related_information: None,
                tags: None,
            }],
            version: None,
        };

        mock.expect_message_contains("is not part of the project");

        mock.expect_notification("textDocument/publishDiagnostics", publish_diagnostics);
        server.text_document_did_open_notification(&did_open);

        let code = "
entity ent is
end entity ent;
"
        .to_owned();

        let did_change = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: file_url.clone(),
                version: Some(1),
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: code.clone(),
            }],
        };

        let publish_diagnostics = PublishDiagnosticsParams {
            uri: file_url.clone(),
            diagnostics: vec![],
            version: None,
        };

        mock.expect_notification("textDocument/publishDiagnostics", publish_diagnostics);
        server.text_document_did_change_notification(&did_change);
    }

    fn write_file(root_uri: &Url, file_name: impl AsRef<str>, contents: impl AsRef<str>) -> Url {
        let path = root_uri.to_file_path().unwrap().join(file_name.as_ref());
        std::fs::write(&path, contents.as_ref()).unwrap();
        Url::from_file_path(path).unwrap()
    }

    fn write_config(root_uri: &Url, contents: impl AsRef<str>) -> Url {
        write_file(root_uri, "vhdl_ls.toml", contents)
    }

    #[test]
    fn initialize_with_config() {
        let (mock, mut server) = setup_server();
        let (_tempdir, root_uri) = temp_root_uri();
        let file_uri = write_file(
            &root_uri,
            "file.vhd",
            "\
entity ent is
end entity;

architecture rtl of ent2 is
begin
end;
",
        );

        let config_uri = write_config(
            &root_uri,
            "
[libraries]
lib.files = [
  'file.vhd'
]
",
        );

        let publish_diagnostics = PublishDiagnosticsParams {
            uri: file_uri.clone(),
            diagnostics: vec![lsp_types::Diagnostic {
                range: Range {
                    start: lsp_types::Position {
                        line: 3,
                        character: "architecture rtl of ".len() as u64,
                    },
                    end: lsp_types::Position {
                        line: 3,
                        character: "architecture rtl of ent2".len() as u64,
                    },
                },
                code: None,
                severity: Some(DiagnosticSeverity::Error),
                source: Some("vhdl ls".to_owned()),
                message: "No entity \'ent2\' within library \'lib\'".to_owned(),
                related_information: None,
                tags: None,
            }],
            version: None,
        };

        expect_loaded_config_messages(&mock, &config_uri);
        mock.expect_notification("textDocument/publishDiagnostics", publish_diagnostics);

        initialize_server(&mut server, root_uri);
    }

    #[test]
    fn initialize_with_bad_config() {
        let (mock, mut server) = setup_server();
        let (_tempdir, root_uri) = temp_root_uri();

        write_config(
            &root_uri,
            "
[libraries
",
        );

        expect_missing_config_messages(&mock);
        initialize_server(&mut server, root_uri);
    }

    #[test]
    fn initialize_with_config_missing_files() {
        let (mock, mut server) = setup_server();
        let (_tempdir, root_uri) = temp_root_uri();

        let config_uri = write_config(
            &root_uri,
            "
[libraries]
lib.files = [
'missing_file.vhd',
]
",
        );

        expect_loaded_config_messages(&mock, &config_uri);
        mock.expect_message_contains("missing_file.vhd");
        initialize_server(&mut server, root_uri);
    }

    #[test]
    fn text_document_declaration() {
        let (mock, mut server) = setup_server();
        let (_tempdir, root_uri) = temp_root_uri();

        let file_url1 = write_file(
            &root_uri,
            "pkg1.vhd",
            "\
package pkg1 is
  type typ_t is (foo, bar);
end package;
",
        );

        let code2 = "\
use work.pkg1.all;
package pkg2 is
  constant c : typ_t := bar;
end package;
        "
        .to_owned();
        let file_url2 = write_file(&root_uri, "pkg2.vhd", &code2);

        let config_uri = write_config(
            &root_uri,
            "
[libraries]
lib.files = [
  '*.vhd'
]
",
        );

        expect_loaded_config_messages(&mock, &config_uri);
        initialize_server(&mut server, root_uri.clone());

        let did_open = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: file_url2.clone(),
                language_id: "vhdl".to_owned(),
                version: 0,
                text: code2.to_owned(),
            },
        };

        server.text_document_did_open_notification(&did_open);

        let response = server.text_document_declaration(&TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: file_url2.clone(),
            },
            position: lsp_types::Position {
                line: 2,
                character: "  constant c : t".len() as u64,
            },
        });

        let expected = Location {
            uri: file_url1.clone(),
            range: Range {
                start: lsp_types::Position {
                    line: 1,
                    character: "  type ".len() as u64,
                },
                end: lsp_types::Position {
                    line: 1,
                    character: "  type tpe_t".len() as u64,
                },
            },
        };

        assert_eq!(response, Some(expected));
    }
}
