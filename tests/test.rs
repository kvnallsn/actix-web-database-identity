//! Tests Module

extern crate actix_web;
extern crate actix_web_db_identity;
extern crate dotenv;

mod common;

use actix_web::http::StatusCode;
use actix_web::test::TestServer;

use common::SqlVariant;

/// Retrieves index page with no token supplied
///
/// Token: None
/// Expected Result: 200 OK
fn get_index(mut srv: TestServer) {
    common::index(&mut srv, None);
}

#[test]
fn sqlite_get_index() {
    let srv = common::build_test_server(SqlVariant::Sqlite);
    get_index(srv);
}

#[test]
fn mysql_get_index() {
    let srv = common::build_test_server(SqlVariant::MySql);
    get_index(srv);
}

#[test]
fn pg_get_index() {
    let srv = common::build_test_server(SqlVariant::Postgres);
    get_index(srv);
}

/// Retrieves profile page with no token supplied
///
/// Token: None
/// Expected Result: 401 Unauthorized
fn no_identity(mut srv: TestServer) {
    common::profile(&mut srv, None, StatusCode::UNAUTHORIZED);
}

#[test]
fn sqlite_no_identity() {
    let srv = common::build_test_server(SqlVariant::Sqlite);
    no_identity(srv);
}

#[test]
fn mysql_no_identity() {
    let srv = common::build_test_server(SqlVariant::MySql);
    no_identity(srv);
}

#[test]
fn pg_no_identity() {
    let srv = common::build_test_server(SqlVariant::Postgres);
    no_identity(srv);
}

/// Retrives the profile page with an invalid token supplied
///
/// Token: Invalid
/// Expected Result: 401 Unauthorized
fn invalid_token(mut srv: TestServer) {
    common::profile(&mut srv, Some("invalidtoken"), StatusCode::UNAUTHORIZED);
}

#[test]
fn sqlite_invalid_token() {
    let srv = common::build_test_server(SqlVariant::Sqlite);
    invalid_token(srv);
}

#[test]
fn mysql_invalid_token() {
    let srv = common::build_test_server(SqlVariant::MySql);
    invalid_token(srv);
}

#[test]
fn pg_invalid_token() {
    let srv = common::build_test_server(SqlVariant::Postgres);
    invalid_token(srv);
}

/// Retrievs the profile page with a valid token
///
/// Token: Valid
/// Expected Result: 200 OK
fn valid_token(mut srv: TestServer) {
    common::profile(&mut srv, Some("g8mlRUwF1AKx7/ZRvReQ+dRhGpoDAzIC"), StatusCode::OK);
}

// There are some problems with this test, most likely due to too many
// concurrent reads/writes in a short period of time with SQLite, ignore
// unless specifically requested
#[test]
#[ignore]
fn sqlite_valid_token() {
    let srv = common::build_test_server(SqlVariant::Sqlite);
    valid_token(srv);
}

#[test]
fn mysql_valid_token() {
    let srv = common::build_test_server(SqlVariant::MySql);
    valid_token(srv);
}

#[test]
fn pg_valid_token() {
    let srv = common::build_test_server(SqlVariant::Postgres);
    valid_token(srv);
}

/// Tests all endpoints with all conditions
fn login_logout(mut srv: TestServer) {
    // Make sure we can get the index (pass ok)
    println!("######### INDEX #########");
    common::index(&mut srv, None);

    // Try the protected route (no token, fail unauthorized)
    println!("######### PROFILE #1 #########");
    common::profile(&mut srv, None, StatusCode::UNAUTHORIZED);

    // Login in (assumes valid credentials)
    println!("######### LOGIN #########");
    let token = match common::login(&mut srv, "mike") {
        Some(t) => t,
        None => panic!("Token not found! Login Failed"),
    };

    // Try the protected route again (no auth token, fail unauthorized)
    println!("######### PROFILE #2 #########");
    common::profile(&mut srv, None, StatusCode::UNAUTHORIZED);

    // Try the protected route again (with token, pass ok)
    println!("######### PROFILE #3 #########");
    common::profile(&mut srv, Some(&token), StatusCode::OK);

    // Log out (no token, expect fail bad request)
    println!("######### LOGOUT #1 #########");
    common::logout(&mut srv, None, StatusCode::BAD_REQUEST);

    // Log out (with token, expect pass ok)
    println!("######### LOGOUT #2 #########");
    common::logout(&mut srv, Some(&token), StatusCode::OK);

    // Try the protected route again (after logout, fail unauthorized)
    println!("######### PROFILE #4 #########");
    common::profile(&mut srv, Some(&token), StatusCode::UNAUTHORIZED);
}

#[test]
fn sqlite_login_logout() {
    let srv = common::build_test_server(SqlVariant::Sqlite);
    login_logout(srv);
}

#[test]
fn mysql_login_logout() {
    let srv = common::build_test_server(SqlVariant::MySql);
    login_logout(srv);
}

#[test]
fn pg_login_logout() {
    let srv = common::build_test_server(SqlVariant::Postgres);
    login_logout(srv);
}

/// Test logging out of a client when a user has multiple clients
/// connected (aka multiple entries for userid in table)
fn multiple_logout(mut srv: TestServer) {
    // Log in twice
    let token_1 = common::login(&mut srv, "george");
    let token_2 = common::login(&mut srv, "george");

    let token_1 = match token_1 {
        Some(t) => t,
        None => panic!("Token 1 not found!"),
    };

    let token_2 = match token_2 {
        Some(t) => t,
        None => panic!("Token 1 not found!"),
    };

    // Make sure both tokens are valid by grabbing the profile
    common::profile(&mut srv, Some(&token_1), StatusCode::OK);
    common::profile(&mut srv, Some(&token_2), StatusCode::OK);

    // Now log out token_2, but keep 1 logged in
    common::logout(&mut srv, Some(&token_2), StatusCode::OK);

    // Try to fetch the profiles again, token 2 should failed
    common::profile(&mut srv, Some(&token_1), StatusCode::OK);
    common::profile(&mut srv, Some(&token_2), StatusCode::UNAUTHORIZED);

    // Log out token 1
    common::logout(&mut srv, Some(&token_1), StatusCode::OK);

    // Try to fetch the profiles again, both should failed
    common::profile(&mut srv, Some(&token_1), StatusCode::UNAUTHORIZED);
    common::profile(&mut srv, Some(&token_2), StatusCode::UNAUTHORIZED);
}

#[test]
#[ignore]
fn sqlite_multiple_logout() {
    let srv = common::build_test_server(SqlVariant::Sqlite);
    multiple_logout(srv);
}

#[test]
fn mysql_multiple_logout() {
    let srv = common::build_test_server(SqlVariant::MySql);
    multiple_logout(srv);
}

#[test]
fn pg_multiple_logout() {
    let srv = common::build_test_server(SqlVariant::Postgres);
    multiple_logout(srv);
}
