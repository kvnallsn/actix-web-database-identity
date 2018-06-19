//! A database (SQL) Identity Provider

#![feature(custom_attribute)]

extern crate actix;
extern crate actix_web;
extern crate base64;
extern crate failure;
extern crate futures;
extern crate rand;

#[macro_use] extern crate diesel;
#[macro_use] extern crate failure_derive;

mod sql;

use std::rc::Rc;

use failure::Error;

use actix::Addr;
use actix::prelude::Syn;

// Actix Web imports
use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use actix_web::error::{Error as ActixWebError};
use actix_web::middleware::{Response as MiddlewareResponse};
use actix_web::middleware::identity::{Identity, IdentityPolicy};

// Futures imports
use futures::Future;
use futures::future::{ok as FutOk};

// (Local) Sql Imports
use sql::{DeleteIdentity, FindIdentity, UpdateIdentity, SqlActor, SqlIdentityModel};

// Rand Imports (thread secure!)
use rand::Rng;

/// Error representing different failure cases
#[derive(Debug, Fail)]
enum SqlIdentityError {
    #[fail(display = "sql variant not supported")]
    SqlVariantNotSupported,

    #[fail(display = "token not found")]
    SqlTokenNotFound,
}

/// Identity that uses a SQL database as identity storage
pub struct SqlIdentity {
    changed: bool,
    identity: Option<String>,
    token: Option<String>,
    inner: Rc<SqlIdentityInner>,
}

impl Identity for SqlIdentity {
    
    /// Returns the current identity, or none
    fn identity(&self) -> Option<&str> {
        self.identity.as_ref().map(|s| s.as_ref())
    }

    /// Remembers a given user (by setting a token value)
    ///
    /// # Arguments
    ///
    /// * `value` - User to remember
    fn remember(&mut self, value: String) {
        self.changed = true;
        self.identity = Some(value);

        // Generate a random token
        let mut arr = [0u8; 24];
        rand::thread_rng().fill(&mut arr[..]);
        self.token = Some(base64::encode(&arr));
    }

    /// Forgets a user, by deleting the identity
    fn forget(&mut self) {
        self.changed = true;
        self.identity = None;
    }

    /// Saves the identity to the backing store, if it has changed
    /// 
    /// # Arguments
    ///
    /// * `resp` - HTTP response to modify
    fn write(&mut self, resp: HttpResponse) -> Result<MiddlewareResponse, ActixWebError> {
        let token = self.token.clone();

        if let Some(ref token) = token {
            if self.changed {
                if let Some(ref identity) = self.identity {
                    // Insert or update
                    return Ok(MiddlewareResponse::Future(
                            self.inner.save(token, identity, resp)
                    ));
                } else {
                    // Delete token
                    return Ok(MiddlewareResponse::Future(
                            self.inner.remove(token, resp)
                    ));
                }
            }
        }

        Ok(MiddlewareResponse::Done(resp))
    }
}

/// Wrapped inner-provider for SQL storage
struct SqlIdentityInner {
    addr: Addr<Syn, SqlActor>,
}

impl SqlIdentityInner {
    /// Creates a new instance of a SqlIdentityInner struct
    ///
    /// # Arguments
    ///
    /// * `addr` - A SQL connection, already opened
    fn new(addr: Addr<Syn, SqlActor>) -> SqlIdentityInner {
        SqlIdentityInner {
            addr,
        }
    }

    /// Saves an identity to the backend provider (SQL database)
    fn save(&self, token: &str, userid: &str, mut resp: HttpResponse) -> Box<Future<Item = HttpResponse, Error = ActixWebError>> {

        {
            // Add the new token/identity to response headers
            let headers = resp.headers_mut();
            headers.append("Twinscroll-Auth", token.parse().unwrap());
        }

        Box::new(
            self.addr
                .send(UpdateIdentity { token: token.to_string(), userid: userid.to_string() })
                .map_err(ActixWebError::from)
                .and_then(move |res| match res {
                    Ok(_) => Ok(resp),
                    Err(_) => Ok(HttpResponse::InternalServerError().finish()),
                }),
        )
    }

