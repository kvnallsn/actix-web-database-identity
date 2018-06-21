//! Tests Module

use super::SqlIdentityPolicy;

use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use actix_web::http::StatusCode;
use actix_web::test;
use actix_web::middleware::identity::{IdentityService, RequestIdentity};

const TEST_SQLITE_DATABASE: &'static str = "tests/db.sqlite3";

fn build_test_server() -> test::TestServer {
    test::TestServer::new(move |app| {
        app.middleware(IdentityService::new(
                SqlIdentityPolicy::sqlite(TEST_SQLITE_DATABASE).unwrap()))

        .resource("/", |r| r.get().h(|_| {
            HttpResponse::Ok()
        }))

        .resource("/login", |r| r.post().h(|mut req: HttpRequest| {
            req.remember("mike".to_string());
            HttpResponse::Ok()
        }))

        .resource("/profile", |r| r.get().h(|req: HttpRequest| {
            match req.identity() {
                Some(_) => HttpResponse::Ok(),
                None => HttpResponse::Unauthorized(),
            }
        }))

        .resource("/logout", |r| r.post().h(|mut req: HttpRequest| {
            req.forget();
            HttpResponse::Ok()
        }));
    })
}

#[test]
#[cfg(unix)]
fn test_get_index() {
    let mut srv = build_test_server();

    let request = srv.get().finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status().is_success());
}

#[test]
#[cfg(unix)]
fn test_no_identity() {
    let mut srv = build_test_server();

    let request = srv.get().uri(srv.url("/profile")).finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status() == StatusCode::UNAUTHORIZED);
}

#[test]
#[cfg(unix)]
fn test_invalid_token() {
    let mut srv = build_test_server();

    let request = srv.get()
        .uri(srv.url("/profile"))
        .header("Authorization", "Bearer invalidtoken")
        .finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status() == StatusCode::UNAUTHORIZED);
}

#[test]
#[cfg(unix)]
fn test_valid_token() {
    let mut srv = build_test_server();

    let request = srv.get()
        .uri(srv.url("/profile"))
        .header("Authorization", "Bearer g8mlRUwF1AKx7/ZRvReQ+dRhGpoDAzIC")
        .finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status() == StatusCode::OK);
}

#[test]
#[cfg(unix)]
fn test_login_logout() {
    let mut srv = build_test_server();

    // Make sure we can get the index (pass ok)
    let request = srv.get().finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status() == StatusCode::OK);

    // Try the protected route (no token, fail unauthorized)
    let request = srv.get().uri(srv.url("/profile")).finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status() == StatusCode::UNAUTHORIZED);

    // Login in (assumes valid credentials)
    let request = srv.post().uri(srv.url("/login")).finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status() == StatusCode::OK);

    // Extract our login token
    let token = response.headers().get("twinscroll-auth");
    assert!(token.is_some());
    let token = token.unwrap();

    // Try the protected route again (no auth token, fail unauthorized)
    let request = srv.get().uri(srv.url("/profile")).finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status() == StatusCode::UNAUTHORIZED);

    // Try the protected route again (with token, pass ok)
    let request = srv.get().uri(srv.url("/profile"))
        .header("Authorization", format!("Bearer {}", token.to_str().unwrap()))
        .finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status() == StatusCode::OK);

    // Log out (no token, expect fail unauthorized)
    let request = srv.post().uri(srv.url("/logout")).finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status() == StatusCode::UNAUTHORIZED);

    // Log out (with token, expect pass ok)
    let request = srv.post().uri(srv.url("/logout"))
        .header("Authorization", format!("Bearer {}", token.to_str().unwrap()))
        .finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status() == StatusCode::OK);

    // Try the protected route again (after logout, fail unauthorized)
    let request = srv.get().uri(srv.url("/profile"))
        .header("Authorization", format!("Bearer {}", token.to_str().unwrap()))
        .finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status() == StatusCode::UNAUTHORIZED);
}
