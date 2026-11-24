use crate::{Client, Error, Frame, Origin, Payload, Value, json, payload};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Serialize;
use std::{
    any::type_name,
    fs,
    path::Path,
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};
use uuid::Uuid;

/// A single entry in the Ray app, with a stable UUID for fluent updates.
///
/// Sending operations consume and return the handle. Keep the returned handle
/// when updating an existing entry: `entry = entry.send("updated")?`.
/// Clones update the same entry. Operations return serialization/state errors;
/// network errors are swallowed unless [`crate::Config::throws`] is enabled.
#[derive(Clone, Debug)]
pub struct Ray {
    client: Client,
    uuid: String,
    can_send: bool,
    limit_origin: Option<String>,
}

macro_rules! commands {
    ($($name:ident => $kind:literal, $doc:literal;)+) => {
        $(#[doc = $doc]
        #[track_caller]
        pub fn $name(self) -> Result<Self, Error> {
            self.send_request(&[Payload::empty($kind)])
        })+
    };
}

macro_rules! aliases {
    ($($name:ident => $target:ident($value:literal), $doc:literal;)+) => {
        $(#[doc = $doc]
        #[track_caller]
        pub fn $name(self) -> Result<Self, Error> {
            self.$target($value)
        })+
    };
}

impl Default for Ray {
    fn default() -> Self {
        Self::new()
    }
}

impl Ray {
    /// Start a new entry using the process's default client.
    ///
    /// The default client uses [`crate::Config::default`]. To configure Ray,
    /// explicitly construct a [`Client`] and call [`Client::ray`].
    #[must_use]
    pub fn new() -> Self {
        static CLIENT: OnceLock<Client> = OnceLock::new();
        CLIENT.get_or_init(Client::default).ray()
    }

    /// Start a new entry using an explicitly owned client session.
    #[must_use]
    pub fn with_client(client: Client) -> Self {
        Self {
            client,
            uuid: Uuid::new_v4().to_string(),
            can_send: true,
            limit_origin: None,
        }
    }

    /// Get this entry's UUID. All updates to the entry share this identifier.
    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    /// Send a serializable value. Booleans/null use PHP's specialized payloads;
    /// structured Rust values use Ray's native JSON renderer, not PHP reflection.
    #[track_caller]
    pub fn send(self, value: impl Serialize) -> Result<Self, Error> {
        self.send_many(&[serde_json::to_value(value)?])
    }

    /// Send multiple values as separate payloads in one request and entry.
    #[track_caller]
    pub fn send_many(self, values: &[Value]) -> Result<Self, Error> {
        let payloads = values
            .iter()
            .map(|value| Payload::value(value.clone(), self.client.config.raw))
            .collect::<Result<Vec<_>, _>>()?;
        self.send_request(&payloads)
    }

    /// Send a value using an ordinary log payload, bypassing bool/null rendering.
    /// Structured values use escaped, preformatted JSON instead of PHP VarDumper.
    #[track_caller]
    pub fn raw(self, value: impl Serialize) -> Result<Self, Error> {
        let payload = Payload::value(serde_json::to_value(value)?, true)?;
        self.send_request(&[payload])
    }

    /// Send a Debug-only value as escaped text. No Serialize implementation needed.
    #[track_caller]
    pub fn debug(self, value: &(impl std::fmt::Debug + ?Sized)) -> Result<Self, Error> {
        self.text(&format!("{value:#?}"))
    }

    /// Display JSON generated from a serializable value.
    #[track_caller]
    pub fn to_json(self, value: impl Serialize) -> Result<Self, Error> {
        let payload = Payload::json_string(&serde_json::to_value(value)?)?;
        self.send_request(&[payload])
    }

    /// Display a valid JSON string using Ray's native JSON renderer.
    /// Invalid JSON returns [`Error::Json`] rather than displaying PHP's null.
    #[track_caller]
    pub fn json(self, value: &str) -> Result<Self, Error> {
        self.to_json(serde_json::from_str::<Value>(value)?)
    }