    /// Removes an identity from the backend provider (SQL database)
    fn remove(&self, token: &str, resp: HttpResponse) -> Box<Future<Item = HttpResponse, Error = ActixWebError>> {

        Box::new(
            self.addr
                .send(DeleteIdentity { token: token.to_string() })
                .map_err(ActixWebError::from)
                .and_then(move |res| match res {
                    Ok(_) => Ok(resp),
                    Err(_) => Ok(HttpResponse::InternalServerError().finish()),
                }),
        )
    }

    /// Loads an identity from the backend provider (SQL database)
    fn load<S>(&self, req: &HttpRequest<S>) -> Box<Future<Item = Option<SqlIdentityModel>, Error = ActixWebError>> {
        let headers = req.headers();
        let auth_header = headers.get("Authorization");

        if let Some(auth_header) = auth_header {
            // Return the identity (or none, if it doesn't exist)
           
            if let Ok(auth_header) = auth_header.to_str() {
                let mut iter = auth_header.split(' ');
                let scheme = iter.next();
                let token = iter.next();

                if scheme.is_some() && token.is_some() {
                    let _scheme = scheme.unwrap();
                    let token = token.unwrap();

                    return Box::new(
                        self.addr
                            .send(FindIdentity { token: token.to_string() })
                            .map_err(ActixWebError::from)
                            .and_then(move |res| match res {
                                Ok(val) => Ok(Some(val)),
                                Err(_) => Ok(None),
                            }),
                    );
                }
            }

            // return Box::new(FutErr(awerror::ErrorBadRequest("Invalid Authorization Header")));
        }

        // Box::new(FutErr(awerror::ErrorBadRequest("No Authorization Header")))

        Box::new(FutOk(None))
    }
}

/// Use a SQL database for request identity storage
pub struct SqlIdentityPolicy(Rc<SqlIdentityInner>);

impl SqlIdentityPolicy {
    /// Creates a new SQLite identity policy
    ///
    /// # Arguments
    ///
    /// * `s` - Sqlite connection string (e.g., sqlite://test.db)
    pub fn sqlite(s: &str) -> Result<SqlIdentityPolicy, Error> {
        Ok(SqlIdentityPolicy(Rc::new(SqlIdentityInner::new(SqlActor::sqlite(3, s)?))))
    }

    /// Creates a new MySQL identity policy
    ///
    /// # Arguments
    ///
    /// * `s` - MySQL connection string
    pub fn mysql(s: &str) -> Result<SqlIdentityPolicy, Error> {
        Ok(SqlIdentityPolicy(Rc::new(SqlIdentityInner::new(SqlActor::mysql(3, s)?))))
    }

    /// Creates a new PostgreSQL identity policy
    ///
    /// # Arguments
    ///
    /// * `s` - PostgresSQL connection string (e.g., psql://user@localhost:3339)
    pub fn postgres(s: &str) -> Result<SqlIdentityPolicy, Error> {
        Ok(SqlIdentityPolicy(Rc::new(SqlIdentityInner::new(SqlActor::pg(3, s)?))))
    }
}

impl<S> IdentityPolicy<S> for SqlIdentityPolicy {
    type Identity = SqlIdentity;
    type Future = Box<Future<Item = SqlIdentity, Error = ActixWebError>>;

    /// Process a request recieved by the server, returns an Identity struct
    ///
    /// # Arguments
    ///
    /// * `req` - The HTTP request recieved
    fn from_request(&self, req: &mut HttpRequest<S>) -> Self::Future {
        let inner = Rc::clone(&self.0);

        Box::new(self.0.load(req).map(move |ident| {
            if let Some(id) = ident {
                SqlIdentity {
                    identity: Some(id.userid),
                    token: Some(id.token),
                    changed: false,
                    inner: inner,
                }
            } else {
                SqlIdentity {
                    identity: None,
                    token: None,
                    changed: false,
                    inner: inner
                }
            }
        }))
    }
}

