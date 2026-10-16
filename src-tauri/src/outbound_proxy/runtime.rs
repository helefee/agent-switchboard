//! Immutable runtime policy and revision-bound HTTP clients.
use super::{OutboundProxyMode, OutboundProxySettings};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

pub(super) fn update(settings: &OutboundProxySettings) {
    store_snapshot(Arc::new(Snapshot {
        revision: settings.revision.clone(), mode: settings.mode, url: settings.url.clone(),
    }));
}

struct Snapshot {
    revision: String,
    mode: OutboundProxyMode,
    url: Option<String>,
}

fn cell() -> &'static RwLock<Arc<Snapshot>> {
    static SNAPSHOT: OnceLock<RwLock<Arc<Snapshot>>> = OnceLock::new();
    SNAPSHOT.get_or_init(|| {
        RwLock::new(Arc::new(Snapshot {
            revision: OutboundProxySettings::default().revision,
            mode: OutboundProxyMode::None,
            url: None,
        }))
    })
}

fn snapshot() -> Arc<Snapshot> {
    cell().read().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
}

fn store_snapshot(next: Arc<Snapshot>) {
    *cell().write().unwrap_or_else(|poisoned| poisoned.into_inner()) = next;
}

fn port_cell() -> &'static RwLock<Option<u16>> {
    static PORT: OnceLock<RwLock<Option<u16>>> = OnceLock::new();
    PORT.get_or_init(|| RwLock::new(None))
}

fn gateway_port() -> Option<u16> {
    port_cell().read().ok().and_then(|port| *port)
}

/// The gateway records its listening port so System mode can refuse to egress
/// through itself.
pub(crate) fn note_gateway_port(port: u16) {
    if let Ok(mut current) = port_cell().write() {
        *current = Some(port);
    }
}

fn system_proxy_points_at_self() -> bool {
    let Some(port) = gateway_port() else {
        return false;
    };
    let mut variables = Vec::new();
    for name in ["http_proxy", "https_proxy", "all_proxy"] {
        variables.push(std::env::var(name).or_else(|_| std::env::var(name.to_uppercase())));
    }
    variables
        .into_iter()
        .flatten()
        .filter_map(|value| reqwest::Url::parse(value.trim()).ok())
        .any(|url| {
            let host_is_loopback = url
                .host_str()
                .is_some_and(|host| host.eq_ignore_ascii_case("localhost"))
                || url
                    .host_str()
                    .is_some_and(|host| matches!(host, "127.0.0.1" | "[::1]" | "::1" | "0.0.0.0"));
            host_is_loopback && url.port() == Some(port)
        })
}

/// The decision resolved from one validated policy snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
enum EgressDecision {
    /// Force direct egress regardless of the environment.
    Direct,
    /// Follow the environment/system proxy configuration.
    System,
    /// Route through this explicit proxy URL.
    Proxy(String),
}

fn decision_for(current: &Snapshot) -> EgressDecision {
    match current.mode {
        OutboundProxyMode::None => EgressDecision::Direct,
        OutboundProxyMode::System => {
            if system_proxy_points_at_self() {
                EgressDecision::Direct
            } else {
                EgressDecision::System
            }
        }
        OutboundProxyMode::Manual => EgressDecision::Proxy(
            current.url.as_deref().expect("manual proxy settings were validated").trim().to_string(),
        ),
    }
}

/// Applies the current policy to an async client builder. `None` forces direct
/// egress; `System` keeps reqwest's environment handling unless the detected
/// proxy is this gateway itself (a request loop); `Manual` routes everything
/// through the configured proxy.
pub(crate) fn apply(builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    apply_snapshot(builder, &snapshot())
}

fn apply_snapshot(builder: reqwest::ClientBuilder, current: &Snapshot) -> reqwest::ClientBuilder {
    match decision_for(current) {
        EgressDecision::Direct => builder.no_proxy(),
        EgressDecision::System => builder,
        EgressDecision::Proxy(url) => builder.proxy(
            reqwest::Proxy::all(&url).expect("proxy URL was validated before publication"),
        ),
    }
}

/// Blocking twin of [`apply`] for the probe transport.
pub(crate) fn apply_blocking(
    builder: reqwest::blocking::ClientBuilder,
) -> reqwest::blocking::ClientBuilder {
    apply_blocking_snapshot(builder, &snapshot())
}

fn apply_blocking_snapshot(
    builder: reqwest::blocking::ClientBuilder,
    current: &Snapshot,
) -> reqwest::blocking::ClientBuilder {
    match decision_for(current) {
        EgressDecision::Direct => builder.no_proxy(),
        EgressDecision::System => builder,
        EgressDecision::Proxy(url) => builder.proxy(
            reqwest::Proxy::all(&url).expect("proxy URL was validated before publication"),
        ),
    }
}

/// Blocking twin of `cached_client`. One key owns one revision and client;
/// serializing cache misses prevents duplicate builds and stale insertions.
pub(crate) fn cached_blocking_client(
    key: &'static str,
    build: impl FnOnce(reqwest::blocking::ClientBuilder) -> reqwest::blocking::Client,
) -> reqwest::blocking::Client {
    static CLIENTS: OnceLock<RwLock<HashMap<&'static str, (String, reqwest::blocking::Client)>>> = OnceLock::new();
    let cell = CLIENTS.get_or_init(|| RwLock::new(HashMap::new()));
    let mut clients = cell.write().unwrap_or_else(|poisoned| poisoned.into_inner());
    let current = snapshot();
    if let Some((revision, client)) = clients.get(key) {
        if revision == &current.revision { return client.clone(); }
    }
    let client = build(apply_blocking_snapshot(reqwest::blocking::ClientBuilder::new(), &current));
    clients.insert(key, (current.revision.clone(), client.clone()));
    client
}

/// Each client is built and labelled from the same immutable policy snapshot.
pub(crate) fn cached_client(
    key: &'static str,
    build: impl FnOnce(reqwest::ClientBuilder) -> reqwest::Client,
) -> reqwest::Client {
    static CLIENTS: OnceLock<RwLock<HashMap<&'static str, (String, reqwest::Client)>>> = OnceLock::new();
    let cell = CLIENTS.get_or_init(|| RwLock::new(HashMap::new()));
    let mut clients = cell.write().unwrap_or_else(|poisoned| poisoned.into_inner());
    let current = snapshot();
    if let Some((revision, client)) = clients.get(key) {
        if revision == &current.revision { return client.clone(); }
    }
    let client = build(apply_snapshot(reqwest::Client::builder(), &current));
    clients.insert(key, (current.revision.clone(), client.clone()));
    client
}
