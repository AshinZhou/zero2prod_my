use actix_web::cookie::time::Duration;
use actix_web::cookie::Cookie;
use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;


pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut error_html = String::new();

    // 过滤 error 层级
    // for m in flash_messages.iter().filter(|m| m.level() == Level::Error) {
    // 所有层级
    for m in flash_messages.iter() {
        writeln!(error_html, "<p><i>{}</i></p>", m.content()).unwrap()
    }
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .cookie(
            Cookie::build("_flash", "")
                .max_age(Duration::ZERO)
                .finish()
        )
        .body(format!(r#"
        <!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Login</title>
</head>
<body>
{}
<form action="/login" method="post">
    <label>Username
        <input
                type="text"
                placeholder="Enter Username"
                name="username"
        >
    </label>
    <label>Password
        <input
                type="password"
                name="password"
                placeholder="Enter Password"
        >
    </label>

    <button type="submit">Login</button>
</form>
</body>
</html>
        "#, error_html))
}