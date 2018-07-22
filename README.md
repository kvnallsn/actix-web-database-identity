# Actix Database Identity Provider

[![Build Status](https://travis-ci.org/kvnallsn/actix-web-database-identity.svg?branch=master)](https://travis-ci.org/kvnallsn/actix-web-database-identity)
[![docs.rs](https://docs.rs/actix-web-sql-identity/badge.svg)](https://docs.rs/crate/actix-web-sql-identity/)

SQL database (diesel) integration for actix framework

## Description

Implements a SQL backend for Actix-Web's identity middleware.  Does not perform any authentication, only "remembers" the user when told to.  The returned header containing the authorized token is configurable via the `SqlIdentityBuilder::response_header()` method.

Normal server application flow:

* Application authenticates a user
* Application calls `remember()` with a user-identifiable string
* SQL identity middleware generates a new token and embeds it in the response header
* Application returns response (with header)

From here, normal flow according to identity middleware guide can be followed

Normal client application flow:

* Client POSTs to a login endpoint, is authorized by server
* Client receives response, extracts the authorization token in the specified header
* On future requests (including logout), the client builds a bearer authentication header with the returned token

### SQL Variants supported

* SQLite 
* MySQL
* Postgres

### Features

_Default_: SQLite, MySQL, Postgres

_sqlite_: Include SQLite support

_mysql_: Include MySQL support

_postgres_: Include PostgreSQL supprt

## Database Requirements

This crate requires a table named *identities* with the following fields:

| Field     | Type      | Constraints                   | Description                                                 |
| --------- | --------- | ----------------------------- | ----------------------------------------------------------- |
| id        | BIGINT    | PRIMARY KEY, AUTO INCREMENT   | A unique id that is not the token, for revoking sessions    |
| token     | CHAR(32)  | NOT NULL, UNIQUE              | The auto-generated token field, will be used to lookup user |
| userid    | TEXT      | NOT NULL                      | The user id to remember, probably a key in another table    |
| ip        | TEXT      |                               | The IP the user most recently connected from                |
| useragent | TEXT      |                               | The user-agent of the most recent connection                |
| created   | TIMESTAMP | NOT NULL                      | Timestamp (w/out timezone) this token was created           |
| modified  | TIMESTAMP | NOT NULL                      | Timestamp (w/out timezone) this token was last used         |

Example SQL files for SQLite, MySQL, and PostgreSQL are available int the sql/ folder on the repository

## Server Example

```rust
extern crate actix_web;
extern crate actix_web_sql_identity;

use actix_web::{http, server, App, HttpRequest, Responder};
use actix_web::middleware::identity::{IdentityService, RequestIdentity};
use actix_web_sql_identity::SqlIdentityBuilder;

const POOL_SIZE: usize = 3;     // Number of connections per pool

fn login(mut req: HttpRequest) -> impl Responder {
    // Should pull username/id from request
    req.remember("username_or_id".to_string());
    "Logged in!".to_string()
}

fn profile(req: HttpRequest) -> impl Responder {
    if let Some(user) = req.identity() {
        format!("Hello, {}!", user)
    } else {
        "Hello, anonymous user!".to_string()
    }
}

fn logout(mut req: HttpRequest) -> impl Responder {
    req.forget();
    "Logged out!".to_string()
}

fn main() {
    server::new(|| {
		// Construct our policy, passing the address and any options
        let policy = SqlIdentityBuilder::new("sqlite://my.db")
            .pool_size(POOL_SIZE);

        App::new()
            .route("/login", http::Method::POST, login)
            .route("/profile", http::Method::GET, profile)
            .route("/logout", http::Method::POST, logout)
            .middleware(IdentityService::new(
                    policy.finish()
                        .expect("failed to connect to database")))
    })
    .bind("127.0.0.1:7070").unwrap()
    .run();
}
```

## Client Example

```rust
extern crate reqwest;

#[macro_use]
extern crate hyper;

use hyper::header::{Authorization, Bearer, Headers};
use reqwest::{Client, Response};

// Build our custom header that will contain our returned token
header! { (XActixAuth, "X-Actix-Auth") => [String] }

/// Builds a GET request to send to the server, with optional authentication
///
/// # Arguments
///
/// * `client` - Client to build request with
/// * `uri` - Endpoint to target (e.g., /index, /profile, etc)
/// * `token` - An optional authentication token
fn build_get(client: &Client, uri: &str, token: Option<&str>) -> Response {
    let mut req = client
        .get(format!("http://127.0.0.1:7070{}", uri).as_str());

    if let Some(token) = token {
        let mut headers = Headers::new();
        headers.set(Authorization(Bearer {
            token: token.to_owned(),
        }));
        req.headers(headers);
    }

    let req = req.build()
        .expect("failed to build request");

    client.execute(req).expect("failed to send request")
}

/// Builds a POST request to send to the server, with optional authentication
///
/// # Arguments
///
/// * `client` - Client to build request with
/// * `uri` - Endpoint to target (e.g., /login, /logout)
/// * `token` - An optional authentication token
fn build_post(client: &Client, uri: &str, token: Option<&str>) -> Response {
    let mut req = client
        .post(format!("http://127.0.0.1:7070{}", uri).as_str());

    if let Some(token) = token {
        let mut headers = Headers::new();
        headers.set(Authorization(Bearer {
            token: token.to_owned(),
        }));
        req.headers(headers);
    }

    let req = req.build()
        .expect("failed to build request");

    client.execute(req).expect("failed to send request")
}

/// Pretty print a response
///
/// # Arguments
///
/// * `resp` - Response to print
fn print_response(resp: &mut Response) {
    let uri = resp.url().clone();
    println!("[{0: <15}] {1: <30} {2}", resp.status(), uri.as_str(), resp.text().expect("failed to read response"));
}

fn main() {
    let client = Client::new();

    // Get Index
    let mut resp = build_get(&client, "/", None); 
    print_response(&mut resp);

    // Get Profile (no auth)
    let mut resp = build_get(&client, "/profile", None);
    print_response(&mut resp);

    // Login
    let mut resp = build_post(&client, "/login", None);
    print_response(&mut resp);

    // Extract the auth token from the header
    // (Header field can be changed on server, default is used here)
    let hdrs = resp.headers();
    let token = hdrs.get::<XActixAuth>().unwrap();
    //println!("[token]: {:?}", token.0);

    // Get Profile (auth)
    let mut resp = build_get(&client, "/profile", Some(token.0.as_ref()));
    print_response(&mut resp);

    // Logout
    let mut resp = build_post(&client, "/logout", Some(token.0.as_str()));
    print_response(&mut resp);

    // Get Profile (auth)
    let mut resp = build_get(&client, "/profile", Some(token.0.as_ref()));
    print_response(&mut resp);
}
```

## License

BSD 3-Clause

## Author

Kevin Allison
kvnallsn AT gmail.com
