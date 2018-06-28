//! Common Test Functions
//!
//! Module: Tests/common

use actix_web_db_identity::SqlIdentityPolicy;

use actix_web::client::{ClientRequest, ClientRequestBuilder};
use actix_web::http::StatusCode;
use actix_web::middleware::identity::{IdentityService, RequestIdentity};
use actix_web::test::TestServer;
use actix_web::{HttpMessage, HttpRequest, HttpResponse};

use dotenv;

/// The different kinds of SQL languanges supported
pub enum SqlVariant {
    Sqlite,
    MySql,
    Postgres,
}

/// Builds a new test server using a specific SQL variant. Returns
/// a new TestServer
///
/// # Arguments
///
/// * `sql` - The SQL variant to use (Sqlite, MySQL, or PostgreSQL)
pub fn build_test_server(sql: SqlVariant) -> TestServer {
    dotenv::from_filename("tests/test.env").ok();

    TestServer::new(move |app| {
        app.middleware(IdentityService::new(match sql {
            SqlVariant::Sqlite => {
                SqlIdentityPolicy::sqlite(1, &dotenv::var("SQLITE_URL").unwrap()).unwrap()
            }
            SqlVariant::MySql => {
                SqlIdentityPolicy::mysql(1, &dotenv::var("MYSQL_URL").unwrap()).unwrap()
            }
            SqlVariant::Postgres => {
                SqlIdentityPolicy::postgres(1, &dotenv::var("PG_URL").unwrap()).unwrap()
            }
        })).resource("/", |r| r.get().h(|_| HttpResponse::Ok()))
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

    match response.headers().get("twinscroll-auth") {
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
