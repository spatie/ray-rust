//! Observable HTTP protocol and session behavior.

#[path = "common/scenario.rs"]
mod scenario;

use spatie_ray::{Client, Config, Error, Frame, Payload, Value, json, ray};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use tiny_http::{Header, Response, Server};

struct Capture {
    client: Client,
    requests: Receiver<(String, Value)>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Capture {
    fn new(config: Config, respond: impl Fn(&str) -> (u16, Value) + Send + 'static) -> Self {
        let server = Server::http("127.0.0.1:0").unwrap();
        let port = server.server_addr().to_ip().unwrap().port();
        let (tx, requests) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::Relaxed) {
                if let Some(mut request) = server.recv_timeout(Duration::from_millis(20)).unwrap() {
                    let path = request.url().to_owned();
                    let mut body = String::new();
                    request.as_reader().read_to_string(&mut body).unwrap();
                    let value = if body.is_empty() {
                        Value::Null
                    } else {
                        serde_json::from_str(&body).unwrap()
                    };
                    tx.send((path.clone(), value)).unwrap();
                    let (status, value) = respond(&path);
                    request
                        .respond(
                            Response::from_string(value.to_string())
                                .with_status_code(status)
                                .with_header(
                                    Header::from_bytes("Content-Type", "application/json").unwrap(),
                                ),
                        )
                        .unwrap();
                }
            }
        });
        Self {
            client: Client::new(config.host("127.0.0.1").port(port)),
            requests,
            stop,
            worker: Some(worker),
        }
    }

    fn ok() -> Self {
        Self::new(Config::default().throws(true).project("Protocol"), |_| {
            (200, json!({}))
        })
    }

    fn next(&self) -> (String, Value) {
        self.requests.recv_timeout(Duration::from_secs(5)).unwrap()
    }

    fn body(&self) -> Value {
        self.next().1
    }

    fn empty(&self) {
        assert!(self.requests.try_recv().is_err());
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.worker.take().unwrap().join().unwrap();
    }
}

#[test]
fn portable_operations_match_real_php_http_captures() {
    let capture = Capture::ok();
    scenario::send(&capture.client).unwrap();
    let expected: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/php-wire.json")).unwrap();
    let mut uuids = HashMap::new();
    let actual: Vec<_> = (0..expected.len())
        .map(|_| {
            let (path, mut body) = capture.next();
            assert_eq!(path, "/");
            let next = uuids.len();
            let uuid = body["uuid"].as_str().unwrap().to_owned();
            body["uuid"] = json!(*uuids.entry(uuid).or_insert(next));
            if body["meta"].is_object() {
                body["meta"] = json!({"project_name": body["meta"]["project_name"]});
            }
            for payload in body["payloads"].as_array_mut().unwrap() {
                payload.as_object_mut().unwrap().shift_remove("origin");
            }
            body
        })
        .collect();
    assert_eq!(actual, expected);
    capture.empty();
}

#[test]
fn macro_evaluates_once_and_preserves_callsite_and_update_uuid() {
    let capture = Capture::ok();
    let mut times = 0;
    let line = line!() + 1;
    let ray = ray!(client: capture.client, { times += 1; "hello" }, false).unwrap();
    let uuid = ray.uuid().to_owned();
    ray.red().unwrap().send(9).unwrap();
    let body = capture.body();
    assert_eq!(times, 1);
    assert_eq!(body["payloads"].as_array().unwrap().len(), 2);
    assert_eq!(body["payloads"][0]["origin"]["line_number"], line);
    assert_eq!(body["payloads"][0]["origin"]["file"], file!());
    assert_eq!(body["uuid"], uuid);
    assert_eq!(capture.body()["uuid"], uuid);
    assert_eq!(capture.body()["uuid"], uuid);
    assert_ne!(capture.client.ray().uuid(), uuid);
}

#[test]
fn structured_values_use_native_json_and_debug_text_is_escaped() {
    let capture = Capture::ok();
    let value = json!({"a": [1, {"b": "<script>"}], "nil": null});
    capture.client.ray().send(&value).unwrap();
    let body = capture.body();
    assert_eq!(body["payloads"][0]["type"], "json_string");
    assert_eq!(
        serde_json::from_str::<Value>(body["payloads"][0]["content"]["value"].as_str().unwrap())
            .unwrap(),
        value
    );
    capture.client.ray().debug(&"<script>").unwrap();
    assert_eq!(
        capture.body()["payloads"][0]["content"]["content"],
        "&quot;&lt;script&gt;&quot;"
    );
    assert!(matches!(
        capture.client.ray().json("{"),
        Err(Error::Json(_))
    ));
}

