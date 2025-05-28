use crate::authentication::{validate_credentials, AuthError, Credentials};
use crate::routes::admin::dashboard::get_username;
use crate::session_state::TypedSession;
use crate::utils::{e500, see_other};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize, Debug)]
pub struct FormData {
    current_password: SecretString,
    new_password: SecretString,
    new_password_check: SecretString,
}
pub async fn change_password(
    form: web::Form<FormData>,
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    tracing::info!("{:?}",form);
    let user_id = if let Some(uuid) = session.get_user_id().map_err(e500)? {
        uuid
    } else {
        return Ok(see_other("/login"));
    };

    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match."
        ).send();
        return Ok(see_other("/admin/password"));
    }

    let username = get_username(&user_id, &pool).await.map_err(e500)?;

    let credentials = Credentials {
        username,
        password: form.0.current_password,
    };

    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                Ok(see_other("/admin/password"))
            }
            AuthError::InternalError(_) => { Err(e500(e).into()) }
        };
    }
    crate::authentication::change_password(user_id, form.0.new_password, &pool)
        .await
        .map_err(e500)?;
    FlashMessage::error("Your password has been changed.").send();
    Ok(see_other("/admin/password"))
}