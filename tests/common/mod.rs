//! Common Test Functions
//!
//! Module: Tests/common

use actix_web_sql_identity::SqlIdentityBuilder;

use actix_web::client::{ClientRequest, ClientRequestBuilder};
use actix_web::http::StatusCode;
use actix_web::middleware::identity::{IdentityService, RequestIdentity};
use actix_web::test::TestServer;
use actix_web::{HttpMessage, HttpRequest, HttpResponse};

use dotenv;

const RESPONSE_HEADER: &'static str = "test-auth";

/// The different kinds of SQL languanges supported
pub enum SqlVariant {
    Sqlite,
    MySql,
    Postgres,
}

/// Builds a new test server using a specific SQL variant and
/// reads the connection string from an environment variable.
/// Returns a new TestServer instance
///
/// # Arguments
///
/// * `sql` - The SQL variant to use (Sqlite, MySQL, or PostgreSQL)
pub fn build_test_server_from_env(variant: SqlVariant) -> TestServer {
    dotenv::from_filename("tests/test.env").ok();

    let uri = match variant {
        SqlVariant::Sqlite => {
            format!("{}/{}",
                    dotenv::var("SQLITE_PATH").unwrap(),
                    dotenv::var("SQLITE_DB").unwrap(),
            )
        },
        SqlVariant::MySql => {
            format!("mysql://{}:{}@{}/{}",
                    dotenv::var("MYSQL_USER").unwrap(),
                    dotenv::var("MYSQL_PASS").unwrap(),
                    dotenv::var("MYSQL_HOST").unwrap(),
                    dotenv::var("MYSQL_DB").unwrap()
            )
        },
        SqlVariant::Postgres => {
            format!("postgres://{}:{}@{}/{}",
                    dotenv::var("PG_USER").unwrap(),
                    dotenv::var("PG_PASS").unwrap(),
                    dotenv::var("PG_HOST").unwrap(),
                    dotenv::var("PG_DB").unwrap()
            )
        }
    };

    build_test_server(uri)
}

/// Builds a new test server using a specific SQL variant and
/// reads the connection string from an environment variable.
/// Returns a new TestServer instance
///
/// # Arguments
///
/// * `uri` - Database connection string (e.g., sqlite://, mysql://, postgres://)
pub fn build_test_server<S: Into<String>>(uri: S) -> TestServer {
    let uri = uri.into();
    println!("Connecting to: {}", uri);


    TestServer::new(move |app| {
        // Build SQL Identity policy
        let policy = SqlIdentityBuilder::new(uri.clone())
            .response_header(RESPONSE_HEADER);

        app.middleware(IdentityService::new(
                policy.finish()
                    .expect("failed to connect to database")
        )).resource("/", |r| r.get().h(|_| HttpResponse::Ok()))
            .resource("/login", |r| {
                r.post().h(|mut req: HttpRequest| {
                    req.remember("mike".to_string());
                    HttpResponse::Ok()
                })
            })
            .resource("/profile", |r| {
                r.get().h(|req: HttpRequest| match req.identity() {
                    Some(_) => HttpResponse::Ok(),
                    None => HttpResponse::Unauthorized(),
                })
            })
            .resource("/logout", |r| {
                r.post().h(|mut req: HttpRequest| {
                    req.forget();
                    HttpResponse::Ok()
                })
            });
    })
}

/// Adds an authorization bearer token to a request
///
/// # Arguments
///
/// * `req` - Request to modify
/// * `token` - Token to add to request authorization header
pub fn add_token_to_request(req: &mut ClientRequestBuilder, token: &str) {
    req.header("Authorization", format!("Bearer {}", token));
}

/// Sends a response to the server and checks if the status code matches
/// the expected status code.  Returns true if statuses match, or false
/// otherwise
///
/// # Arugments
///
/// * `srv` - An instance of a TestServer
/// * `req` - The client request to send
/// * `exp` - The expected HTTP status code
pub fn check_response(srv: &mut TestServer, req: ClientRequest, exp: StatusCode) -> bool {
    println!("{:?}", req);
    let resp = srv.execute(req.send()).unwrap();
    println!("{:?}", resp);

    resp.status() == exp
}

/// Attempts to log a user in with the given username
/// Note: The server automatically assumes authentication passes
///
/// # Arguments
///
/// * `srv` - An instance of a TestServer
/// * `username` - Username to login
pub fn login(srv: &mut TestServer, _username: &str) -> Option<String> {
    let request = srv.post()
        .uri(srv.url("/login"))
        .finish()
        .unwrap();

    println!("{:?}", request);
    let response = srv.execute(request.send()).unwrap();
    println!("{:?}", response);
    assert!(response.status() == StatusCode::OK, "Login Failed");

    match response.headers().get(RESPONSE_HEADER) {
        Some(token) => Some(token.to_str().unwrap().to_string()),
        None => None,
    }
}

/// Attempts to log the user out with the provided token
///
/// # Arguments
///
/// * `srv` - An instance of a TestServer
/// * `token` - The token corresponding to the user to log out, or None
/// * `code` - Status code to expect (200 Ok, 401 Unauthorized, etc...)
pub fn logout(srv: &mut TestServer, token: Option<&str>, code: StatusCode) {
    let mut request = srv.post();
    let mut request = request.uri(srv.url("/logout"));

    if token.is_some() {
        add_token_to_request(&mut request, token.unwrap());
    }

    let request = request.finish().unwrap();

    assert!(check_response(srv, request, code));
}

/// Builds a common get request, appending an auth token if provided.
/// The url for the route will hit the endpoint specified in uri
///
/// # Arguments
///
/// * `srv` - An instance of a TestServer
/// * `uri` - Endpoint to hit
/// * `token` - An optional authorization token
fn build_get(srv: &mut TestServer, uri: &str, token: Option<&str>) -> ClientRequest {
    let mut request = srv.get();
    let mut request = request.uri(srv.url(uri));

    match token {
        Some(t) => add_token_to_request(&mut request, t),
        None => (),
    }

    request.finish().unwrap()
}

/// Attempts to get the index page
/// Note: This should always succeed
///
/// # Arguments
///
/// * `srv` - An instance of a TestServer
/// * `token` - An optional authorization token
pub fn index(srv: &mut TestServer, token: Option<&str>) {
    let request = build_get(srv, "/", token);
    assert!(
        check_response(srv, request, StatusCode::OK),
        "Failed to GET /index!"
    );
}

/// Attempts to get the profile page
///
/// # Arguments
///
/// * `srv` - An instance of a TestServer
/// * `token` - An optional authorization token
/// * `code` - Status code to expect (200 Ok, 401 Unauthorized, etc...)
pub fn profile(srv: &mut TestServer, token: Option<&str>, code: StatusCode) {
    let request = build_get(srv, "/profile", token);
    assert!(check_response(srv, request, code));
}
