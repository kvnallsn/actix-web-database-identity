//! A database (SQL) Identity Provider

#![feature(custom_attribute)]

extern crate actix;
extern crate actix_web;
extern crate failure;
extern crate futures;

#[macro_use] extern crate diesel;
#[macro_use] extern crate failure_derive;

mod sql;

use std::rc::Rc;

use failure::Error;

use actix::Addr;
use actix::prelude::Syn;

// Actix Web imports
use actix_web::{HttpRequest, HttpResponse};
use actix_web::error::{Error as ActixWebError};
use actix_web::middleware::Response;
use actix_web::middleware::identity::{Identity, IdentityPolicy};

// Futures imports
use futures::Future;

// (Local) Sql Imports
use sql::{FindIdentity, UpdateIdentity, SqlActor, SqlIdentityModel};

/// Error representing different failure cases
#[derive(Debug, Fail)]
enum SqlIdentityError {
    #[fail(display = "sql variant not supported: {}", variant)]
    SqlVariantNotSupported {
        variant: String,
    },

    #[fail(display = "token not found")]
    SqlTokenNotFound,
}

/// Identity that uses a SQL database as identity storage
pub struct SqlIdentity {
    changed: bool,
    identity: Option<String>,
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
    fn write(&mut self, resp: HttpResponse) -> Result<Response, ActixWebError> {
        if self.changed {
            self.inner.save(self.identity.clone());
        }

        Ok(Response::Done(resp))
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

    fn save(&self, identity: Option<String>) -> Box<Future<Item = HttpResponse, Error = ActixWebError>> {
        // TODO: Make it actually save

        return Box::new(
            self.addr
                .send(UpdateIdentity { token: identity.unwrap(), userid: 1 })
                .map_err(ActixWebError::from)
                .and_then(move |res| match res {
                    Ok(_) => Ok(HttpResponse::Ok().finish()),
                    Err(_) => Ok(HttpResponse::InternalServerError().finish()),
                }),
        );
    }

    /// Loads an identity from the backend provider
    fn load<S>(&self, _req: &HttpRequest<S>) -> Box<Future<Item = Option<SqlIdentityModel>, Error = ActixWebError>> {
        // TODO: Extract the Auth Header

        // Return the identity (or none, if it doesn't exist)
        return Box::new(
            self.addr
                .send(FindIdentity { token: "test".to_string() })
                .map_err(ActixWebError::from)
                .and_then(move |res| match res {
                    Ok(val) => Ok(Some(val)),
                    Err(_) => Ok(None),
                }),
        );
    }
}

/// Use a SQL database for request identity storage
pub struct SqlIdentityPolicy(Rc<SqlIdentityInner>);

impl SqlIdentityPolicy {
    /// Creates a new sqlite identity policy
    ///
    /// # Arguments
    ///
    /// * `s` - Sqlite Connection String (e.g., sqlite://test.db)
    pub fn sqlite(s: &str) -> Result<SqlIdentityPolicy, Error> {
        Ok(SqlIdentityPolicy(Rc::new(SqlIdentityInner::new(SqlActor::sqlite(3, s)?))))
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
                    identity: Some(id.token),
                    changed: false,
                    inner: inner,
                }
            } else {
                SqlIdentity {
                    identity: None,
                    changed: false,
                    inner: inner
                }
            }
        }))
    }
}

