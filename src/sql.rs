//! SQL Actor

// Actix Imports
use actix::Addr;
use actix::prelude::{Actor, Handler, Message, Syn};
use actix::sync::{SyncArbiter, SyncContext};

// Diesel (SQL ORM) Imports
use diesel::{ExpressionMethods, RunQueryDsl, QueryDsl};
use diesel::sqlite::SqliteConnection;
use diesel::r2d2::{ConnectionManager, Pool};

// Failure (error management system) Imports
use failure::Error;

use super::SqlIdentityError;

table! {
    identities (id) {
        id -> Integer,
        token -> Text,
    }
}

#[derive(Queryable)]
pub struct SqlIdentityModel {
    pub id: i32,
    pub token: String
}

/// Represents the different types of pools available
/// (e.g., SQLite, Postgresql, MySQL)
enum SqlPool {
    SqlitePool(Pool<ConnectionManager<SqliteConnection>>),
}

/// Represents an actix SQL actor
pub struct SqlActor(SqlPool);

impl SqlActor {
    /// Creates a new SQLite Actor, for a connection to a SQLite database
    ///
    /// # Arguments
    ///
    /// * `n` - Number of threads
    /// * `s` - SQLite connection string
    pub fn sqlite(n: usize, s: &str) -> Result<Addr<Syn, SqlActor>, Error> {
        let manager = ConnectionManager::<SqliteConnection>::new(s);
        let pool = Pool::builder()
            .build(manager)?;

        let addr = SyncArbiter::start(n, move || SqlActor(SqlPool::SqlitePool(pool.clone())));
        Ok(addr)
    }
}

impl Actor for SqlActor {
    type Context = SyncContext<Self>;
}

/// Searches for given identity based on a token value
pub struct FindIdentity {
    pub token: String,   
}

impl Message for FindIdentity {
    type Result = Result<SqlIdentityModel, Error>;
}

impl Handler<FindIdentity> for SqlActor {
    type Result = Result<SqlIdentityModel, Error>;

    fn handle(&mut self, msg: FindIdentity, _: &mut Self::Context) -> Self::Result {
        match self.0 {
            SqlPool::SqlitePool(ref p) => {
                use self::identities::dsl::*;

                let conn: &SqliteConnection = &(*(p.get()?)); 
                let mut results = identities.filter(token.eq(msg.token))
                    .limit(1)
                    .load::<SqlIdentityModel>(conn)?;

                if results.len() == 1 {
                    return Ok(results.remove(0));
                }
            },
        }

        Err(SqlIdentityError::SqlTokenNotFound.into())
    }
}
