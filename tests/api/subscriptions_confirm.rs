use crate::helpers::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn confirmation_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn the_link_return_by_subscriber_returns_a_200_if_called() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;
    app.post_subscriptions(body.into()).await;

    let _get_link = |str: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(str)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };

    let confirmation_link = app.get_confirmation_links().await.html;


    let response = reqwest::get(confirmation_link).await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
}
#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;
    app.post_subscriptions(body.into()).await;
    let  confirmation_link = app.get_confirmation_links().await.html;
    let response = reqwest::get(confirmation_link).await.unwrap().error_for_status().unwrap();
    assert_eq!(response.status().as_u16(), 200);

    let saved = sqlx::query!(
        "SELECT email, name, status FROM subscriptions"
    ).fetch_one(&app.db_pool)
        .await.expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");
}

