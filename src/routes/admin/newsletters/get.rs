use actix_web::{get, http::header::ContentType, HttpResponse};
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use std::fmt::Write;

#[get("/newsletters")]
pub async fn publish_newsletter_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut error_html = String::new();
    let idempotency_key = uuid::Uuid::new_v4();
    for message in flash_messages
        .iter()
        .filter(|m| m.level() == Level::Error || m.level() == Level::Info)
    {
        writeln!(error_html, "<p><i>{}</i></p>", message.content()).unwrap();
    }

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
            <html lang="en">
            <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title>Login</title>
            </head>
            <body>
                {error_html}
                <form action="/admin/newsletters" method="post">
                    <label>Title:<br>
                        <input
                            type="text"
                            placeholder="Enter the issue title"
                            name="title"
                        >
                    </label>
                    <br>
                    <label>Plain text content:<br>
                        <textarea
                            placeholder="Enter the content in plain text"
                            name="text_content"
                            rows="20"
                            cols="50"
                        ></textarea>
                    </label>
                    <br>
                    <label>HTML content:<br>
                        <textarea
                            placeholder="Enter the content in HTML format"
                            name="html_content"
                            rows="20"
                            cols="50"
                        ></textarea>
                    </label>
                    <br>
                    <input hidden type="text" name="idempotency_key" value="{idempotency_key}">
                    <button type="submit">Login</button>
                </form>
            </body>
            </html>"#,
        ))
}
