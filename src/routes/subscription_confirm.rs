use actix_web::{web, HttpResponse, ResponseError};
use actix_web::http::StatusCode;
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;
use crate::routes::ConfirmError::NotFoundSubscriber;
use crate::routes::error_chain_fmt;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[derive(thiserror::Error)]
pub enum ConfirmError {
    #[error("There is no subscriber associated with the provided token.")]
    NotFoundSubscriber,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for ConfirmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
impl ResponseError for ConfirmError {
    fn status_code(&self) -> StatusCode {
        match self {
            ConfirmError::NotFoundSubscriber => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[allow(clippy::async_yields_async)]
#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(parameters: web::Query<Parameters>, pool: web::Data<PgPool>) -> Result<HttpResponse, ConfirmError> {
    // let id = match get_subscriber_id_from_token(&pool, &parameters.subscription_token).await {
    //     Ok(id) => id,
    //     Err(_) => return HttpResponse::InternalServerError().finish(),
    // };
    // match id {
    //     // Non-existing token!
    //     None => HttpResponse::Unauthorized().finish(),
    //     Some(subscriber_id) => {
    //         if confirm_subscriber(&pool, subscriber_id).await.is_err() {
    //             return HttpResponse::InternalServerError().finish();
    //         }
    //         HttpResponse::Ok().finish()
    //     }
    // }
    let id = get_subscriber_id_from_token(&pool, &parameters.subscription_token)
        .await
        .context("Failed to retrieve the subscriber ID associated with the provided token.")?
        .ok_or(NotFoundSubscriber)?;
    confirm_subscriber(&pool, id)
        .await
        .context("Failed to update the subscriber's status to `confirmed`.")?;
    Ok(HttpResponse::Ok().finish())
    
}

#[allow(clippy::async_yields_async)]
#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, pool))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(subscription_token, pool))]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token,
    )
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
    Ok(result.map(|r| r.subscriber_id))
}