#[test]
fn session_state_is_shared_but_independent_clients_are_isolated() {
    let capture = Capture::ok();
    capture.client.ray().disable().unwrap();
    capture.client.clone().ray().send("not sent").unwrap();
    assert!(capture.client.ray().disabled().unwrap());
    assert!(Client::default().ray().enabled().unwrap());
    capture
        .client
        .ray()
        .enable()
        .unwrap()
        .when(false)
        .send("hidden")
        .unwrap()
        .when(true)
        .when_with(false, |ray| ray.red())
        .unwrap()
        .text("visible")
        .unwrap();
    assert_eq!(
        capture.body()["payloads"][0]["content"]["content"],
        "visible"
    );
    capture.empty();
}

#[test]
fn callsite_limits_are_independent_and_nonpositive_limits_are_unlimited() {
    let capture = Capture::ok();
    for i in 0..4 {
        capture.client.ray().limit(2).unwrap().send(i).unwrap();
        capture.client.ray().once().unwrap().send("once").unwrap();
        capture.client.ray().limit(0).unwrap().send("zero").unwrap();
        capture
            .client
            .ray()
            .limit(-2)
            .unwrap()
            .send("negative")
            .unwrap();
    }
    let values: Vec<_> = (0..11)
        .map(|_| capture.body()["payloads"][0]["content"].clone())
        .collect();
    assert_eq!(
        values.iter().filter(|v| v["values"][0] == "once").count(),
        1
    );
    assert_eq!(
        values.iter().filter(|v| v["values"][0] == "zero").count(),
        4
    );
    assert_eq!(
        values
            .iter()
            .filter(|v| v["values"][0] == "negative")
            .count(),
        4
    );
    capture.empty();
}

#[test]
fn named_counters_share_an_entry_and_reset() {
    let capture = Capture::ok();
    capture.client.ray().count_named("one").unwrap();
    capture.client.ray().count_named("two").unwrap();
    capture.client.ray().count_named("one").unwrap();
    let first = capture.body();
    let second = capture.body();
    let third = capture.body();
    assert_eq!(first["uuid"], third["uuid"]);
    assert_ne!(first["uuid"], second["uuid"]);
    assert_eq!(
        third["payloads"][0]["content"]["content"],
        "Called `one` 2 times."
    );
    assert_eq!(capture.client.ray().counter_value("one").unwrap(), 2);
    capture.client.ray().clear_counters().unwrap();
    assert_eq!(capture.client.ray().counter_value("one").unwrap(), 0);
}

#[test]
fn total_and_sliding_rate_limits_notify_once_and_recover() {
    let capture = Capture::ok();
    capture.client.per_second(Some(1)).unwrap();
    capture.client.ray().send("allowed").unwrap();
    capture.client.ray().send("blocked").unwrap();
    capture.client.ray().send("also blocked").unwrap();
    assert_eq!(
        capture.body()["payloads"][0]["content"]["values"][0],
        "allowed"
    );
    assert_eq!(
        capture.body()["payloads"][0]["content"]["label"],
        "Rate limit"
    );
    capture.empty();
    thread::sleep(Duration::from_millis(1050));
    capture.client.ray().send("recovered").unwrap();
    assert_eq!(
        capture.body()["payloads"][0]["content"]["values"][0],
        "recovered"
    );
    capture
        .client
        .clear_rate_limits()
        .unwrap()
        .max(Some(0))
        .unwrap();
    capture.client.ray().send("blocked at zero").unwrap();
    assert_eq!(
        capture.body()["payloads"][0]["content"]["label"],
        "Rate limit"
    );
    capture.client.clear_rate_limits().unwrap();
    capture.client.ray().send("reset").unwrap();
    assert_eq!(
        capture.body()["payloads"][0]["content"]["values"][0],
        "reset"
    );
}

#[test]
fn transport_errors_are_optional_but_read_failures_are_explicit() {
    let capture = Capture::new(Config::default(), |_| (503, json!({})));
    assert!(capture.client.ray().send("best effort").is_ok());
    assert!(capture.client.windows().is_err());
    let strict = Capture::new(Config::default().throws(true), |_| (503, json!({})));
    assert!(matches!(
        strict.client.ray().send("strict"),
        Err(Error::Transport(_))
    ));
}

