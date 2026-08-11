#![forbid(unsafe_code)]

extern crate alloc;

use alloc::{string::String, vec::Vec};
use core::{error::Error, fmt};

/// SHA-256 of the reviewed full Codex 0.147.0 V2 schema.
const CODEX_0_147_SCHEMA_SHA256: &str =
    "ff10829cd75b67297019b39ab508ac699198574663579aa18336b7dc55ea178f";
/// Ordered discriminators retained in the reviewed authority-surface projection.
const CODEX_0_147_THREAD_ITEM_TYPES: [&str; 18] = [
    "userMessage",
    "hookPrompt",
    "agentMessage",
    "plan",
    "reasoning",
    "commandExecution",
    "fileChange",
    "mcpToolCall",
    "dynamicToolCall",
    "collabAgentToolCall",
    "subAgentActivity",
    "webSearch",
    "imageView",
    "sleep",
    "imageGeneration",
    "enteredReviewMode",
    "exitedReviewMode",
    "contextCompaction",
];
/// Complete ordered field set from the reviewed `ThreadStartParams` projection.
const CODEX_0_147_THREAD_START_FIELDS: [&str; 25] = [
    "allowProviderModelFallback",
    "approvalPolicy",
    "approvalsReviewer",
    "baseInstructions",
    "config",
    "cwd",
    "developerInstructions",
    "dynamicTools",
    "environments",
    "ephemeral",
    "experimentalRawEvents",
    "historyMode",
    "mockExperimentalField",
    "model",
    "modelProvider",
    "multiAgentMode",
    "permissions",
    "personality",
    "runtimeWorkspaceRoots",
    "sandbox",
    "selectedCapabilityRoots",
    "serviceName",
    "serviceTier",
    "sessionStartSource",
    "threadSource",
];

/// Result of checking whether an app-server protocol can preserve Tiber authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompatibilityReport {
    /// Protocol item types present without a matching isolation proof.
    unverified_operations: Vec<String>,
}

impl CompatibilityReport {
    /// Reports whether app-server exposes only operations Tiber can safely treat as inference.
    #[expect(
        clippy::implicit_return,
        reason = "a single-expression predicate is clearer than an explicit return"
    )]
    #[inline]
    #[must_use]
    pub const fn is_compatible(&self) -> bool {
        self.unverified_operations.is_empty()
    }

    /// Returns protocol item types whose isolation from Tiber remains unverified.
    #[expect(
        clippy::implicit_return,
        reason = "a single-expression accessor is clearer than an explicit return"
    )]
    #[inline]
    #[must_use]
    pub fn unverified_operations(&self) -> &[String] {
        &self.unverified_operations
    }
}

/// A typed failure produced while inspecting the app-server protocol schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompatibilityError {
    /// Stable machine-readable classification.
    code: &'static str,
    /// Actionable human-readable diagnostic.
    message: String,
}

impl CompatibilityError {
    /// Stable machine-readable error code.
    #[expect(
        clippy::implicit_return,
        reason = "a single-expression accessor is clearer than an explicit return"
    )]
    #[inline]
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for CompatibilityError {
    #[expect(
        clippy::implicit_return,
        reason = "the formatter directly returns the delegated formatting result"
    )]
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[expect(
    clippy::missing_trait_methods,
    reason = "the Error defaults correctly express a leaf diagnostic with no source"
)]
impl Error for CompatibilityError {}

/// Inspects a provenance-bound app-server authority-surface projection.
///
/// # Errors
///
/// Returns a typed error when the schema is not valid JSON.
#[inline]
#[expect(
    clippy::implicit_return,
    reason = "iterator predicates and the final Result use idiomatic expression returns"
)]
pub fn inspect_protocol_schema(schema: &str) -> Result<CompatibilityReport, CompatibilityError> {
    let document = match serde_json::from_str::<serde_json::Value>(schema) {
        Ok(document) => document,
        Err(error) => {
            return Err(CompatibilityError {
                code: "app_server_schema_invalid",
                message: format!("app-server protocol schema is invalid: {error}"),
            });
        }
    };

    if document.get("title").and_then(serde_json::Value::as_str) != Some("CodexAppServerProtocolV2")
    {
        return Err(unrecognized_schema(
            "expected the CodexAppServerProtocolV2 schema title",
        ));
    }
    if document
        .pointer("/_provenance/codexVersion")
        .and_then(serde_json::Value::as_str)
        != Some("0.147.0")
        || document
            .pointer("/_provenance/schemaSha256")
            .and_then(serde_json::Value::as_str)
            != Some(CODEX_0_147_SCHEMA_SHA256)
    {
        return Err(unrecognized_schema(
            "expected the verified Codex 0.147.0 authority-surface projection",
        ));
    }
    let thread_start_properties = match document
        .pointer("/definitions/ThreadStartParams/properties")
        .and_then(serde_json::Value::as_object)
    {
        Some(properties) => properties,
        None => {
            return Err(unrecognized_schema("missing ThreadStartParams.properties"));
        }
    };
    if thread_start_properties.len() != CODEX_0_147_THREAD_START_FIELDS.len()
        || !CODEX_0_147_THREAD_START_FIELDS
            .into_iter()
            .all(|field| thread_start_properties.contains_key(field))
    {
        return Err(unrecognized_schema(
            "ThreadStartParams differs from the verified 0.147 authority surface",
        ));
    }
    let thread_item_types = match parse_thread_item_types(&document) {
        Ok(item_types) => item_types,
        Err(error) => return Err(error),
    };
    if thread_item_types != CODEX_0_147_THREAD_ITEM_TYPES {
        return Err(unrecognized_schema(
            "ThreadItem differs from the verified 0.147 authority surface",
        ));
    }
    let unverified_operations = ["commandExecution", "fileChange"]
        .into_iter()
        .map(|operation| format!("thread-item:{operation}:no-isolation-proof"))
        .collect::<Vec<_>>();

    Ok(CompatibilityReport {
        unverified_operations,
    })
}

/// Parses every V2 `ThreadItem` discriminator without searching unrelated schema text.
#[expect(
    clippy::implicit_return,
    reason = "the parser returns its collected semantic values as a final expression"
)]
#[expect(
    clippy::single_call_fn,
    reason = "a named parser keeps schema-shape validation separate and independently reviewable"
)]
fn parse_thread_item_types(document: &serde_json::Value) -> Result<Vec<&str>, CompatibilityError> {
    let variants = match document
        .pointer("/definitions/ThreadItem/oneOf")
        .and_then(serde_json::Value::as_array)
    {
        Some(variants) if !variants.is_empty() => variants,
        Some(_) | None => return Err(unrecognized_schema("missing ThreadItem.oneOf variants")),
    };
    let mut item_types = Vec::with_capacity(variants.len());
    for variant in variants {
        let Some(item_type) = variant
            .pointer("/properties/type/enum/0")
            .and_then(serde_json::Value::as_str)
        else {
            return Err(unrecognized_schema(
                "ThreadItem variant lacks a string type discriminator",
            ));
        };
        item_types.push(item_type);
    }
    Ok(item_types)
}

/// Builds the stable fail-closed error for an unknown schema structure.
#[expect(
    clippy::implicit_return,
    reason = "the constructor is clearest as a single struct expression"
)]
fn unrecognized_schema(detail: &str) -> CompatibilityError {
    CompatibilityError {
        code: "app_server_schema_contract_unrecognized",
        message: format!("cannot verify app-server authority contract: {detail}"),
    }
}
