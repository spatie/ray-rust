//! Emits the same portable operations as tests/protocol/php.php.

use spatie_ray::{Client, Error, json};

pub fn send(client: &Client) -> Result<(), Error> {
    let ray = client
        .ray()
        .send_many(&[
            json!("Hello world"),
            json!(42),
            json!(true),
            json!(null),
            json!(false),
            json!(0),
            json!(""),
        ])?
        .green()?
        .orange()?
        .yellow()?
        .red()?
        .purple()?
        .blue()?
        .gray()?
        .grey()?
        .small()?
        .large()?
        .label("Beatles")?
        .send("updated")?
        .hide()?
        .remove()?
        .new_screen("My debug screen")?
        .screen_green()?
        .screen_orange()?
        .screen_red()?
        .screen_purple()?
        .screen_blue()?
        .screen_gray()?
        .clear_screen()?
        .clear_all()?
        .separator()?
        .notify("This is my notification")?
        .confetti()?
        .show_app()?
        .hide_app()?
        .html("<b>Bold string</b>")?
        .text("  <em>A & B's \"text\"</em>\n   preserved")?
        .send_custom("my custom content", "hey")?
        .link("spatie.be", "Spatie homepage")?
        .table(json!({"First": "First value", "Second": 2, "Third": true, "Last": null}))?
        .table_with_label(["John", "Paul", "George", "Ringo"], "Beatles")?
        .to_json(json!({"a": 1, "b": {"c": 3}, "url": "https://spatie.be", "name": "Zoë 🎉"}))?
        .image("https://placekitten.com/200/300")?
        .image("aGVsbG8=")?
        .image("data:image/jpeg;base64,aGVsbG8=")?
        .expand(3)?
        .expand_keys(&["myKey", "anotherKey.nestedKey"])?
        .expand_all()?
        .raw(true)?
        .raw(None::<u8>)?
        .raw("raw")?
        .send(11111111111111111_u64)?
        .xml("<one><two><three>3</three></two></one>")?
        .xml("<one/>")?
        .file("tests/fixtures/content.txt")?
        .file("tests/fixtures/missing.txt")?;
    ray.disable()?
        .send("not sent")?
        .enable()?
        .when(false)
        .send("not sent either")?
        .when(true)
        .text("shown")?
        .remove_when(false)?
        .show_if(false)?;
    client.ray().count_named("first")?;
    client.ray().count_named("first")?;
    client.ray().count_named("second")?;
    for i in 1..=4 {
        client.ray().limit(2)?.send(i)?;
    }
    for i in 1..=3 {
        client.ray().once()?.send(i)?;
    }
    client.clear_rate_limits()?.max(Some(1))?;
    client.ray().send("last allowed")?;
    client.ray().send("rate limited")?;
    client.ray().send("rate limited again")?;
    Ok(())
}
