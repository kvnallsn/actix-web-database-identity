# Actix Database Identity Provider

[![Build Status](https://travis-ci.org/kvnallsn/actix-web-database-identity.svg?branch=master)](https://travis-ci.org/kvnallsn/actix-web-database-identity)

SQL database (diesel) integration for actix framework

## SQL Backend

Uses either SQLite, Postgresql, or MySQL as the backend for an identity provider

### Currently Implemented

* SQLite 
* MySQL
* Postgres

## Example

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
        let policy = SqlIdentityBuilder::new("my.db")
            .pool_size(POOL_SIZE);

        App::new()
            .route("/login", http::Method::POST, login)
            .route("/profile", http::Method::GET, profile)
            .route("/logout", http::Method::POST, logout)
            .middleware(IdentityService::new(
                    policy.sqlite()
                        .expect("failed to connect to database")))
    })
    .bind("127.0.0.1:7070").unwrap()
    .run();
}
```

## License

BSD 3-Clause

## Author

Kevin Allison
kvnallsn AT gmail.com
