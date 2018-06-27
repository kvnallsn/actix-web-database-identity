//! Common Test Functions
//!
//! Module: Tests/common

use actix_web_db_identity::SqlIdentityPolicy;

use actix_web::client::ClientRequest;
use actix_web::http::StatusCode;
use actix_web::middleware::identity::{IdentityService, RequestIdentity};
use actix_web::test::TestServer;
use actix_web::{HttpRequest, HttpResponse};

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
