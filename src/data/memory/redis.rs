use config::SeriatimConfig;

use r2d2_redis::{r2d2, RedisConnectionManager};

use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Outcome, Request, State};

use std::sync::Mutex;

type RedisPool = r2d2::Pool<RedisConnectionManager>;

pub fn init_pool(cfg: &SeriatimConfig) -> Result<RedisPool, Box<dyn std::error::Error>> {
    let manager = RedisConnectionManager::new(cfg.redis_url.clone())?;
    let pool = r2d2::Pool::builder().build(manager)?;
    Ok(pool)
}

pub struct Connection {
    pub redis_connection: Mutex<r2d2::PooledConnection<RedisConnectionManager>>,
}

impl<'a, 'r> FromRequest<'a, 'r> for Connection {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let pool = request.guard::<State<RedisPool>>()?;
        match pool.get() {
            Ok(redis_connection) => Outcome::Success(Connection {
                redis_connection: Mutex::new(redis_connection),
            }),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
        }
    }
}
