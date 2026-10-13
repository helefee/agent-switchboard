//! Bounded MCP probes. Protocol negotiation is shared by both transports.
use asb_core::contracts::AppKind;
use asb_core::extensions::contracts::{ExtensionTarget, McpCheckOutcome, McpCheckResult, McpDefinition};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

mod http;
mod material;
mod protocol;
mod sse;
mod stdio;

pub(super) const SUPPORTED_PROTOCOL_VERSIONS: [&str; 3] =
    ["2025-11-25", "2025-06-18", "2025-03-26"];
pub(super) const INITIALIZE_DEADLINE: Duration = Duration::from_secs(10);
pub(crate) const TOTAL_DEADLINE: Duration = Duration::from_secs(30);
pub(super) const HTTP_REQUEST_DEADLINE: Duration = Duration::from_secs(10);
pub(super) const MAX_PAGES: u64 = 20;
pub(super) const MAX_OUTPUT_BYTES: usize = 1_048_576;
pub(super) const RECEIVE_POLL: Duration = Duration::from_millis(25);

pub(crate) struct CheckRequest {
    pub definition_id: String,
    pub definition_revision: u64,
    pub target: ExtensionTarget,
    pub working_dir: PathBuf,
}

pub(crate) fn run_check(
    request: &CheckRequest,
    definition: &McpDefinition,
    deadline: &CheckDeadline,
    resolve_secret: &dyn Fn(&str) -> Option<String>,
) -> McpCheckResult {
    let started = Instant::now();
    let probe = probe_definition(request, definition, deadline, resolve_secret);
    McpCheckResult {
        definition_id: request.definition_id.clone(),
        definition_revision: request.definition_revision,
        target: request.target.clone(),
        checked_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        protocol_version: probe.protocol_version,
        outcome: probe.outcome,
        duration_ms: started.elapsed().as_millis() as u64,
        truncated: probe.truncated,
    }
}

fn probe_definition(
    request: &CheckRequest,
    definition: &McpDefinition,
    deadline: &CheckDeadline,
    resolve_secret: &dyn Fn(&str) -> Option<String>,
) -> ProbeOutcome {
    if let Err(error) = deadline.remaining() { return ProbeOutcome::for_error(None, error); }
    if matches!(definition, McpDefinition::ClaudeSse { .. } | McpDefinition::ClaudeWs { .. }) {
        return ProbeOutcome::native(None);
    }
    let material = match material::material_for(definition, resolve_secret) {
        Ok(material) => material,
        Err(message) => return ProbeOutcome::failed("material", message),
    };
    match definition {
        McpDefinition::Stdio { command, args, codex_options, .. } => {
            let wrapped = if request.target.client() == AppKind::Claude
                && asb_core::extensions::mcp::ClaudeHost::current()
                    == asb_core::extensions::mcp::ClaudeHost::Windows
            {
                asb_core::extensions::mcp::wrap_windows_launcher(command, args)
            } else {
                None
            };
            let (command, args) = wrapped.as_ref()
                .map(|(command, args)| (command.as_str(), args.as_slice()))
                .unwrap_or((command.as_str(), args.as_slice()));
            let initialize_timeout = if request.target.client() == AppKind::Codex {
                codex_options.as_ref().and_then(|options| options.startup_timeout_sec)
                    .map(Duration::from_secs).unwrap_or(INITIALIZE_DEADLINE)
            } else {
                INITIALIZE_DEADLINE
            };
            stdio::check_stdio(command, args, &material.env, &request.working_dir,
                deadline, initialize_timeout)
        }
        McpDefinition::Http { url, .. } =>
            http::check_http(url, material.headers, material.bearer, deadline),
        McpDefinition::ClaudeSse { .. } | McpDefinition::ClaudeWs { .. } =>
            ProbeOutcome::native(None),
    }
}

struct ProbeOutcome {
    outcome: McpCheckOutcome,
    protocol_version: Option<String>,
    truncated: bool,
}

