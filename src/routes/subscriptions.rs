use crate::domain::NewSubscriber;
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;
use crate::telemetry::init_subscriber;
use actix_web::{web, HttpResponse, ResponseError};
use chrono::Utc;
use rand::Rng;
use reqwest::{Client, Error};
use serde::Deserialize;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use std::fmt::{Debug, Display, Formatter};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

#[tracing::instrument(name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
subscriber_email = %form.email,
subscriber_name = form.name
    )
)]
pub async fn subscribe(web::Form(form): web::Form<FormData>, pool: web::Data<PgPool>, email_client: web::Data<EmailClient>, base_url: web::Data<ApplicationBaseUrl>) -> Result<HttpResponse, actix_web::Error> {
    println!("form:{:?}, ", form);
    let subscriber_form = match form.try_into() {
        Ok(form) => form,
        Err(_) => return Ok(HttpResponse::BadRequest().finish()),
    };
    let token = generate_subscription_token();
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return Ok(HttpResponse::InternalServerError().finish()),
    };

    let subscriber_id = match insert_subscriber(&subscriber_form, &mut transaction).await {
        Ok(id) => id,
        Err(_) => return Ok(HttpResponse::InternalServerError().finish()),
    };

    store_token(&mut transaction, subscriber_id, &token).await?;
    if transaction.commit().await.is_err() {
        return Ok(HttpResponse::InternalServerError().finish());
    }
    if send_confirmation_email(&email_client, subscriber_form, &base_url.0, token).await.is_err()
    {
        return Ok(HttpResponse::InternalServerError().finish())
    }


    Ok(HttpResponse::Ok().finish())

}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, subscriber_form)
)]
async fn send_confirmation_email(email_client: &EmailClient, subscriber_form: NewSubscriber, base_url: &str, token: String) -> Result<(), Error> {
    let confirmation_link = format!("{}/subscriptions/confirm?subscription_token={}", base_url, token);
    email_client
        .send_email(
            subscriber_form.email,
            "Welcome!",
            &format!(
                "Welcome to our newsletter {} <br />\
                Click <a href=\"{}\">here</a> to confirm your subscription.",
                subscriber_form.name.as_ref(),
                confirmation_link),
            &format!(
                "Welcome to our newsletter {} \n Visit {} to confirm your subscription",
                subscriber_form.name.as_ref(),
                confirmation_link
            ),
        )
        .await
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(form, transaction)
)]
pub async fn insert_subscriber(form: &NewSubscriber, transaction: &mut Transaction<'_, Postgres>) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
INSERT INTO subscriptions (id, email, name, subscribed_at, status)
VALUES ($1, $2, $3, $4,'pending_confirmation')
"#,
        subscriber_id,
        form.email.as_ref(),
        form.name.as_ref(),
        Utc::now()
    )
        .execute(&mut **transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to excute query: {:?}", e);
        e
    })?;
    Ok(subscriber_id)
}

fn generate_subscription_token() -> String {
    let mut rng = rand::thread_rng();
    std::iter::repeat_with(|| rng.sample(rand::distributions::Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}


struct StoreTokenError(sqlx::Error);

impl Debug for StoreTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // write!(f, "{}\n Caused by:\n\t{}", self, self.0)
        // 这里因为 实现了 Error 的 source . 
        error_chain_fmt(self, f)
    }

}

impl Display for StoreTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
               trying to store a subscription token."
        )
    }
}
// 这里实现 source 就可以使用链式调用的方法 ,循环递归错误信息了.这是一种约定俗成的方法.我们
// 我们在代码开发阶段对 source 进行规范的定义, 在后文中就可以更好 的使用.
impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl ResponseError for StoreTokenError {}


#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    let query = sqlx::query!(
        r#"
    INSERT INTO subscription_tokens (subscription_token, subscriber_id)
    VALUES ($1, $2)
        "#,
        subscription_token,
        subscriber_id
    );
    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        StoreTokenError(e)
    })?;
    Ok(())
}

// 这个方法就是一直循环调用 source 方法, 直到 source 返回 None 为止 去格式化错误原因,层层递归.
fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

