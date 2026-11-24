use crate::{Error, json};
use base64::{Engine, prelude::BASE64_STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{io::Cursor, panic::Location, path::Path};

/// Source location used by Ray's clickable editor links.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Origin {
    /// Source filename.
    pub file: String,
    /// One-based source line.
    pub line_number: u32,
    /// Machine running the application.
    pub hostname: String,
}

impl Origin {
    #[track_caller]
    pub(crate) fn caller() -> Self {
        let location = Location::caller();
        Self {
            file: location.file().into(),
            line_number: location.line(),
            hostname: hostname::get()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }
    }
}

/// A stack frame in the Ray protocol. Rust has no PHP class or vendor-frame concept.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Frame {
    /// Source filename, when debug symbols are available.
    pub file_name: String,
    /// One-based source line, or zero when unavailable.
    pub line_number: u32,
    /// Empty for Rust stack frames.
    pub class: String,
    /// Rust symbol name.
    pub method: String,
    /// Whether this is a dependency frame, when known by the caller.
    pub vendor_frame: bool,
}

/// A Ray protocol payload, before an origin is attached.
///
/// This is also the extension point for custom Ray renderers; no runtime
/// method registration or inheritance is needed.
#[derive(Clone, Debug, PartialEq)]
pub struct Payload {
    pub(crate) kind: String,
    pub(crate) content: Value,
}

impl Payload {
    /// Construct a protocol payload. `content` must follow the Ray app's schema.
    #[must_use]
    pub fn new(kind: impl Into<String>, content: Value) -> Self {
        Self {
            kind: kind.into(),
            content,
        }
    }

    pub(crate) fn empty(kind: &str) -> Self {
        // PHP's empty getContent(): array encodes as [], not {}.
        Self::new(kind, json!([]))
    }

    pub(crate) fn custom(content: impl Into<Value>, label: &str) -> Self {
        Self::new("custom", json!({"content": content.into(), "label": label}))
    }

    pub(crate) fn value(value: Value, raw: bool) -> Result<Self, Error> {
        if raw {
            let rendered = primitive(value)?;
            return Ok(Self::new(
                "log",
                json!({"values": [rendered], "meta": [{"clipboard_data": "[],"}]}),
            ));
        }
        match value {
            Value::Bool(_) => Ok(Self::custom(value, "Boolean")),
            Value::Null => Ok(Self::custom(value, "Null")),
            Value::Array(_) | Value::Object(_) => Self::json_string(&value),
            Value::Number(ref number) if number.is_f64() => Self::json_string(&value),
            _ => {
                let clipboard = match &value {
                    Value::String(text) => text.clone(),
                    _ => value.to_string(),
                };
                let clipboard: String = clipboard.chars().take(20000).collect();
                let value = if value.as_i64().is_some_and(|n| n >= 11111111111111111)
                    || value.as_u64().is_some_and(|n| n >= 11111111111111111)
                {
                    Value::String(value.to_string())
                } else {
                    value
                };
                Ok(Self::new(
                    "log",
                    json!({"values": [value], "meta": [{"clipboard_data": clipboard}]}),
                ))
            }
        }
    }

    pub(crate) fn json_string(value: &Value) -> Result<Self, Error> {
        Ok(Self::new("json_string", json!({"value": php_json(value)?})))
    }
}

pub(crate) fn php_json(value: &Value) -> Result<String, Error> {
    let json = serde_json::to_string(value)?.replace('/', "\\/");
    let mut encoded = String::new();
    for ch in json.chars() {
        if ch.is_ascii() {
            encoded.push(ch);
        } else {
            use std::fmt::Write;
            for unit in ch.encode_utf16(&mut [0; 2]) {
                // Writing to a String is infallible.
                let _ = write!(encoded, "\\u{unit:04x}");
            }
        }
    }
    Ok(encoded)
}

pub(crate) fn escape(text: &str, html5: bool) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', if html5 { "&apos;" } else { "&#039;" })
}

pub(crate) fn primitive(value: Value) -> Result<Value, Error> {
    match value {
        Value::Array(_) | Value::Object(_) | Value::Number(_)
            if !value.is_i64() && !value.is_u64() =>
        {
            Ok(Value::String(format!(
                "<pre>{}</pre>",
                escape(&serde_json::to_string_pretty(&value)?, true)
            )))
        }
        _ => Ok(value),
    }
}

pub(crate) fn image(location: &str) -> Payload {
    let data = location
        .strip_prefix("data:image/")
        .and_then(|rest| rest.split_once(";base64,"))
        .map_or(location, |(_, data)| data);
    let location = if Path::new(location).is_file() {
        format!("file://{location}")
    } else if BASE64_STANDARD
        .decode(data)
        .is_ok_and(|decoded| BASE64_STANDARD.encode(decoded) == data)
    {
        format!("data:image/png;base64,{data}")
    } else {
        location.into()
    };
    Payload::custom(
        format!("<img src=\"{}\" alt=\"\" />", location.replace('"', "")),
        "Image",
    )
}

pub(crate) fn xml(xml: &str) -> Result<String, Error> {
    use quick_xml::{Reader, Writer, events::Event};
    roxmltree::Document::parse(xml).map_err(|e| Error::Xml(e.to_string()))?;
    let mut reader = Reader::from_str(xml);
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);
    let mut has_declaration = false;
    loop {
        let event = reader.read_event().map_err(|e| Error::Xml(e.to_string()))?;
        match event {
            Event::Eof => break,
            Event::Decl(_) => has_declaration = true,
            Event::Text(ref text) if text.as_ref().iter().all(u8::is_ascii_whitespace) => continue,
            _ => {}
        }
        writer
            .write_event(event)
            .map_err(|e| Error::Xml(e.to_string()))?;
    }
    let formatted = String::from_utf8(writer.into_inner().into_inner())
        .map_err(|e| Error::Xml(e.to_string()))?;
    let formatted = if has_declaration {
        formatted
    } else {
        format!("<?xml version=\"1.0\"?>\n{formatted}")
    };
    Ok(escape(formatted.trim(), false)
        .replace('\n', "<br>")
        .replace(' ', "&nbsp;"))
}
