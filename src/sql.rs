//! SQL Actor

// Actix Imports
use actix::prelude::{Actor, Handler, Message};
use actix::sync::{SyncArbiter, SyncContext};
use actix::Addr;

use chrono::prelude::Utc;
use chrono::NaiveDateTime;

// Diesel (SQL ORM) Imports
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::{self, ExpressionMethods, QueryDsl, RunQueryDsl};

#[cfg(feature = "sqlite")]
use diesel::sqlite::SqliteConnection;

#[cfg(feature = "mysql")]
use diesel::mysql::MysqlConnection;

#[cfg(feature = "postgres")]
use diesel::pg::PgConnection;

// Failure (error management system) Imports
use failure::Error;

use super::{SqlIdentity, SqlIdentityError};

table! {
    identities (id) {
        id -> Int8,
        token -> Text,
        userid -> Text,
        ip -> Nullable<Text>,
        useragent -> Nullable<Text>,
        created -> Timestamp,
        modified -> Timestamp,
    }
}

/// Describes what type of SQL connection to create
#[derive(Clone, Debug)]
pub enum Variant {
    Sqlite,
    Mysql,
    Pg,
}

#[derive(Debug, Queryable)]
pub struct SqlIdentityModel {
    pub id: i64,
    pub token: String,
    pub userid: String,
    pub ip: Option<String>,
    pub useragent: Option<String>,
    pub created: NaiveDateTime,
    pub modified: NaiveDateTime,
}

/// Represents the different types of pools available
/// (e.g., SQLite, Postgresql, MySQL)
enum SqlPool {
    #[cfg(feature = "sqlite")]
    SqlitePool(Pool<ConnectionManager<SqliteConnection>>),

    #[cfg(feature = "mysql")]
    MySqlPool(Pool<ConnectionManager<MysqlConnection>>),

    #[cfg(feature = "postgres")]
    PgPool(Pool<ConnectionManager<PgConnection>>),
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
    pub fn sqlite(n: usize, s: &str) -> Result<Addr<SqlActor>, Error> {
        #[cfg(feature = "sqlite")]
        {
            let manager = ConnectionManager::<SqliteConnection>::new(s);
            let pool = Pool::builder().build(manager)?;

            let addr = SyncArbiter::start(n, move || SqlActor(SqlPool::SqlitePool(pool.clone())));
            Ok(addr)
        }

        #[cfg(not(feature = "sqlite"))]
        {
            let _ = n;
            let _ = s;
            warn!("SQLite support not enabled!");
            Err(SqlIdentityError::SqlVariantNotSupported.into())
        }
    }

    /// Creates a new MySQL Actor, for a connection to a MySQL database
    ///
    /// # Arguments
    ///
    /// * `n` - Number of threads
    /// * `s` - MySQL connection string
    pub fn mysql(n: usize, s: &str) -> Result<Addr<SqlActor>, Error> {
        #[cfg(feature = "mysql")]
        {
            let manager = ConnectionManager::<MysqlConnection>::new(s);
            let pool = Pool::builder().build(manager)?;

            let addr = SyncArbiter::start(n, move || SqlActor(SqlPool::MySqlPool(pool.clone())));
            Ok(addr)
        }

        #[cfg(not(feature = "mysql"))]
        {
            let _ = n;
            let _ = s;
            warn!("MySQL support not enabled!");
            Err(SqlIdentityError::SqlVariantNotSupported.into())
        }
    }

    /// Creates a new PostgresSQL Actor, for a connection to a PostgresSQL database
    ///
    /// # Arguments
    ///
    /// * `n` - Number of threads
    /// * `s` - PostgresSQL connection string
    pub fn pg(n: usize, s: &str) -> Result<Addr<SqlActor>, Error> {
        #[cfg(feature = "postgres")]
        {
            let manager = ConnectionManager::<PgConnection>::new(s);
            let pool = Pool::builder().build(manager)?;

            let addr = SyncArbiter::start(n, move || SqlActor(SqlPool::PgPool(pool.clone())));
            Ok(addr)
        }

        #[cfg(not(feature = "postgres"))]
        {
            let _ = n;
            let _ = s;
            warn!("PostgreSQL support not enabled!");
            Err(SqlIdentityError::SqlVariantNotSupported.into())
        }
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
        use self::identities::dsl::*;

        let session = match self.0 {
            #[cfg(feature = "sqlite")]
            SqlPool::SqlitePool(ref p) => {
                let conn: &SqliteConnection = &(*(p.get()?));
                identities.filter(token.eq(msg.token)).first(conn)?
            }

            #[cfg(feature = "mysql")]
            SqlPool::MySqlPool(ref p) => {
                let conn: &MysqlConnection = &(*(p.get()?));
                identities.filter(token.eq(msg.token)).first(conn)?
            }

            #[cfg(feature = "postgres")]
            SqlPool::PgPool(ref p) => {
                let conn: &PgConnection = &(*(p.get()?));
                identities.filter(token.eq(msg.token)).first(conn)?
            }
        };

        Ok(session)
    }
}

