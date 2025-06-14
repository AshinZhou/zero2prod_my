use crate::authentication::UserId;
use crate::utils::e500;
use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use std::fmt::{Debug};
use uuid::Uuid;

pub async fn admin_dashboard(
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>
) -> Result<HttpResponse, actix_web::Error> {
    let username = get_username(&user_id.0, &pool).await.map_err(e500)?;

    Ok(HttpResponse::Ok().content_type(ContentType::html()).body(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Admin dashboard</title>
</head>
<body>
    <p>Welcome {username}!</p>
    <p>Available actions:</p>
    <ol>
        <li><a href="/admin/password">Change password</a></li>
        <li>
          <form name="logoutForm" action="/admin/logout" method="post">
            <input type="submit" value="Logout">
          </form>
        </li>
    </ol>
</body>
</html>"#,
    )))
}


#[tracing::instrument(
    name = "Get username",
    skip(pool)
)]
pub async fn get_username(user_id: &Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
SELECT users.username
FROM users
WHERE user_id = $1
"#,
        user_id
    )
        .fetch_one(pool)
        .await
        .context("Failed to perform a query to retrieve a username.")?;
    Ok(row.username)
}
