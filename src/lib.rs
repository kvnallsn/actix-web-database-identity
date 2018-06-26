//! A database (SQL) Identity Provider

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
    #[allow(dead_code)]
    #[fail(display = "sql variant not supported")]
    SqlVariantNotSupported,

    #[fail(display = "token not found")]
    SqlTokenNotFound,
}

enum SqlIdentityState {
    Saved,
    Deleted,
    Unchanged,
}

/// Identity that uses a SQL database as identity storage
pub struct SqlIdentity {
    state: SqlIdentityState,
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
        self.identity = Some(value);

        // Generate a random token
        let mut arr = [0u8; 24];
        rand::thread_rng().fill(&mut arr[..]);
        self.token = Some(base64::encode(&arr));

        self.state = SqlIdentityState::Saved;
    }

    /// Forgets a user, by deleting the identity
    fn forget(&mut self) {
        self.identity = None;
        self.state = SqlIdentityState::Deleted;
    }

    /// Saves the identity to the backing store, if it has changed
    /// 
    /// # Arguments
    ///
    /// * `resp` - HTTP response to modify
    fn write(&mut self, resp: HttpResponse) -> Result<MiddlewareResponse, ActixWebError> {

        match self.state {
            SqlIdentityState::Saved if self.token.is_some() && self.identity.is_some() => {
                let token = self.token.as_ref().unwrap();
                let identity = self.identity.as_ref().unwrap();
                self.state = SqlIdentityState::Unchanged;
                Ok(MiddlewareResponse::Future(
                        self.inner.save(token, identity, resp)
                ))
            },

            SqlIdentityState::Deleted if self.token.is_some() => {
                let token = self.token.as_ref().unwrap();
                self.state = SqlIdentityState::Unchanged;
                Ok(MiddlewareResponse::Future(
                        self.inner.remove(token, resp)
                ))
            },

            SqlIdentityState::Deleted | SqlIdentityState::Saved => {
                // Not logged in/log in failed
                Ok(MiddlewareResponse::Done(
                        HttpResponse::BadRequest().finish()
                ))
            },

            _ => { 
                self.state = SqlIdentityState::Unchanged;
                Ok(MiddlewareResponse::Done(resp))
            }
        }
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
        }

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
    ///
    /// # Example
    ///
    /// ```no_run
    /// # extern crate actix_web;
    /// # extern crate actix_web_db_identity;
    ///
    /// use actix_web::App;
    /// use actix_web::middleware::identity::IdentityService;
    /// use actix_web_db_identity::SqlIdentityPolicy;
    ///
    /// let app = App::new().middleware(IdentityService::new(
    ///     // <- create sqlite identity middleware
    ///     SqlIdentityPolicy::sqlite("db.sqlite3").expect("failed to open database")
    /// ));
    /// ```
    pub fn sqlite(s: &str) -> Result<SqlIdentityPolicy, Error> {
        Ok(SqlIdentityPolicy(Rc::new(SqlIdentityInner::new(SqlActor::sqlite(3, s)?))))
    }

    /// Creates a new MySQL identity policy
    ///
    /// # Arguments
    ///
    /// * `s` - MySQL connection string
    ///
    /// # Example
    ///
    /// ```no_run
    /// # extern crate actix_web;
    /// # extern crate actix_web_db_identity;
    ///
    /// use actix_web::App;
    /// use actix_web::middleware::identity::IdentityService;
    /// use actix_web_db_identity::SqlIdentityPolicy;
    ///
    /// let app = App::new().middleware(IdentityService::new(
    ///     // <- create mysql identity middleware
    ///     SqlIdentityPolicy::mysql("server=127.0.0.1;uid=root;pwd=12345;database=test").expect("failed to open database")
    /// ));
    /// ```
    pub fn mysql(s: &str) -> Result<SqlIdentityPolicy, Error> {
        Ok(SqlIdentityPolicy(Rc::new(SqlIdentityInner::new(SqlActor::mysql(3, s)?))))
    }

    /// Creates a new PostgreSQL identity policy
    ///
    /// # Arguments
    ///
    /// * `s` - PostgresSQL connection string (e.g., psql://user@localhost:3339)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # extern crate actix_web;
    /// # extern crate actix_web_db_identity;
    ///
    /// use actix_web::App;
    /// use actix_web::middleware::identity::IdentityService;
    /// use actix_web_db_identity::SqlIdentityPolicy;
    ///
    /// let app = App::new().middleware(IdentityService::new(
    ///     // <- create postgresql identity middleware
    ///     SqlIdentityPolicy::postgres("postgresql://user:pass@localhost:5432/mydb").expect("failed to open database")
    /// ));
    /// ```
    pub fn postgres(s: &str) -> Result<SqlIdentityPolicy, Error> {
        debug!("Connecting to {}", s);
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
                    state: SqlIdentityState::Unchanged,
                    inner: inner,
                }
            } else {
                SqlIdentity {
                    identity: None,
                    token: None,
                    state: SqlIdentityState::Unchanged,
                    inner: inner
                }
            }
        }))
    }
}

