use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse};

#[derive(serde::Deserialize)]
pub struct QueryParams {
    pub error: String,
    tag: String,
}

pub async fn login_form(query: Option<web::Query<QueryParams>>) -> HttpResponse {
    let error_form = match query {
        None => { "".into() }
        Some(query) => {
            format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&query.0.error))
        }
    };
    HttpResponse::Ok()
        .content_type(ContentType::html())
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
        "#, error_form))
}