#[test]
fn xml_rejects_malformed_documents_without_losing_mixed_content_spaces() {
    let capture = Capture::ok();
    for xml in ["<one>", "<one></two>", "<a/><b/>", "not XML"] {
        assert!(matches!(capture.client.ray().xml(xml), Err(Error::Xml(_))));
    }
    capture
        .client
        .ray()
        .xml("<p>Hello <b>Ray</b> !</p>")
        .unwrap();
    let body = capture.body();
    let content = body["payloads"][0]["content"]["content"].as_str().unwrap();
    assert!(content.contains("Hello&nbsp;"), "{content}");
    assert!(content.contains("&nbsp;!"), "{content}");
    capture.empty();
}

#[test]
fn app_queries_return_server_json() {
    let capture = Capture::new(Config::default(), |path| match path {
        "/windows" => (200, json!([{"name": "Main", "id": 7}])),
        "/theme" => (200, json!({"theme": "dark"})),
        _ => (404, json!({})),
    });
    assert_eq!(
        capture.client.windows().unwrap(),
        json!([{"name": "Main", "id": 7}])
    );
    assert_eq!(capture.client.theme().unwrap(), json!({"theme": "dark"}));
}

#[test]
fn pause_resumes_and_stop_is_an_error() {
    let resume = Capture::new(Config::default().throws(true).project("A & B"), |_| {
        (200, json!({"active": false}))
    });
    resume.client.ray().pause().unwrap();
    let lock = resume.body()["payloads"][0]["content"]["name"]
        .as_str()
        .unwrap()
        .to_owned();
    let (url, _) = resume.next();
    assert!(url.starts_with(&format!("/locks/{lock}?hostname=")));
    assert!(url.ends_with("&project_name=A%20%26%20B"));
    let stop = Capture::new(Config::default().throws(true), |_| {
        (200, json!({"active": false, "stop_execution": true}))
    });
    assert!(matches!(
        stop.client.ray().pause(),
        Err(Error::StopExecutionRequested)
    ));
}

#[test]
fn timers_lap_and_can_be_reset() {
    let capture = Capture::ok();
    capture.client.ray().measure("first").unwrap();
    thread::sleep(Duration::from_millis(10));
    capture.client.ray().measure("first").unwrap();
    capture
        .client
        .ray()
        .stop_time("first")
        .unwrap()
        .measure("first")
        .unwrap();
    let first = capture.body()["payloads"][0]["content"].clone();
    let second = capture.body()["payloads"][0]["content"].clone();
    let reset = capture.body()["payloads"][0]["content"].clone();
    assert_eq!(first["is_new_timer"], true);
    assert_eq!(first["total_time"], 0.0);
    assert_eq!(second["is_new_timer"], false);
    assert!(second["total_time"].as_f64().unwrap() >= 10.0);
    assert_eq!(reset["is_new_timer"], true);
}

#[test]
fn paths_are_mapped_in_origins_and_frames_and_custom_payloads_work() {
    let capture = Capture::new(
        Config::default()
            .remote_path("tests", "/local")
            .throws(true),
        |_| (200, json!({})),
    );
    let frame = Frame {
        file_name: "tests/other.rs".into(),
        line_number: 21,
        class: String::new(),
        method: "work".into(),
        vendor_frame: false,
    };
    capture
        .client
        .ray()
        .trace_frames(&[frame])
        .unwrap()
        .send_request(&[Payload::new(
            "custom",
            json!({"content": "hello", "label": "extension"}),
        )])
        .unwrap();
    let body = capture.body();
    assert_eq!(
        body["payloads"][0]["origin"]["file"]
            .as_str()
            .unwrap()
            .replace('\\', "/"),
        "/local/ray.rs"
    );
    assert_eq!(
        body["payloads"][0]["content"]["frames"][0]["file_name"],
        "/local/other.rs"
    );
    assert_eq!(
        capture.body()["payloads"][0]["content"]["label"],
        "extension"
    );
}

#[test]
fn error_and_stack_payloads_carry_rust_information() {
    let capture = Capture::ok();
    let error = std::io::Error::other("something broke");
    capture.client.ray().exception(&error).unwrap();
    let body = capture.body();
    assert_eq!(body["payloads"][0]["type"], "exception");
    assert_eq!(body["payloads"][0]["content"]["message"], "something broke");
    assert!(
        !body["payloads"][0]["content"]["frames"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(capture.body()["payloads"][0]["content"]["color"], "red");
    capture.client.ray().trace().unwrap();
    let body = capture.body();
    let frames = body["payloads"][0]["content"]["frames"].as_array().unwrap();
    assert!(frames.iter().any(|frame| {
        frame["method"]
            .as_str()
            .unwrap()
            .contains("error_and_stack_payloads")
    }));
}