impl ProbeOutcome {
    fn failed(classification: &str, error: impl Into<String>) -> Self {
        Self {
            outcome: McpCheckOutcome::Failed {
                classification: classification.to_string(), error: error.into(),
            },
            protocol_version: None, truncated: false,
        }
    }

    fn native(protocol_version: Option<String>) -> Self {
        Self { outcome: McpCheckOutcome::NeedsNativeConfirmation, protocol_version, truncated: false }
    }

    fn for_error(protocol_version: Option<String>, error: CheckError) -> Self {
        if error.kind == CheckErrorKind::Authentication {
            return Self::native(protocol_version);
        }
        let outcome = if error.kind == CheckErrorKind::Cancelled {
            McpCheckOutcome::Cancelled
        } else {
            McpCheckOutcome::Failed {
                classification: match error.kind {
                    CheckErrorKind::TimedOut => "timeout",
                    CheckErrorKind::OutputLimit => "output",
                    CheckErrorKind::Transport => "network",
                    _ => "protocol",
                }.to_string(),
                error: error.message,
            }
        };
        Self { outcome, protocol_version, truncated: error.kind == CheckErrorKind::OutputLimit }
    }

    fn directory_error(protocol_version: &str, error: CheckError) -> Self {
        if matches!(error.kind, CheckErrorKind::Cancelled | CheckErrorKind::Authentication) {
            return Self::for_error(Some(protocol_version.to_string()), error);
        }
        Self {
            outcome: McpCheckOutcome::Partial {
                error: format!("握手成功但目录读取未完成：{}", error.message),
            },
            protocol_version: Some(protocol_version.to_string()),
            truncated: error.kind == CheckErrorKind::OutputLimit,
        }
    }
}

/// All phase/request deadlines are bounded by this single check deadline.
#[derive(Clone)]
pub(crate) struct CheckDeadline {
    ends_at: Instant,
    cancel_flag: Arc<AtomicBool>,
}

impl CheckDeadline {
    pub(crate) fn new(cancel_flag: Arc<AtomicBool>) -> Self {
        Self { ends_at: Instant::now() + TOTAL_DEADLINE, cancel_flag }
    }

    fn limited(&self, limit: Duration) -> Self {
        Self { ends_at: self.ends_at.min(Instant::now() + limit), cancel_flag: self.cancel_flag.clone() }
    }

    fn remaining(&self) -> Result<Duration, CheckError> {
        if self.cancel_flag.load(Ordering::SeqCst) {
            return Err(CheckError::cancelled());
        }
        self.ends_at.checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero()).ok_or_else(CheckError::timed_out)
    }

    fn request_timeout(&self) -> Result<Duration, CheckError> {
        Ok(self.remaining()?.min(HTTP_REQUEST_DEADLINE))
    }

    fn cleanup_timeout(&self) -> Option<Duration> {
        self.ends_at.checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .map(|remaining| remaining.min(Duration::from_millis(500)))
    }

    async fn cancelled(&self) {
        while !self.cancel_flag.load(Ordering::SeqCst) {
            tokio::time::sleep(RECEIVE_POLL).await;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CheckErrorKind { Cancelled, TimedOut, OutputLimit, Closed, Protocol, Transport, Authentication }

#[derive(Debug)]
struct CheckError { kind: CheckErrorKind, message: String }

impl CheckError {
    fn cancelled() -> Self { Self { kind: CheckErrorKind::Cancelled, message: "检测已取消".into() } }
    fn timed_out() -> Self { Self { kind: CheckErrorKind::TimedOut, message: "检测超时".into() } }
    fn output_limit() -> Self { Self { kind: CheckErrorKind::OutputLimit, message: "输出或分页超过上限".into() } }
    fn closed() -> Self { Self { kind: CheckErrorKind::Closed, message: "连接在响应完成前关闭".into() } }
    fn protocol(message: impl Into<String>) -> Self {
        Self { kind: CheckErrorKind::Protocol, message: message.into() }
    }
    fn transport(message: impl Into<String>) -> Self {
        Self { kind: CheckErrorKind::Transport, message: message.into() }
    }
}
