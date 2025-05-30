use crate::helpers::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};
use fake::faker::internet::en::SafeEmail;
use fake::faker::name::zh_tw::Name;
use fake::Fake;
use std::time::Duration;
use uuid::Uuid;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, MockBuilder, ResponseTemplate};

#[tokio::test]
async fn invalid_password_is_rejected() {
    let app = spawn_app().await;
    let username = app.test_user.username;
    let password = Uuid::new_v4().to_string();
    let res = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .basic_auth(username, Some(password))
        .json(
            &serde_json::json!({
                "title": "Newsletter title",
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>",
                }
            })
        )
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(res.status().as_u16(), 401);
    assert_eq!(
        r#"Basic realm="publish""#,
        res.headers()["WWW-Authenticate"]
    );
}
#[tokio::test]
async fn non_existing_user_is_rejected() {
    let app = spawn_app().await;
    let username = Uuid::new_v4().to_string();
    let password = Uuid::new_v4().to_string();
    let res = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .basic_auth(username, Some(password))
        .json(
            &serde_json::json!({
                "title": "Newsletter title",
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>",
                }
            })
        )
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(res.status().as_u16(), 401);
    assert_eq!(
        r#"Basic realm="publish""#,
        res.headers()["WWW-Authenticate"]
    );
}

#[tokio::test]
async fn requests_missing_authorization_are_rejected() {
    let app = spawn_app().await;

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .json(
            &serde_json::json!({
                "title": "Newsletter title",
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>",
                }
            })
        )
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 401);
    // assert_eq!(
    //     response.text().await.unwrap(),
    //     "Unauthorized: Missing or invalid API token"
    // );
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;
    app.post_test_user_login().await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Act
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string()

    });
    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    // Assert
    // assert_eq!(response.status().as_u16(), 200);
    assert_is_redirect_to(&response, "/admin/newsletters");
    // Mock verifies on Drop that we haven't sent the newsletter email
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.post_test_user_login().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    });
    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    // Assert
    // assert_eq!(response.status().as_u16(), 200);
    assert_is_redirect_to(&response, "/admin/newsletters");
    // Mock verifies on Drop that we have sent the newsletter email

    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains(
        "<p><i>The newsletter issue has been accepted - \
        emails will go out shortly.</i></p>"
    ));

    app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    // Arrange
    let app = spawn_app().await;
    app.post_test_user_login().await;

    let test_cases = vec![
        (
            serde_json::json!({
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
            }),
            "missing title",
        ),
        (
            serde_json::json!({
              "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
            }),
            "missing content",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_publish_newsletter(&invalid_body).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            // Additional customised error message on test failure
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}


async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = serde_urlencoded::to_string(&serde_json::json!({
        "name": name,
        "email": email
    }))
        .unwrap();


    // let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;
    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();
    
    app.get_confirmation_links().await
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let link = create_unconfirmed_subscriber(app).await.html;

    
    tracing::error!("linklink:{link}");
    reqwest::get(link)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}


#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.post_test_user_login().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    });

    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    assert_is_redirect_to(&response, "/admin/newsletters");

    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been accepted - \
        emails will go out shortly.</i></p>"));

    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html_page = app.get_publish_newsletter_html().await;

    tracing::info!("html_page:{html_page}");
    assert!(html_page.contains("<p><i>The newsletter issue has been accepted - \
        emails will go out shortly.</i></p>"));

    app.dispatch_all_pending_emails().await;
}


#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.post_test_user_login().await;


    Mock::given(path("/email"))
        .and(method("POST"))
        // Setting a long delay to ensure that the second request
        // arrives before the first one completes
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act - Submit two newsletter forms concurrently
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    });
    let response1 = app.post_publish_newsletter(&newsletter_request_body);
    let response2 = app.post_publish_newsletter(&newsletter_request_body);
    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );
    app.dispatch_all_pending_emails().await;
    // Mock verifies on Drop that we have sent the newsletter email **once**
}

fn when_sending_an_email() -> MockBuilder {
    Mock::given(path("/email")).and(method("POST"))
}

// #[tokio::test]
// async fn transient_errors_do_not_cause_duplicate_deliveries_on_retries() {
//     let app = spawn_app().await;
//     let newsletter_request_body = serde_json::json!({
//          "title": "Newsletter title",
//         "text_content": "Newsletter body as plain text",
//         "html_content": "<p>Newsletter body as HTML</p>",
//         "idempotency_key": uuid::Uuid::new_v4().to_string()
//     });
//
//     create_confirmed_subscriber(&app).await;
//     create_confirmed_subscriber(&app).await;
//
//     app.post_test_user_login().await;
//
//     when_sending_an_email()
//         .respond_with(ResponseTemplate::new(200))
//         .up_to_n_times(1)
//         .expect(1)
//         .mount(&app.email_server)
//         .await;
//
//     when_sending_an_email()
//         .respond_with(ResponseTemplate::new(500))
//         .up_to_n_times(1)
//         .expect(1)
//         .mount(&app.email_server)
//         .await;
//
//     let response = app.post_publish_newsletter(&newsletter_request_body)
//         .await;
//
//     assert_eq!(response.status().as_u16(), 500);
//
//
//     when_sending_an_email()
//         .respond_with(ResponseTemplate::new(200))
//         .expect(1)
//         .named("Delivery retry")
//         .mount(&app.email_server)
//         .await;
//     let response = app.post_publish_newsletter(&newsletter_request_body).await;
//
//     assert_eq!(response.status().as_u16(), 303)
// }
