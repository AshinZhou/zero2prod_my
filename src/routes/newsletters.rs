use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::routes::error_chain_fmt;
use actix_web::body::BoxBody;
use actix_web::http::header::{HeaderMap, HeaderValue};
use actix_web::http::{header, StatusCode};
use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::PgPool;
use std::fmt::{Debug, Formatter};

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[derive(Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for PublishError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse<BoxBody> {
        match self {
            PublishError::AuthError(_) => { HttpResponse::new(StatusCode::UNAUTHORIZED) }
            PublishError::UnexpectedError(_) => {
                let mut response = HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();

                response.headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);

                response
            }
        }
    }
}

#[tracing::instrument(
    name = "Publish a newsletter issue to all subscribers",
    skip(data, pool, email_client, request),
    fields(
       username = tracing::field::Empty,
       user_id = tracing::field::Empty
    )
)]
pub async fn publish_newsletters(
    data: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest)
    -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;

    tracing::Span::current().record(
        "username",
        &tracing::field::display(&credentials.username),
    );
    let user_id = validate_credentials(credentials, &pool).await?;
    tracing::Span::current().record(
        "user_id",
        &tracing::field::display(&user_id),
    );

    let subscribers = get_subscribers(&pool).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &data.title,
                        &data.content.html,
                        &data.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email.as_ref())
                    })?;
            }
            Err(err) => {
                tracing::warn!(
                    error.cause_chain = ?err,
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid",
                );
            }
        }
    }
    Ok(HttpResponse::Ok().finish())
}

async fn get_subscribers(pool: &PgPool) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers = sqlx::query!(
r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#
)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(err) => Err(anyhow::anyhow!(err)),
        })
        .collect();
    Ok(confirmed_subscribers)
}

struct Credentials {
    username: String,
    password: SecretString,
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF-8 string")?;

    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'")?;

    let decoded_bytes = STANDARD
        .decode(base64encoded_segment)
        .context("Failed to base64-decode 'Basic' header")?;

    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF-8")?;

    let mut credentials = decoded_credentials.splitn(2, ":");

    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A 'Basic' auth credential missing the username"))?
        .to_string();

    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A 'Basic' auth credential missing the password"))?
        .to_string();


    Ok(Credentials {
        username,
        password: SecretString::from(password),
    })
}

#[tracing::instrument(
    name = "Validate credentials",
    skip(credentials, pool)
)]
async fn validate_credentials(credentials: Credentials, pool: &PgPool) -> Result<uuid::Uuid, PublishError> {
    let (user_id, expected_password_hash) = get_stored_credentials(&credentials.username, pool)
        .await
        .map_err(PublishError::UnexpectedError)?
        .ok_or_else(|| PublishError::AuthError(anyhow::anyhow!("Unknown username")))?;
    let expected_password_hash = PasswordHash::new(&expected_password_hash)
        .context("Failed to parse hash in PHC string format")
        .map_err(PublishError::UnexpectedError)?;
    tokio::task::spawn_blocking(move || {
        tracing::info_span!("Verify password hash")
            .in_scope( || {
                Argon2::default()
                    .verify_password(credentials.password.expose_secret().as_bytes(), &expected_password_hash)
            })
    })
        .await
        .context("Invalid password")
        .map_err(PublishError::AuthError)?;
    Ok(user_id)
}

#[tracing::instrument(
    name = "Get stored credentials",
    skip(username, pool)
)]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, String)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
SELECT user_id, password_hash
FROM users
WHERE username = $1
"#,
        username
    )
        .fetch_optional(pool)
        .await
        .context("Failed to perform a query to retrieve stored credentials.")?
        .map(|row| (row.user_id, row.password_hash));
    Ok(row)
}