/// Inserts or Updates an Identity
#[derive(Debug, AsChangeset)]
#[table_name = "identities"]
pub struct UpdateIdentity {
    pub id: i64,
    pub ip: Option<String>,
    pub useragent: Option<String>,
    pub modified: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[table_name = "identities"]
pub struct CreateIdentity {
    pub token: String,
    pub userid: String,
    pub ip: Option<String>,
    pub useragent: Option<String>,
    pub created: NaiveDateTime,
    pub modified: NaiveDateTime,
}

impl UpdateIdentity {
    pub fn update(ident: &SqlIdentity) -> UpdateIdentity {
        let now = Utc::now();

        UpdateIdentity {
            id: ident.id,
            ip: ident.ip.clone(),
            useragent: ident.user_agent.clone(),
            modified: now.naive_utc(),
        }
    }

    pub fn create(ident: &SqlIdentity) -> CreateIdentity {
        let now = Utc::now();

        CreateIdentity {
            token: ident
                .token
                .as_ref()
                .map(|s| s.as_ref())
                .unwrap_or("")
                .to_string(),
            userid: ident
                .identity
                .as_ref()
                .map(|s| s.as_ref())
                .unwrap_or("")
                .to_string(),
            ip: ident.ip.clone(),
            useragent: ident.user_agent.clone(),
            created: ident.created,
            modified: now.naive_utc(),
        }
    }
}

impl Message for CreateIdentity {
    type Result = Result<usize, Error>;
}

impl Handler<CreateIdentity> for SqlActor {
    type Result = Result<usize, Error>;

    fn handle(&mut self, msg: CreateIdentity, _: &mut Self::Context) -> Self::Result {
        use self::identities::dsl::*;

        match self.0 {
            #[cfg(feature = "sqlite")]
            SqlPool::SqlitePool(ref p) => {
                let conn: &SqliteConnection = &(*(p.get()?));
                let n = diesel::insert_into(identities).values(&msg).execute(conn)?;

                Ok(n)
            }

            #[cfg(feature = "mysql")]
            SqlPool::MySqlPool(ref p) => {
                let conn: &MysqlConnection = &(*(p.get()?));
                let n = diesel::insert_into(identities).values(&msg).execute(conn)?;

                Ok(n)
            }

            #[cfg(feature = "postgres")]
            SqlPool::PgPool(ref p) => {
                let conn: &PgConnection = &(*(p.get()?));
                let n = diesel::insert_into(identities).values(&msg).execute(conn)?;

                Ok(n)
            }
        }
    }
}

impl Message for UpdateIdentity {
    type Result = Result<usize, Error>;
}

impl Handler<UpdateIdentity> for SqlActor {
    type Result = Result<usize, Error>;

    fn handle(&mut self, msg: UpdateIdentity, _: &mut Self::Context) -> Self::Result {
        use self::identities::dsl::*;

        match self.0 {
            #[cfg(feature = "sqlite")]
            SqlPool::SqlitePool(ref p) => {
                let conn: &SqliteConnection = &(*(p.get()?));
                //let n = diesel::replace_into(identities).values(&msg).execute(conn)?;
                let n = diesel::update(identities.find(msg.id))
                    .set(&msg)
                    .execute(conn)?;

                Ok(n)
            }

            #[cfg(feature = "mysql")]
            SqlPool::MySqlPool(ref p) => {
                let conn: &MysqlConnection = &(*(p.get()?));
                //let n = diesel::replace_into(identities).values(&msg).execute(conn)?;
                let n = diesel::update(identities.find(msg.id))
                    .set(&msg)
                    .execute(conn)?;

                Ok(n)
            }

            #[cfg(feature = "postgres")]
            SqlPool::PgPool(ref p) => {
                let conn: &PgConnection = &(*(p.get()?));
                let n = diesel::update(identities.find(msg.id))
                    .set(&msg)
                    .execute(conn)?;
                //let n = diesel::insert_into(identities)
                //.values(&msg)
                //.on_conflict(token)
                //.do_update()
                //.set(&msg)
                //.execute(conn)?;

                Ok(n)
            }
        }
    }
}

/// Deletes an identity from the table (aka logout)
pub struct DeleteIdentity {
    pub token: String,
}

impl Message for DeleteIdentity {
    type Result = Result<usize, Error>;
}

impl Handler<DeleteIdentity> for SqlActor {
    type Result = Result<usize, Error>;

    fn handle(&mut self, msg: DeleteIdentity, _: &mut Self::Context) -> Self::Result {
        use self::identities::dsl::*;

        match self.0 {
            #[cfg(feature = "sqlite")]
            SqlPool::SqlitePool(ref p) => {
                let conn: &SqliteConnection = &(*(p.get()?));
                let n = diesel::delete(identities.filter(token.eq(msg.token))).execute(conn)?;

                Ok(n)
            }

            #[cfg(feature = "mysql")]
            SqlPool::MySqlPool(ref p) => {
                let conn: &MysqlConnection = &(*(p.get()?));
                let n = diesel::delete(identities.filter(token.eq(msg.token))).execute(conn)?;
                Ok(n)
            }

            #[cfg(feature = "postgres")]
            SqlPool::PgPool(ref p) => {
                let conn: &PgConnection = &(*(p.get()?));
                let n = diesel::delete(identities.filter(token.eq(msg.token))).execute(conn)?;
                Ok(n)
            }
        }
    }
}
