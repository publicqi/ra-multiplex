//! ## Language server protocol
//!
//! Specification can be found at
//! <https://microsoft.github.io/language-server-protocol/specifications/specification-current/>.
//!
//! We're not interested in supporting or even parsing the whole protocol, we only want a subset
//! that will allow us to mupltiplex messages between multiple clients and a single server.
//!
//! LSP has several main message types:
//!
//! ### Request Message
//! Requests from client to server. Requests contain an `id` property which is either `integer` or
//! `string`.
//!
//! ### Response Message
//! Responses from server for client requests. Also contain an `id` property, but according to the
//! the specification it can also be null, it's unclear what we should do when it is null. We could
//! either send the response to all clients or drop it.
//!
//! ### Notification Message
//! Notifications must not receive a response, this doesn't really mean anything to us as we're
//! just relaying the messages. It sounds like it'd allow us to simply pass a notification from any
//! client to the server and to pass a server notification to all clients, however there are some
//! subtypes of notifications defined by the LSP where that could be confusing to the client or
//! server:
//! - Cancel notifications - contains an `id` property again, so we could multiplex this like any
//!   other request
//! - Progress notifications - contains a `token` property which could be used to identify the
//!   client but the specification also says it has nothing to do with the request IDs

use serde_derive::{Deserialize, Serialize};

pub mod jsonrpc;
pub mod transport;

/// Params for the `initialize` request
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub process_id: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_info: Option<ClientInfo>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_path: Option<String>,

    pub root_uri: Option<String>,

    /// User provided initialization options
    ///
    /// This is where lspmux-proxy should be inserting it's setup. However we
    /// should remove them again before forwarding them to the language server.
    pub initialization_options: Option<InitializationOptions>,

    pub capabilities: serde_json::Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<TraceValue>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub workspace_folders: Vec<WorkspaceFolder>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct InitializationOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lsp_mux: Option<LspMuxOptions>,

    #[serde(flatten)]
    pub other_options: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum TraceValue {
    Off,
    Messages,
    Verbose,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorkspaceFolder {
    pub uri: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LspMuxOptions {
    /// Version number of the ra-multiplex binary
    ///
    /// Version is for now naively checked for equality, the server will refuse
    /// connections to mismatched clients.
    ///
    /// If you're using ra-multiplex just make sure you're using the same build
    /// for both the proxy and server, restarting the server if you've upgraded.
    ///
    /// If you're connecting directly from a client make sure to set the same
    /// protocol version reported by `ra-multiplex --version`.
    pub version: String,

    /// The language server to run
    ///
    /// Can be either an absolute path like `/usr/local/bin/rust-analyzer` or a
    /// plain name like `rust-analyzer` which will then be resolved according to
    /// the *server's* path.
    pub server: String,

    /// Arguments which will be passed to the language server, defaults to an
    /// empty list if omited.
    #[serde(default = "Vec::new")]
    pub args: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    capabilities: serde_json::Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    server_info: Option<ServerInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    name: String,
    version: Option<String>,
}

#[cfg(test)]
mod tests {
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use serde_json::{from_value, json, to_value, Value};

    use super::*;

    fn test<T>(input: Value)
    where
        T: Serialize + DeserializeOwned,
    {
        let deserialized = from_value::<T>(input.clone()).expect("failed to deserialize");
        let serialized = to_value(&deserialized).expect("failed to serialize");
        assert_eq!(input, serialized);
    }

    #[test]
    fn lsp_mux_only() {
        test::<InitializationOptions>(json!({
            "lspMux": {
                "version": "1",
                "server": "some-language-server",
                "args": ["a", "b", "c"]
            }
        }))
    }

    #[test]
    fn lsp_mux_and_other_stuff() {
        test::<InitializationOptions>(json!({
            "lspMux": {
                "version": "1",
                "server": "some-language-server",
                "args": ["a", "b", "c"]
            },
            "lsp_mux": "not the right key",
            "lspmux": "also not it",
            "lsp mux": "wrong one",
            "a": 1,
            "b": null,
            "c": {},
            "d": [],
        }))
    }

    #[test]
    #[should_panic = "missing field `version`"]
    fn missing_version() {
        test::<InitializationOptions>(json!({
            "lspMux": {
                "server": "some-language-server",
                "args": ["a", "b", "c"]
            },
        }))
    }

    #[test]
    #[should_panic = "missing field `server`"]
    fn missing_server() {
        test::<InitializationOptions>(json!({
            "lspMux": {
                "version": "1",
                "args": ["a", "b", "c"]
            },
        }))
    }
}
