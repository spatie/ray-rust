---
title: Using Ray with Rust
weight: 4
---

This page covers how to use Ray with Rust. There are many different methods available, and this page attempts to show most of them. It follows the [PHP usage guide](https://myray.app/docs/php/vanilla-php/usage). See the [API reference](https://docs.rs/spatie-ray) for a full list of methods.

## Basic output

To display something in Ray, use the `ray!` macro. It accepts many different data types: strings, arrays, structs, and more.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
use spatie_ray::{ray, json};

ray!("Hello world")?;
ray!(json!({"a": 1, "b": 2}))?.color("red")?;
# let an_object = json!({"company": "Spatie"});
ray!(an_object)?;
# Ok(())
# }
```

The `ray!` macro accepts multiple arguments. These are displayed as separate messages in the same entry.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!("as", "many", "arguments", "as", "you", "like")?;
# Ok(())
# }
```

Values must implement `serde::Serialize`. Derive it on your own types, or use `ray!().debug(&value)` for a type implementing `Debug`. Rust arrays, structs and floating-point values use Ray's native JSON renderer instead of PHP's reflection-based HTML dump. [Compatibility](compatibility.md) records the differences.

## Customizing output

Ray has some useful methods that you can use to add more context or differentiate messages from one another.

### Expanding arrays and objects

Ray's `expand` command opens nodes in compatible renderers. Pass a depth to `expand`, or use `expand_keys` for names and dot notation. These commands are primarily intended for custom payloads that use Ray's expandable HTML renderer; native JSON expansion is controlled by the app.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::{ray, json};
# let array_or_object = json!({"myKey": {"nestedKey": 1}, "anotherKey": 2});
ray!(&array_or_object)?.expand(1)?;
ray!(&array_or_object)?.expand(3)?;
ray!(&array_or_object)?.expand_keys(&["myKey", "anotherKey"])?;
ray!(&array_or_object)?.expand_keys(&["myKey.nestedKey"])?;
ray!(&array_or_object)?.expand_all()?;
# Ok(())
# }
```

### Using colors

You can colorize things you sent to Ray by using one of the color functions.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!("this is green")?.green()?;
ray!("this is yellow")?.yellow()?;
ray!("this is red")?.red()?;
ray!("this is blue")?.blue()?;
ray!("this is purple")?.purple()?;
ray!("this is gray")?.gray()?;
# Ok(())
# }
```

Like PHP, `yellow()` sends the `orange` color. `grey()` is an alias for `gray()`.

### Using sizes

Ray can display things in different sizes.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!("small")?.small()?;
ray!("regular")?;
ray!("large")?.large()?;
# Ok(())
# }
```

### Adding a label

You can customize the label displayed next to an item with the `label` function.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!(["John", "Paul", "George", "Ringo"])?.label("Beatles")?;
# Ok(())
# }
```

### Displaying a table

You can send a map or array to Ray with the `table` function.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::{ray, json};
ray!().table(json!({
    "First": "First value",
    "Second": "Second value",
    "Third": "Third value"
}))?;

ray!().table_with_label(["John", "Paul", "George", "Ringo"], "Beatles")?;
# Ok(())
# }
```

Nested table cells are escaped, preformatted JSON. Top-level strings, integers, booleans and null match PHP's table payloads.

## Output formats

### Working with JSON

Want to display the JSON representation of anything you'd like in Ray? Use `to_json`. You can provide any value that can be serialized with Serde.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::{ray, json};
ray!().to_json(json!({"a": 1, "b": {"c": 3}}))?;
# Ok(())
# }
```

You can send a valid JSON string to Ray with the `json` function. It will be displayed using Ray's JSON renderer. Unlike PHP's `json`, invalid JSON returns an error instead of displaying null.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
let json_string = r#"{"a": 1, "b": {"c": 3}}"#;
ray!().json(json_string)?;
# Ok(())
# }
```

Rust has no variadic methods. Chain `to_json` or `json` for multiple values, or use `send_many(&[Value])` to send multiple serializable values in one request.

### Working with XML

You can send a valid XML string to Ray with the `xml` function. It will be displayed as formatted XML. Invalid XML returns an error. XML entity spelling and declarations may differ from PHP's DOM formatter for complex documents.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
let xml_string = "<one><two><three>3</three></two></one>";
ray!().xml(xml_string)?;
# Ok(())
# }
```

### Working with files

You can display the contents of any UTF-8 text file in Ray with the `file` function. A missing file is displayed as a message. Other file errors are returned.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!().file("somefile.txt")?;
# Ok(())
# }
```

### Displaying images

To display an image, call the `image` function and pass a fully-qualified filename, URL, or a valid base64-encoded image as its only argument.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!().image("https://placekitten.com/200/300")?;
ray!().image("/home/user/kitten.jpg")?;
ray!().image("data:image/png;base64,aGVsbG8=")?;
ray!().image("aGVsbG8=")?;
# Ok(())
# }
```

The short base64 examples demonstrate transport syntax, not a valid image. Supply actual image bytes when displaying one. Like PHP, base64 input is sent with an `image/png` data prefix, even when the supplied data prefix used another image type.

### Displaying a link

You can render a clickable link in Ray by using the `link` or `url` methods. Pass an empty label to use the URL.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!().link("spatie.be", "")?;
ray!().link("spatie.be", "Spatie homepage")?;
ray!().url("myray.app", "")?;
# Ok(())
# }
```

We assume that `spatie.be` means `https://spatie.be`. URL and label are escaped rather than interpreted as HTML; use `html` for trusted markup.

### Rendering HTML

To render a piece of HTML directly in Ray, you can use the `html` method.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!().html("<b>Bold string</b>")?;
# Ok(())
# }
```

### Displaying text content

To display raw text while preserving whitespace formatting, use the `text` method. If the text contains HTML, it will be displayed as-is and is not rendered.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!().text("<em>this string is html encoded</em>")?;
ray!().text("  whitespace formatting\n   is preserved as well.")?;
# Ok(())
# }
```

## Inspecting values

### Display the type name of an object

To quickly send the Rust type name of a value to Ray, use the `class_name` function.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::{ray, json};
# let an_object = json!({"company": "Spatie"});
ray!().class_name(&an_object)?;
# Ok(())
# }
```

### Showing raw values

Use `raw` to bypass specialized boolean/null handling. Structured values use preformatted JSON, not PHP's private-property reflection. For `Debug`-only values, use `debug`.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!().raw(true)?;
ray!().debug(&std::time::Duration::from_millis(25))?;
# Ok(())
# }
```

PHP runtime information, Carbon-specific formatting and private-property invasion have no direct Rust equivalent and are intentionally omitted.

## Caller & stack trace

### See the caller of a function

Sometimes you want to know where your code is being called. You can determine that by using the `caller` function. To see the entire backtrace, use `trace` or `backtrace`.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!().caller()?;
ray!().trace()?;
# Ok(())
# }
```

Stack resolution depends on debug symbols. The macro's file/line metadata remains available even without them. `trace_frames` and `caller_frame` also accept explicit protocol frames.

## Measuring & counting

### Counting execution times

You can display a count of how many times a piece of code was called using `count`.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
for _ in 1..=2 {
    ray!().count()?;
    for _ in 1..=4 {
        ray!().count()?;
    }
}
# Ok(())
# }
```

Use `count_named` to share a named counter. Read its value with `counter_value`. All calls for a given counter update the same entry. `clear_counters` resets the session's counters.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::{Client, Config};
let client = Client::new(Config::default());
for _ in 1..=4 {
    client.ray().count_named("first")?;
    if client.ray().counter_value("first")? == 2 {
        println!("counter value is two!");
    }
}
# assert_eq!(client.ray().counter_value("first")?, 4);
# Ok(())
# }
```

### Measuring performance

You can use `measure` to display runtime. When `measure` is called again, the time between this and the previous call is also displayed. An empty name selects the default timer. `stop_time("")` resets all timers; pass a name to reset just one.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!().measure("")?;
// Do some work.
ray!().measure("")?;
ray!().measure_with(|| {
    let _ = (1..=100).sum::<u64>();
})?;
# Ok(())
# }
```

Durations are measured with a monotonic clock and sent in milliseconds, like PHP. PHP heap-memory measurements are not portable to Rust; the corresponding protocol fields are zero. Panics in the closure are not caught.

## Limiting output

### Limiting the number of sent payloads

To limit the number of requests sent by a particular callsite, use `limit`. It works well for debugging loops.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
for i in 1..=10 {
    ray!().limit(3)?.text(&format!("A #{i}"))?;
    ray!().limit(6)?.text(&format!("B #{i}"))?;
    ray!().text(&format!("C #{i}"))?;
}
# Ok(())
# }
```

If the limit is zero or negative, limiting is disabled. Like PHP, each chained request, including color and label changes, consumes one allowance. The first limit for a source location is retained for the life of the client.

### Using a rate limiter

A rate limiter reduces the number of sent messages. Configure it on the shared client session:

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::{Client, Config};
let client = Client::new(Config::default());
client.max(Some(10))?;
client.per_second(Some(10))?;
client.ray().send("limited output")?;
client.clear_rate_limits()?;
# Ok(())
# }
```

Ray displays one notification when a limit is reached. `None` disables a particular rate limit. Per-second limits use a sliding one-second window. Failed network sends count toward limits, matching PHP's attempt-based behavior.

### Sending a payload once

To only send a payload once from a source location, use `once`.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
for i in 1..=10 {
    ray!().once()?.send(i)?;
}
for i in 1..=10 {
    ray!().once()?.html(&format!("<strong>{i}</strong>"))?;
}
# Ok(())
# }
```

## Conditional output

### Conditionally showing items

`show_if` removes the entry when the condition is false. `remove_when` removes it when the condition is true.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!("will be shown")?.show_if(true)?;
ray!("will not be shown")?.show_if(false)?;
# Ok(())
# }
```

### Conditionally sending items to Ray

Use `when` to gate all following sends. A later call replaces the gate. Use `when_with` for a conditional callback that does not change the gate of later methods.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
for number in 1..=10 {
    ray!().when(number < 10).text(&format!("value is less than ten: {number}"))?.blue()?;
    ray!().text(&format!("value: {number}"))?
        .when_with(number % 2 == 0, |ray| ray.large())?
        .green()?;
}
# Ok(())
# }
```

## Handling exceptions

### Displaying errors

You can display information about a Rust error in Ray with `exception`.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
let error = std::io::Error::other("test");
ray!().exception(&error)?;
# Ok(())
# }
```

This sends the Rust type name, message, source chain and an observation-site stack trace. Rust errors do not retain PHP-style throw sites or snippets. Handle `Result` with ordinary Rust control flow; deferred exception queues and shutdown rethrows are not emulated.

## Updating & removing items

### Updating displayed items

You can update values that are already displayed in Ray. Hold on to the returned instance and call `send` on it.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
let mut ray = ray!("counting down!")?;
for number in (1..=10).rev() {
    ray = ray.send(number)?;
}
ray.red()?.large()?;
# Ok(())
# }
```

Fluent methods consume and return the handle. Cloning a handle keeps the same UUID; creating a new one creates a new entry.

### Removing items

To remove an item that is already displayed in Ray, call `remove` on the returned instance.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
let ray = ray!("will be removed")?;
ray.remove()?;
# Ok(())
# }
```

## Execution control

### Pausing execution

You can pause the current thread by using `pause`. Press Continue in Ray to resume. Stop execution returns `Error::StopExecutionRequested`; propagate it to stop your operation. The library never kills your process.

This example requires a running Ray app and is compile-checked, not executed by rustdoc. Continue and Stop are exercised by the HTTP tests.

```rust,no_run
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!().pause()?;
# Ok(())
# }
```

Network failure resumes execution unless `throws(true)` is set. The client polls once per second with the current hostname and project name. This is a blocking API; use it off your async executor's worker threads.

## Flow helpers

### Returning items

To send something to Ray and have that same value returned, use `pass`. This is handy when debugging return values.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
let value = ray!().pass("return value")?;
# assert_eq!(value, "return value");
# Ok(())
# }
```

## Custom functions

Use ordinary Rust functions or extension traits instead of dynamically registered methods. Send custom content with `send_custom`. `send_request` and `Payload::new` are available for protocol extensions.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!().send_custom("my custom content", "hey")?;
# Ok(())
# }
```

## Managing the Ray app

### Creating a new screen

You can use `new_screen` or `clear_screen` to programmatically create a new screen. Your previous messages are automatically saved and moved to the message archive. Pass an empty name for an unnamed screen.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!().new_screen("")?;
ray!().new_screen("My debug screen")?;
ray!().clear_screen()?;
ray!().clear_all()?;
# Ok(())
# }
```

`clear_all` also clears previous screens. Screen color helpers include `screen_green`, `screen_red`, `screen_blue`, `screen_orange`, `screen_purple` and `screen_gray`.

### Showing and hiding the app

You can show and hide the Ray app via code.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!().show_app()?;
ray!().hide_app()?;
# Ok(())
# }
```

Use `client.windows()` and `client.theme()` to query the app. Both return the server's JSON and report transport failures.

### Enabling & disabling Ray

You can enable and disable sending stuff to Ray with `enable` and `disable`. This affects all handles sharing the same client.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::{Client, Config};
let client = Client::new(Config::default());
client.ray().send("one")?;
client.ray().disable()?;
client.ray().send("two")?;
# assert!(client.ray().disabled()?);
client.ray().enable()?;
client.ray().send("three")?;
# assert!(client.ray().enabled()?);
# Ok(())
# }
```

## Fun stuff

### Displaying a notification

You can use Ray to display a system notification.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!().notify("This is my notification")?;
# Ok(())
# }
```

### Shooting confetti

For those times when success is to be celebrated.

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::ray;
ray!().confetti()?;
# Ok(())
# }
```
