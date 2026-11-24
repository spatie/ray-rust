use crate::Error;
use serde_json::Value;
use std::{
    collections::{HashMap, VecDeque},
    env,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

/// Connection settings. Defaults match the framework-agnostic PHP client.
#[derive(Clone, Debug)]
#[must_use]
pub struct Config {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) timeout: Duration,
    pub(crate) enabled: bool,
    pub(crate) throws: bool,
    pub(crate) project: String,
    pub(crate) paths: Option<(String, String)>,
    pub(crate) raw: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "localhost".into(),
            port: 23517,
            timeout: Duration::from_secs(2),
            enabled: true,
            throws: false,
            project: String::new(),
            paths: None,
            raw: false,
        }
    }
}

impl Config {
    /// Read `RAY_HOST`, `RAY_PORT`, `RAY_ENABLED`, `RAY_PROJECT_NAME`,
    /// `RAY_REMOTE_PATH`, `RAY_LOCAL_PATH` and `RAY_ALWAYS_SEND_RAW_VALUES`.
    ///
    /// # Errors
    /// Returns [`Error::Config`] for invalid ports, booleans or non-Unicode values.
    pub fn from_env() -> Result<Self, Error> {
        fn get(key: &str) -> Result<Option<String>, Error> {
            match env::var(key) {
                Ok(value) => Ok(Some(value)),
                Err(env::VarError::NotPresent) => Ok(None),
                Err(_) => Err(Error::Config(format!("{key} must be Unicode"))),
            }
        }
        fn boolean(key: &str, default: bool) -> Result<bool, Error> {
            match get(key)?.as_deref() {
                None => Ok(default),
                Some("true" | "1") => Ok(true),
                Some("false" | "0") => Ok(false),
                Some(_) => Err(Error::Config(format!("{key} must be true or false"))),
            }
        }
        let mut config = Self::default();
        if let Some(host) = get("RAY_HOST")? {
            config.host = host;
        }
        if let Some(port) = get("RAY_PORT")? {
            config.port = port
                .parse::<u16>()
                .ok()
                .filter(|port| *port != 0)
                .ok_or_else(|| Error::Config("RAY_PORT must be between 1 and 65535".into()))?;
        }
        config.enabled = boolean("RAY_ENABLED", true)?;
        config.raw = boolean("RAY_ALWAYS_SEND_RAW_VALUES", false)?;
        config.project = get("RAY_PROJECT_NAME")?.unwrap_or_default();
        if let (Some(remote), Some(local)) = (get("RAY_REMOTE_PATH")?, get("RAY_LOCAL_PATH")?) {
            config.paths = Some((remote, local));
        }
        Ok(config)
    }

    /// Set the Ray app's host (without a scheme or path).
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Set the Ray app's port. The default is 23517.
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the complete HTTP request timeout. The default is two seconds.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable or disable sending on this client initially.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Report transport failures instead of silently continuing as PHP does.
    pub fn throws(mut self, throws: bool) -> Self {
        self.throws = throws;
        self
    }

    /// Set the project name sent in request metadata.
    pub fn project(mut self, project: impl Into<String>) -> Self {
        self.project = project.into();
        self
    }

    /// Map a remote source path prefix to a local editor path prefix.
    pub fn remote_path(mut self, remote: impl Into<String>, local: impl Into<String>) -> Self {
        self.paths = Some((remote.into(), local.into()));
        self
    }

    /// Use ordinary log payloads for every value, bypassing specialized rendering.
    pub fn always_send_raw_values(mut self, raw: bool) -> Self {
        self.raw = raw;
        self
    }
}

#[derive(Debug)]
pub(crate) struct State {
    pub enabled: bool,
    pub project: String,
    pub counters: HashMap<String, (String, u64)>,
    pub limits: HashMap<String, (usize, i64)>,
    pub timers: HashMap<String, (Instant, Instant)>,
    pub max: Option<usize>,
    pub per_second: Option<usize>,
    pub hits: usize,
    pub recent: VecDeque<Instant>,
    pub notified: bool,
}

/// An HTTP connection and explicitly shared debug session.
///
/// Clones share enablement, project, counters, timers and rate limits, but each
/// call to [`Client::ray`] creates a new message UUID. Independent clients do
/// not share state. Plain HTTP matches the PHP protocol; use an SSH tunnel for
/// remote debugging rather than exposing the unencrypted Ray port publicly.
#[derive(Clone, Debug)]
pub struct Client {
    pub(crate) config: Arc<Config>,
    agent: ureq::Agent,
    state: Arc<Mutex<State>>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

impl Client {
    /// Create an isolated session with these settings.
    #[must_use]
    pub fn new(config: Config) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(config.timeout))
            .max_redirects(0)
            .proxy(None)
            .build()
            .into();
        let state = State {
            enabled: config.enabled,
            project: config.project.clone(),
            counters: HashMap::new(),
            limits: HashMap::new(),
            timers: HashMap::new(),
            max: None,
            per_second: None,
            hits: 0,
            recent: VecDeque::new(),
            notified: false,
        };
        Self {
            config: Arc::new(config),
            agent,
            state: Arc::new(Mutex::new(state)),
        }
    }

    /// Start a new entry in this session.
    #[must_use]
    pub fn ray(&self) -> crate::Ray {
        crate::Ray::with_client(self.clone())
    }

    pub(crate) fn state(&self) -> Result<MutexGuard<'_, State>, Error> {
        self.state.lock().map_err(|_| Error::StatePoisoned)
    }

    fn url(&self, path: &str) -> String {
        let host = &self.config.host;
        if host.contains(':') && !host.starts_with('[') {
            format!("http://[{host}]:{}/{path}", self.config.port)
        } else {
            format!("http://{host}:{}/{path}", self.config.port)
        }
    }

    pub(crate) fn post(&self, request: &Value) -> Result<(), Error> {
        self.agent
            .post(self.url(""))
            .header("Accept", "application/json")
            .header("User-Agent", "Ray 1.0")
            .send_json(request)?;
        Ok(())
    }

    pub(crate) fn get(&self, path: &str) -> Result<Value, Error> {
        Ok(self
            .agent
            .get(self.url(path))
            .header("Accept", "application/json")
            .header("User-Agent", "Ray 1.0")
            .call()?
            .body_mut()
            .read_json::<Value>()?)
    }

    /// Get the app's open windows. Transport errors always propagate here.
    pub fn windows(&self) -> Result<Value, Error> {
        self.get("windows")
    }

    /// Get the app's theme. Transport errors always propagate here.
    pub fn theme(&self) -> Result<Value, Error> {
        self.get("theme")
    }

    /// Limit the total number of requests from this session; `None` disables it.
    pub fn max(&self, max: Option<usize>) -> Result<&Self, Error> {
        self.state()?.max = max;
        Ok(self)
    }

    /// Limit requests in a sliding one-second window; `None` disables it.
    pub fn per_second(&self, max: Option<usize>) -> Result<&Self, Error> {
        self.state()?.per_second = max;
        Ok(self)
    }

    /// Remove rate limits and reset their counters and notification.
    pub fn clear_rate_limits(&self) -> Result<&Self, Error> {
        let mut state = self.state()?;
        state.max = None;
        state.per_second = None;
        state.hits = 0;
        state.recent.clear();
        state.notified = false;
        Ok(self)
    }
}
