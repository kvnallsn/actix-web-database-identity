//! Tests Module

extern crate actix_web;
extern crate actix_web_db_identity;
extern crate dotenv;

use actix_web_db_identity::SqlIdentityPolicy;

use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use actix_web::client::ClientRequest;
use actix_web::http::StatusCode;
use actix_web::test;
use actix_web::middleware::identity::{IdentityService, RequestIdentity};

use std::thread;
use std::time::Duration;

enum Sql {
    Sqlite,
    MySql,
    Pg,
}

fn build_test_server(sql: Sql) -> test::TestServer {
    dotenv::from_filename("tests/test.env").ok();


    test::TestServer::new(move |app| {
        app.middleware(IdentityService::new(
                match sql {
                    Sql::Sqlite => SqlIdentityPolicy::sqlite(&dotenv::var("SQLITE_URL").unwrap()).unwrap(),
                    Sql::MySql => SqlIdentityPolicy::mysql(&dotenv::var("MYSQL_URL").unwrap()).unwrap(),
                    Sql::Pg => SqlIdentityPolicy::postgres(&dotenv::var("PG_URL").unwrap()).unwrap(),
                }
            )
        )

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

fn check_response(srv: &mut test::TestServer, req: ClientRequest, exp: StatusCode) -> bool {
    let resp = srv.execute(req.send()).unwrap();

    resp.status() == exp
}

/// Retrieves index page with no token supplied
///
/// Token: None
/// Expected Result: 200 OK
fn get_index(mut srv: test::TestServer) {
    let request = srv.get().finish().unwrap();
    assert!(check_response(&mut srv, request, StatusCode::OK));
}

#[test]
fn sqlite_get_index() {
    let srv = build_test_server(Sql::Sqlite);
    get_index(srv);
}

#[test]
fn mysql_get_index() {
    let srv = build_test_server(Sql::MySql);
    get_index(srv);
}

#[test]
fn pg_get_index() {
    let srv = build_test_server(Sql::Pg);
    get_index(srv);
}

/// Retrieves profile page with no token supplied
///
/// Token: None
/// Expected Result: 401 Unauthorized
fn no_identity(mut srv: test::TestServer) {
    let request = srv.get().uri(srv.url("/profile")).finish().unwrap();
    assert!(check_response(&mut srv, request, StatusCode::UNAUTHORIZED));
}

#[test]
fn sqlite_no_identity() {
    let srv = build_test_server(Sql::Sqlite);
    no_identity(srv);
}

#[test]
fn mysql_no_identity() {
    let srv = build_test_server(Sql::MySql);
    no_identity(srv);
}

#[test]
fn pg_no_identity() {
    let srv = build_test_server(Sql::Pg);
    no_identity(srv);
}

/// Retrives the profile page with an invalid token supplied
///
/// Token: Invalid
/// Expected Result: 401 Unauthorized
fn invalid_token(mut srv: test::TestServer) {
    let request = srv.get()
        .uri(srv.url("/profile"))
        .header("Authorization", "Bearer invalidtoken")
        .finish().unwrap();
    assert!(check_response(&mut srv, request, StatusCode::UNAUTHORIZED));
}

#[test]
fn sqlite_invalid_token() {
    let srv = build_test_server(Sql::Sqlite);
    invalid_token(srv);
}

#[test]
fn mysql_invalid_token() {
    let srv = build_test_server(Sql::MySql);
    invalid_token(srv);
}

#[test]
fn pg_invalid_token() {
    let srv = build_test_server(Sql::Pg);
    invalid_token(srv);
}

/// Retrievs the profile page with a valid token
///
/// Token: Valid
/// Expected Result: 200 OK
fn valid_token(mut srv: test::TestServer) {
    let request = srv.get()
        .uri(srv.url("/profile"))
        .header("Authorization", "Bearer g8mlRUwF1AKx7/ZRvReQ+dRhGpoDAzIC")
        .finish().unwrap();
    assert!(check_response(&mut srv, request, StatusCode::OK));
}

#[test]
fn sqlite_valid_token() {
    let srv = build_test_server(Sql::Sqlite);
    valid_token(srv);
}

#[test]
fn mysql_valid_token() {
    let srv = build_test_server(Sql::MySql);
    valid_token(srv);
}

#[test]
fn pg_valid_token() {
    let srv = build_test_server(Sql::Pg);
    valid_token(srv);
}

/// Tests all endpoints with all conditions
fn login_logout(mut srv: test::TestServer) {
    // Make sure we can get the index (pass ok)
    let request = srv.get().finish().unwrap();
    assert!(check_response(&mut srv, request, StatusCode::OK));

    // Try the protected route (no token, fail unauthorized)
    let request = srv.get().uri(srv.url("/profile")).finish().unwrap();
    assert!(check_response(&mut srv, request, StatusCode::UNAUTHORIZED));

    // Login in (assumes valid credentials)
    let request = srv.post().uri(srv.url("/login")).finish().unwrap();
    let response = srv.execute(request.send()).unwrap();
    assert!(response.status() == StatusCode::OK);

    // Extract our login token
    let token = response.headers().get("twinscroll-auth");
    assert!(token.is_some());
    let token = token.unwrap().to_str().unwrap();

    // Try the protected route again (no auth token, fail unauthorized)
    let request = srv.get().uri(srv.url("/profile")).finish().unwrap();
    assert!(check_response(&mut srv, request, StatusCode::UNAUTHORIZED));

    // Try the protected route again (with token, pass ok)
    let request = srv.get().uri(srv.url("/profile"))
        .header("Authorization", format!("Bearer {}", token))
        .finish().unwrap();
    assert!(check_response(&mut srv, request, StatusCode::OK));

    // Log out (no token, expect fail unauthorized)
    let request = srv.post().uri(srv.url("/logout")).finish().unwrap();
    assert!(check_response(&mut srv, request, StatusCode::UNAUTHORIZED));

    // Log out (with token, expect pass ok)
    let request = srv.post().uri(srv.url("/logout"))
        .header("Authorization", format!("Bearer {}", token))
        .finish().unwrap();
    assert!(check_response(&mut srv, request, StatusCode::OK));

    // Try the protected route again (after logout, fail unauthorized)
    let request = srv.get().uri(srv.url("/profile"))
        .header("Authorization", format!("Bearer {}", token))
        .finish().unwrap();
    assert!(check_response(&mut srv, request, StatusCode::UNAUTHORIZED));
}

#[test]
fn sqlite_login_logout() {
    thread::sleep(Duration::from_millis(100));
    let srv = build_test_server(Sql::Sqlite);
    login_logout(srv);    
}

#[test]
fn mysql_login_logout() {
    let srv = build_test_server(Sql::MySql);
    login_logout(srv);    
}

#[test]
fn pg_login_logout() {
    let srv = build_test_server(Sql::Pg);
    login_logout(srv);    
}
