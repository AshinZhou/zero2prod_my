use crate::telemetry::spawn_blocking_with_tracing;
use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials ")]
    InvalidCredentials(#[source] anyhow::Error),    
    #[error(transparent)]
    InternalError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub(crate) username: String,
    pub(crate) password: SecretString,
}
#[tracing::instrument(
    name = "Validate credentials",
    skip(credentials, pool)
)]
pub async fn validate_credentials(credentials: Credentials, pool: &PgPool) -> Result<uuid::Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_password_hash = SecretString::from("$argon2id$v=19$m=15000,t=2,p=1$goV5w/49LP4yuat+AN4pIQ$lY3KAqM/9V7ds+i9QwIAeOF+AAowx5ysp+ksRHtk0os");
    if let Some((stored_user_id, stored_password_hash)) = get_stored_credentials(&credentials.username, pool)
        .await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }
    spawn_blocking_with_tracing(move ||
        verify_password_hash(expected_password_hash, credentials.password)
    )
        .await
        .context("Invalid password")??;

    user_id.ok_or_else(|| AuthError::InvalidCredentials(anyhow::anyhow!("Unknown username")))
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, provided_password)
)]
fn verify_password_hash(
    expected_password_hash: SecretString,
    provided_password: SecretString,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(&expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")?;
    Argon2::default()
        .verify_password(provided_password.expose_secret().as_bytes(), &expected_password_hash)
        .map_err(|_err| AuthError::InvalidCredentials(anyhow::anyhow!("Invalid password")))
}

#[tracing::instrument(
    name = "Get stored credentials",
    skip(username, pool)
)]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, SecretString)>, anyhow::Error> {
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
        .map(|row| (row.user_id, SecretString::from(row.password_hash)));
    Ok(row)
}
fn compute_password_hash(
    password: SecretString
) -> Result<SecretString, anyhow::Error> {
    let salt = SaltString::generate(&mut rand::thread_rng());

    let password_hash = argon2::Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(15000, 2, 1, None).unwrap(),
    )
        .hash_password(password.expose_secret().as_bytes(), &salt)?
        .to_string();

    Ok(SecretString::from(password_hash))
}

#[tracing::instrument(
name= "Change password"
skip(password, pool)
)]
pub async fn change_password(
    user_id: Uuid,
    password: SecretString,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    let password_hash = spawn_blocking_with_tracing(
        move || compute_password_hash(password)
    ).await?
        .context("Failed to hash password")?;

    sqlx::query!(
        r#"
UPDATE users
SET password_hash = $1
WHERE user_id = $2
"#,
        password_hash.expose_secret(),
        user_id
    ).execute(pool)
        .await
        .context("Failed to change user's password in the database")?;

    Ok(())
}