    /// Send arbitrary content with a custom label.
    #[track_caller]
    pub fn send_custom(self, content: &str, label: &str) -> Result<Self, Error> {
        self.send_request(&[Payload::custom(content, label)])
    }

    /// Render HTML in the Ray app. Only send HTML you trust.
    #[track_caller]
    pub fn html(self, html: &str) -> Result<Self, Error> {
        self.send_custom(html, "HTML")
    }

    /// Display literal text, preserving spaces and line breaks.
    #[track_caller]
    pub fn text(self, text: &str) -> Result<Self, Error> {
        self.send_custom(
            &payload::escape(text, true)
                .replace(' ', "&nbsp;")
                .replace('\n', "<br>"),
            "Text",
        )
    }

    /// Display formatted XML. Invalid XML returns [`Error::Xml`].
    #[track_caller]
    pub fn xml(self, xml: &str) -> Result<Self, Error> {
        self.send_custom(&payload::xml(xml)?, "XML")
    }

    /// Display a file's contents, escaped as HTML, labelled with its filename.
    /// Missing files are displayed as a message, like PHP; other I/O errors propagate.
    #[track_caller]
    pub fn file(self, filename: impl AsRef<Path>) -> Result<Self, Error> {
        let filename = filename.as_ref();
        let content = match fs::read_to_string(filename) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return self
                    .send_custom(&format!("File not found: '{}'", filename.display()), "File");
            }
            Err(error) => return Err(error.into()),
        };
        self.send_custom(
            &payload::escape(&content, false).replace('\n', "<br />"),
            &filename.file_name().unwrap_or_default().to_string_lossy(),
        )
    }

    /// Display an image from a filename, URL, base64 string or image data URI.
    #[track_caller]
    pub fn image(self, location: &str) -> Result<Self, Error> {
        self.send_request(&[payload::image(location)])
    }

    /// Render a clickable link, adding https:// when no http prefix is present.
    /// URL and label are HTML-escaped; unlike PHP, markup cannot be injected.
    #[track_caller]
    pub fn link(self, url: &str, label: &str) -> Result<Self, Error> {
        let url = if url.starts_with("http") {
            url.to_owned()
        } else {
            format!("https://{url}")
        };
        let label = if label.is_empty() { &url } else { label };
        self.html(&format!(
            "<a href='{}'>{}</a>",
            payload::escape(&url, false),
            payload::escape(label, false)
        ))
    }

    /// Alias for [`Ray::link`]. An empty label uses the URL.
    #[track_caller]
    pub fn url(self, url: &str, label: &str) -> Result<Self, Error> {
        self.link(url, label)
    }

    /// Format an array or object as a table with the default label "Table".
    #[track_caller]
    pub fn table(self, values: impl Serialize) -> Result<Self, Error> {
        self.table_with_label(values, "Table")
    }

    /// Format an array or object as a table with a custom label.
    /// Nested cells are escaped, preformatted JSON rather than PHP VarDumper HTML.
    #[track_caller]
    pub fn table_with_label(self, values: impl Serialize, label: &str) -> Result<Self, Error> {
        let values = match serde_json::to_value(values)? {
            Value::Array(values) => Value::Array(
                values
                    .into_iter()
                    .map(payload::primitive)
                    .collect::<Result<_, _>>()?,
            ),
            Value::Object(values) => Value::Object(
                values
                    .into_iter()
                    .map(|(key, value)| Ok((key, payload::primitive(value)?)))
                    .collect::<Result<_, Error>>()?,
            ),
            _ => return Err(Error::Config("table expects an array or object".into())),
        };
        self.send_request(&[Payload::new(
            "table",
            json!({"values": values, "label": label}),
        )])
    }

    /// Send the Rust type name of a value (the equivalent of PHP className).
    #[track_caller]
    pub fn class_name<T: ?Sized>(self, _value: &T) -> Result<Self, Error> {
        self.send(type_name::<T>())
    }

    /// Start a named screen. An empty string creates an unnamed screen.
    #[track_caller]
    pub fn new_screen(self, name: &str) -> Result<Self, Error> {
        self.send_request(&[Payload::new("new_screen", json!({"name": name}))])
    }

    /// Start an unnamed screen, archiving the previous screen.
    #[track_caller]
    pub fn clear_screen(self) -> Result<Self, Error> {
        self.new_screen("")
    }

    /// Set this entry's color.
    #[track_caller]
    pub fn color(self, color: &str) -> Result<Self, Error> {
        self.send_request(&[Payload::new("color", json!({"color": color}))])
    }

    /// Set the current screen's color.
    #[track_caller]
    pub fn screen_color(self, color: &str) -> Result<Self, Error> {
        self.send_request(&[Payload::new("screen_color", json!({"color": color}))])
    }

    /// Set this entry's label.
    #[track_caller]
    pub fn label(self, label: &str) -> Result<Self, Error> {
        self.send_request(&[Payload::new("label", json!({"label": label}))])
    }

    /// Set this entry's size: "sm", "md" or "lg".
    #[track_caller]
    pub fn size(self, size: &str) -> Result<Self, Error> {
        self.send_request(&[Payload::new("size", json!({"size": size}))])
    }

    /// Display a system notification.
    #[track_caller]
    pub fn notify(self, text: &str) -> Result<Self, Error> {
        self.send_request(&[Payload::new("notify", json!({"value": text}))])
    }

    /// Open nodes to a depth in renderers supporting Ray's expand command.
    #[track_caller]
    pub fn expand(self, level: u32) -> Result<Self, Error> {
        self.send_request(&[Payload::new("expand", json!({"keys": [], "level": level}))])
    }

    /// Open named nodes (dot notation supported) in compatible renderers.
    #[track_caller]
    pub fn expand_keys(self, keys: &[&str]) -> Result<Self, Error> {
        self.send_request(&[Payload::new("expand", json!({"keys": keys, "level": null}))])
    }

    /// Open all levels in a compatible renderer.
    #[track_caller]
    pub fn expand_all(self) -> Result<Self, Error> {
        self.expand(999)
    }

    commands! {
        clear_all => "clear_all", "Clear the current screen and its history.";
        remove => "remove", "Remove this entry from the app.";
        hide => "hide", "Collapse this entry immediately.";
        separator => "separator", "Add a visual separator.";
        show_app => "show_app", "Bring the app to the foreground.";
        hide_app => "hide_app", "Hide the app.";
        confetti => "confetti", "Shoot confetti in the current screen.";
    }

    aliases! {
        green => color("green"), "Output in green.";
        orange => color("orange"), "Output in orange.";
        yellow => color("orange"), "Output in yellow (PHP's alias for orange).";
        red => color("red"), "Output in red.";
        purple => color("purple"), "Output in purple.";
        blue => color("blue"), "Output in blue.";
        gray => color("gray"), "Output in gray.";
        grey => color("gray"), "Alias for gray.";
        screen_green => screen_color("green"), "Color the screen green.";
        screen_orange => screen_color("orange"), "Color the screen orange.";
        screen_red => screen_color("red"), "Color the screen red.";
        screen_purple => screen_color("purple"), "Color the screen purple.";
        screen_blue => screen_color("blue"), "Color the screen blue.";
        screen_gray => screen_color("gray"), "Color the screen gray.";
        small => size("sm"), "Output text smaller.";
        large => size("lg"), "Output text bigger.";
    }

    /// Change the project name for all entries using this client session.
    pub fn project(self, project: &str) -> Result<Self, Error> {
        self.client.state()?.project = project.into();
        Ok(self)
    }

    /// Enable sending for this client session.
    pub fn enable(self) -> Result<Self, Error> {
        self.client.state()?.enabled = true;
        Ok(self)
    }

    /// Disable sending for this client session.
    pub fn disable(self) -> Result<Self, Error> {
        self.client.state()?.enabled = false;
        Ok(self)
    }

    /// Check whether this client session is enabled.
    pub fn enabled(&self) -> Result<bool, Error> {
        Ok(self.client.state()?.enabled)
    }

    /// Check whether this client session is disabled.
    pub fn disabled(&self) -> Result<bool, Error> {
        Ok(!self.enabled()?)
    }

    /// Gate subsequent sends. A later call replaces the condition.
    #[must_use]
    pub fn when(mut self, condition: bool) -> Self {
        self.can_send = condition;
        self
    }

    /// Run a callback conditionally without changing the gate of later methods.
    pub fn when_with(
        self,
        condition: bool,
        callback: impl FnOnce(Self) -> Result<Self, Error>,
    ) -> Result<Self, Error> {
        if condition { callback(self) } else { Ok(self) }
    }

    /// Remove this entry when a condition is true.
    #[track_caller]
    pub fn remove_when(self, condition: bool) -> Result<Self, Error> {
        if condition { self.remove() } else { Ok(self) }
    }

    /// Remove this entry when a condition is false (PHP's showIf).
    #[track_caller]
    pub fn show_if(self, condition: bool) -> Result<Self, Error> {
        self.remove_when(!condition)
    }

    /// Send a value and return it instead of the Ray handle.
    #[track_caller]
    pub fn pass<T: Serialize>(self, value: T) -> Result<T, Error> {
        self.send(&value)?;
        Ok(value)
    }

    /// Limit sends from this source location across entries in this client session.
    /// Zero or negative values disable the limit, as in PHP.
    #[track_caller]
    pub fn limit(mut self, count: i64) -> Result<Self, Error> {
        let origin = Origin::caller();
        let key = format!("{}:{}", origin.file, origin.line_number);
        self.client
            .state()?
            .limits
            .entry(key.clone())
            .or_insert((0, count));
        self.limit_origin = Some(key);
        Ok(self)
    }

    /// Limit subsequent sends from this source location to one request.
    #[track_caller]
    pub fn once(self) -> Result<Self, Error> {
        self.limit(1)
    }

    /// Count calls at this source location, updating a single entry.
    #[track_caller]
    pub fn count(self) -> Result<Self, Error> {
        self.count_named("")
    }

    /// Count calls with a shared name and update the counter's existing entry.
    #[track_caller]
    pub fn count_named(mut self, name: &str) -> Result<Self, Error> {
        let origin = Origin::caller();
        let key = if name.is_empty() {
            format!("{}:{}", origin.file, origin.line_number)
        } else {
            name.into()
        };
        let times = {
            let mut state = self.client.state()?;
            let (uuid, times) = state
                .counters
                .entry(key)
                .or_insert_with(|| (self.uuid.clone(), 0));
            *times += 1;
            self.uuid.clone_from(uuid);
            *times
        };
        let name = if name.is_empty() {
            String::new()
        } else {
            format!("`{name}` ")
        };
        self.send_custom(
            &format!(
                "Called {name}{times} {}.",
                if times == 1 { "time" } else { "times" }
            ),
            "Count",
        )
    }

    /// Read a named counter, returning zero if it has not been used.
    pub fn counter_value(&self, name: &str) -> Result<u64, Error> {
        Ok(self
            .client
            .state()?
            .counters
            .get(name)
            .map_or(0, |(_, count)| *count))
    }

    /// Reset this session's counters.
    pub fn clear_counters(self) -> Result<Self, Error> {
        self.client.state()?.counters.clear();
        Ok(self)
    }

    /// Start or lap a named timer; an empty name uses "default".
    /// Durations use milliseconds. PHP heap-memory fields are zero in Rust.
    #[track_caller]
    pub fn measure(self, name: &str) -> Result<Self, Error> {
        let name = if name.is_empty() { "default" } else { name };
        let now = Instant::now();
        let (new, total, lap) = {
            let mut state = self.client.state()?;
            let new = !state.timers.contains_key(name);
            let (start, previous) = state.timers.entry(name.into()).or_insert((now, now));
            let total = now.duration_since(*start);
            let lap = now.duration_since(*previous);
            *previous = now;
            (new, total, lap)
        };
        self.send_request(&[measure_payload(name, new, total, lap)])
    }

    /// Run a closure and display the elapsed time. Panics are not caught.
    #[track_caller]
    pub fn measure_with(self, callback: impl FnOnce()) -> Result<Self, Error> {
        let start = Instant::now();
        callback();
        let elapsed = start.elapsed();
        self.send_request(&[measure_payload("closure", false, elapsed, elapsed)])
    }

    /// Reset a timer. An empty name resets all timers.
    pub fn stop_time(self, name: &str) -> Result<Self, Error> {
        if name.is_empty() {
            self.client.state()?.timers.clear();
        } else {
            self.client.state()?.timers.remove(name);
        }
        Ok(self)
    }

    /// Display a stack trace. Resolution depends on available debug symbols.
    #[track_caller]
    pub fn trace(self) -> Result<Self, Error> {
        self.trace_frames(&capture_frames())
    }

    /// Alias for [`Ray::trace`].
    #[track_caller]
    pub fn backtrace(self) -> Result<Self, Error> {
        self.trace()
    }

    /// Display explicitly provided stack frames (for example from another runtime).
    #[track_caller]
    pub fn trace_frames(self, frames: &[Frame]) -> Result<Self, Error> {
        let frames = self.map_frames(frames);
        self.send_request(&[Payload::new("trace", json!({"frames": frames}))])
    }

    /// Show the caller of the current function when debug symbols are available.
    #[track_caller]
    pub fn caller(self) -> Result<Self, Error> {
        let frames = capture_frames();
        let frame = frames.get(1).or_else(|| frames.first());
        match frame {
            Some(frame) => self.caller_frame(frame),
            None => Ok(self),
        }
    }

    /// Display an explicitly provided caller frame.
    #[track_caller]
    pub fn caller_frame(self, frame: &Frame) -> Result<Self, Error> {
        let frames = self.map_frames(std::slice::from_ref(frame));
        self.send_request(&[Payload::new("caller", json!({"frame": frames[0]}))])
    }

    /// Display an error with a Rust type name and an observation-site stack trace.
    /// Rust errors do not retain throw sites; the source chain is included in metadata.
    #[track_caller]
    pub fn exception<E: std::error::Error + ?Sized>(self, error: &E) -> Result<Self, Error> {
        let mut sources = Vec::new();
        let mut source = error.source();
        while let Some(error) = source {
            sources.push(error.to_string());
            source = error.source();
        }
        let frames = self.map_frames(&capture_frames());
        let frames: Vec<_> = frames
            .into_iter()
            .map(|mut frame| {
                frame["snippet"] = json!([]);
                frame
            })
            .collect();
        self.send_request(&[Payload::new(
            "exception",
            json!({
                "class": type_name::<E>(), "message": error.to_string(),
                "frames": frames, "meta": {"sources": sources}
            }),
        )])?
        .red()
    }

    /// Pause until Continue is pressed in Ray. Stop returns [`Error::StopExecutionRequested`].
    /// Network failure resumes execution by default, or returns an error in throws mode.
    /// This blocks the current thread and polls once per second, like PHP.
    #[track_caller]
    pub fn pause(self) -> Result<Self, Error> {
        if !self.can_send || !self.enabled()? {
            return Ok(self);
        }
        let name = Uuid::new_v4().simple().to_string();
        let this = self.send_request(&[Payload::new("create_lock", json!({"name": name}))])?;
        let project = this.client.state()?.project.clone();
        let host = Origin::caller().hostname;
        let path = format!(
            "locks/{name}?hostname={}&project_name={}",
            utf8_percent_encode(&host, NON_ALPHANUMERIC),
            utf8_percent_encode(&project, NON_ALPHANUMERIC)
        );
        loop {
            thread::sleep(Duration::from_secs(1));
            let response = match this.client.get(&path) {
                Ok(value) => value,
                Err(_) if !this.client.config.throws => break,
                Err(error) => return Err(error),
            };
            if response["stop_execution"] == true {
                return Err(Error::StopExecutionRequested);
            }
            if response["active"] != true {
                break;
            }
        }
        Ok(this)
    }

    fn map_frames(&self, frames: &[Frame]) -> Vec<Value> {
        frames
            .iter()
            .map(|frame| {
                json!({
                    "file_name": self.map_path(&frame.file_name),
                    "line_number": frame.line_number, "class": frame.class,
                    "method": frame.method, "vendor_frame": frame.vendor_frame,
                })
            })
            .collect()
    }

    fn map_path(&self, path: &str) -> String {
        if let Some((remote, local)) = &self.client.config.paths
            && let Some(suffix) = path.strip_prefix(remote)
        {
            return format!("{local}{suffix}");
        }
        path.into()
    }

    /// Send custom protocol payloads with the current source location and entry UUID.
    #[track_caller]
    pub fn send_request(self, payloads: &[Payload]) -> Result<Self, Error> {
        if payloads.is_empty() || !self.can_send {
            return Ok(self);
        }
        let mut origin = Origin::caller();
        origin.file = self.map_path(&origin.file);
        let (project, rate_limited, notify) = {
            let mut state = self.client.state()?;
            if !state.enabled {
                drop(state);
                return Ok(self);
            }
            if let Some(key) = &self.limit_origin
                && let Some((count, limit)) = state.limits.get_mut(key)
            {
                if *limit > 0 && *count >= *limit as usize {
                    drop(state);
                    return Ok(self);
                }
                *count += 1;
            }
            let now = Instant::now();
            while state
                .recent
                .front()
                .is_some_and(|hit| now.duration_since(*hit) >= Duration::from_secs(1))
            {
                state.recent.pop_front();
            }
            let limited = state.max.is_some_and(|max| state.hits >= max)
                || state
                    .per_second
                    .is_some_and(|max| state.recent.len() >= max);
            let notify = limited && !state.notified;
            state.notified = limited;
            if !limited {
                state.hits += 1;
                state.recent.push_back(now);
            }
            (state.project.clone(), limited, notify)
        };
        let (payloads, meta) = if rate_limited {
            if !notify {
                return Ok(self);
            }
            (
                vec![Payload::custom(
                    "Rate limit has been reached...",
                    "Rate limit",
                )],
                json!([]),
            )
        } else {
            (
                payloads.to_vec(),
                json!({"project_name": project, "ray_package_version": env!("CARGO_PKG_VERSION"), "language": "rust"}),
            )
        };
        let payloads: Vec<_> = payloads
            .into_iter()
            .map(|payload| {
                json!({
                    "type": payload.kind, "content": payload.content, "origin": origin,
                })
            })
            .collect();
        let request = json!({"uuid": self.uuid, "payloads": payloads, "meta": meta});
        match self.client.post(&request) {
            Err(error) if self.client.config.throws => Err(error),
            _ => Ok(self),
        }
    }
}

fn measure_payload(name: &str, new: bool, total: Duration, lap: Duration) -> Payload {
    Payload::new(
        "measure",
        json!({
            "name": name, "is_new_timer": new,
            "total_time": total.as_secs_f64() * 1000.0,
            "max_memory_usage_during_total_time": 0,
            "time_since_last_call": lap.as_secs_f64() * 1000.0,
            "max_memory_usage_since_last_call": 0,
        }),
    )
}

fn capture_frames() -> Vec<Frame> {
    backtrace::Backtrace::new()
        .frames()
        .iter()
        .flat_map(|frame| frame.symbols())
        .filter_map(|symbol| {
            let method = symbol.name()?.to_string();
            if method.starts_with("spatie_ray::ray::") || method.starts_with("backtrace::") {
                return None;
            }
            Some(Frame {
                file_name: symbol
                    .filename()
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                line_number: symbol.lineno().unwrap_or_default(),
                class: String::new(),
                method,
                vendor_frame: false,
            })
        })
        .collect()
}
