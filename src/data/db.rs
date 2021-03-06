use config::SeriatimConfig;

use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Outcome, Request, State};

type PgPool = Pool<ConnectionManager<PgConnection>>;

pub fn init_pool(cfg: &SeriatimConfig) -> PgPool {
    let manager = ConnectionManager::<PgConnection>::new(cfg.database_url.clone());
    Pool::new(manager).expect("Could not connect to database")
}

pub struct Connection {
    pub pg_connection: PooledConnection<ConnectionManager<PgConnection>>,
}

impl<'a, 'r> FromRequest<'a, 'r> for Connection {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let pool = request.guard::<State<PgPool>>()?;
        match pool.get() {
            Ok(pg_connection) => Outcome::Success(Connection { pg_connection }),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
        }
    }
}
