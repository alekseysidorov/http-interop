use http::{header::USER_AGENT, HeaderValue, Method};
use reqwest::Client;
use tower::ServiceBuilder;
use tower_http::ServiceBuilderExt;
use tower_http_client::ServiceExt;
use tower_reqwest::HttpClientLayer;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

mod utils;

// Check that client request builder uses proper methods.
#[test]
fn test_service_ext_request_builder_methods() -> anyhow::Result<()> {
    let mut fake_client = ServiceBuilder::new()
        .layer(HttpClientLayer)
        .service(Client::new());

    assert_eq!(
        fake_client.get("http://localhost").build()?.method(),
        Method::GET
    );
    assert_eq!(
        fake_client.post("http://localhost").build()?.method(),
        Method::POST
    );
    assert_eq!(
        fake_client.put("http://localhost").build()?.method(),
        Method::PUT
    );
    assert_eq!(
        fake_client.patch("http://localhost").build()?.method(),
        Method::PATCH
    );
    assert_eq!(
        fake_client.delete("http://localhost").build()?.method(),
        Method::DELETE
    );
    assert_eq!(
        fake_client.head("http://localhost").build()?.method(),
        Method::HEAD
    );

    Ok(())
}

// Check that we can use tower-http layers on top of the compatibility wrapper.
#[tokio::test]
async fn test_service_ext_execute() -> anyhow::Result<()> {
    let (mock_server, mock_uri) = utils::start_mock_server().await;
    // Arrange the behaviour of the MockServer adding a Mock:
    // when it receives a GET request on '/hello' it will respond with a 200.
    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(ResponseTemplate::new(200))
        // Mounting the mock on the mock server - it's now effective!
        .mount(&mock_server)
        .await;

    let client = ServiceBuilder::new()
        .override_response_header(USER_AGENT, HeaderValue::from_static("tower-reqwest"))
        .layer(HttpClientLayer)
        .service(Client::new());

    let response = client
        .clone()
        .execute(
            http::request::Builder::new()
                .method(http::Method::GET)
                .uri(format!("{mock_uri}/hello"))
                .body("")?,
        )
        .await?;

    assert!(response.status().is_success());
    assert_eq!(
        response.headers().get(USER_AGENT).unwrap(),
        HeaderValue::from_static("tower-reqwest")
    );

    Ok(())
}

// Check that the `get` method is useful.
#[tokio::test]
async fn test_service_ext_get() -> anyhow::Result<()> {
    let (mock_server, mock_uri) = utils::start_mock_server().await;

    Mock::given(method("GET"))
        .and(path("/hello"))
        .respond_with(ResponseTemplate::new(200))
        // Mounting the mock on the mock server - it's now effective!
        .mount(&mock_server)
        .await;

    let client = ServiceBuilder::new()
        .layer(HttpClientLayer)
        .service(Client::new());

    let response = client
        .clone()
        .get(format!("{mock_uri}/hello"))
        .send()?
        .await?;
    assert!(response.status().is_success());

    Ok(())
}

#[cfg(feature = "json")]
#[tokio::test]
async fn test_service_ext_put_json() -> anyhow::Result<()> {
    use http::header::CONTENT_TYPE;
    use tower_http_client::ResponseExt;
    use wiremock::Request;

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct Data {
        id: String,
        next: String,
    }

    let (mock_server, mock_uri) = utils::start_mock_server().await;

    let data = Data {
        id: "req-1".to_owned(),
        next: "resp-1".to_owned(),
    };

    Mock::given(method("PUT"))
        .and(path("/hello"))
        .respond_with(|req: &Request| {
            let value: Data = req.body_json().unwrap();
            assert_eq!(value.id, "req-1");
            assert_eq!(req.headers.get(CONTENT_TYPE).unwrap(), "application/json");

            ResponseTemplate::new(200).set_body_json(Data {
                id: value.next,
                next: "wiremock-1".to_owned(),
            })
        })
        // Mounting the mock on the mock server - it's now effective!
        .mount(&mock_server)
        .await;

    let client = ServiceBuilder::new()
        .layer(HttpClientLayer)
        .service(Client::new());

    let response = client
        .clone()
        .put(format!("{mock_uri}/hello"))
        .json(&data)?
        .send()?
        .await?;
    let value: Data = response.body_reader().json().await?;
    assert_eq!(value.id, "resp-1");
    assert_eq!(value.next, "wiremock-1");

    Ok(())
}
