use crate::domain::SubscriberEmail;
use reqwest::{Client, Url};
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use std::time::Duration;


pub struct EmailClient {
    http_client: Client,
    base_url: Url,
    sender: SubscriberEmail,
    authorization_token: SecretString,
}

impl EmailClient {
    pub fn new(base_url: String, sender: SubscriberEmail, authorization_token: SecretString, timeout: Duration) -> Self {
        let base_url = Url::parse(&base_url).expect("Invalid pares base_url to reqwest::Url");
        let http_client = Client::builder().timeout(timeout)
            .build()
            .unwrap();
        Self {
            http_client,
            base_url,
            sender,
            authorization_token,
        }
    }

    pub async fn send_email(&self, recipient: SubscriberEmail, subject: &str, html_content: &str, text_content: &str) -> Result<(), reqwest::Error> {
        // let url = self.base_url.join("email").expect("aa");
        let url = format!("{}/email", self.base_url);
        let request_body = SendEmailRequest::new(self.sender.as_ref(),
                                                 recipient.as_ref(),
                                                 subject,
                                                 html_content,
                                                 text_content);

        self
            .http_client
            .post(&url)
            .header("X-Postmark-Server-Token", self.authorization_token.expose_secret())
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    text_body: &'a str,
    html_body: &'a str,
}
impl<'a> SendEmailRequest<'a> {
    pub fn new(from: &'a str, to: &'a str, subject: &'a str, text_body: &'a str, html_body: &'a str) -> Self {
        Self {
            from,
            to,
            subject,
            text_body,
            html_body,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use claims::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::zh_cn::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::SecretString;
    use std::time::Duration;

    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};


    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_url: String) -> EmailClient {
        let fa = Faker.fake::<String>();    
        EmailClient::new(base_url, email(), SecretString::from(fa), Duration::from_millis(200))
    }
    struct SendEmailBodyMatcher;
    impl Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                dbg!(&body);
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }
    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());


        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _ = email_client
            .send_email(email(), &subject(), &content(), &content()).await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;


        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content()).await;

        assert_ok!(outcome)
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());


        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content()).await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        let response = ResponseTemplate::new(200).set_delay(Duration::from_secs(180));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content()).await;

        assert_err!(outcome);
    }
}