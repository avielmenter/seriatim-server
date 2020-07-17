use data::memory::redis::Connection;
use data::user::UserID;

use r2d2_redis::redis::{Commands, RedisResult};
use r2d2_redis::{r2d2, RedisConnectionManager};

use std::collections::HashSet;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::SystemTime;

use uuid;

#[derive(TaggedID, CookieGuard, Serialize, Deserialize)]
pub struct SessionID(uuid::Uuid);

#[derive(Serialize, Deserialize, RedisData)]
pub struct Data {
    pub ip: IpAddr,
    pub session_id: SessionID,
    pub time_created: SystemTime,
    pub time_last_login: SystemTime,
    pub user_id: UserID,
}

pub struct Session {
    connection: Rc<Connection>,
    pub data: Data,
}

type ConnectionLock<'m> = std::sync::MutexGuard<'m, r2d2::PooledConnection<RedisConnectionManager>>;

fn connection_error(msg: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::ConnectionRefused, msg)
}

fn connection_lock(connection: &Rc<Connection>) -> RedisResult<ConnectionLock> {
    let con = connection.redis_connection.lock().or(Err(connection_error(
        "Could not get a lock on the Redis connection",
    )))?;
    Ok(con)
}

impl Session {
    fn get_connection(&self) -> RedisResult<ConnectionLock> {
        connection_lock(&self.connection)
    }

    pub fn get_by_id(connection: Rc<Connection>, p_session_id: &SessionID) -> RedisResult<Self> {
        let data: Data = connection_lock(&connection)?.get(p_session_id)?;

        Ok(Session { connection, data })
    }

    pub fn get_by_user_id(
        connection: Rc<Connection>,
        p_user_id: &UserID,
    ) -> RedisResult<Vec<Self>> {
        let session_ids: HashSet<SessionID> = connection_lock(&connection)?.smembers(p_user_id)?;

        session_ids
            .iter()
            .map(|session_id| Session::get_by_id(connection.clone(), session_id))
            .collect()
    }

    pub fn create(
        connection: Rc<Connection>,
        p_user_id: &UserID,
        p_ip_addr: &IpAddr,
    ) -> RedisResult<Self> {
        let data = Data {
            ip: p_ip_addr.clone(),
            session_id: SessionID::generate(),
            time_created: SystemTime::now(),
            time_last_login: SystemTime::now(),
            user_id: p_user_id.clone(),
        };
        let mut session = Session { connection, data };
        session.update_data()?;
        session.create_user_index()?;

        Ok(session)
    }

    // pub fn check_ip(self, ip: &IpAddr) -> RedisResult<Self> {
    //     if self.data.ip == *ip {
    //         Ok(self)
    //     } else {
    //         Err(redis::RedisError::from(std::io::Error::new(
    //             std::io::ErrorKind::PermissionDenied,
    //             "Invalid IP address for this session",
    //         )))
    //     }
    // }

    pub fn create_user_index(&mut self) -> RedisResult<()> {
        self.get_connection()?
            .sadd(&self.data.user_id, &self.data.session_id)
    }

    pub fn delete(&mut self) -> RedisResult<()> {
        let mut con = self.get_connection()?;
        con.del(&self.data.session_id)?;
        con.srem(&self.data.user_id, &self.data.session_id)
    }

    pub fn trim_oldest_sessions(
        connection: Rc<Connection>,
        p_user_id: &UserID,
        keep: usize,
    ) -> RedisResult<()> {
        let mut sessions = Self::get_by_user_id(connection, &p_user_id)?;
        sessions.sort_by_key(|session| session.data.time_last_login);

        let num_sessions = sessions.len();
        sessions
            .into_iter()
            .enumerate()
            .filter_map(|(i, mut session)| {
                if keep > 0 && num_sessions > keep && i < num_sessions - keep {
                    Some(session.delete())
                } else {
                    None
                }
            })
            .collect::<RedisResult<Vec<()>>>()?;

        Ok(())
    }

    fn update_data(&mut self) -> RedisResult<()> {
        self.get_connection()?
            .set(&self.data.session_id, &self.data)
    }

    pub fn update_login_time(&mut self) -> RedisResult<()> {
        self.data.time_last_login = SystemTime::now();
        self.update_data()
    }
}

impl<'a, 'r> rocket::request::FromRequest<'a, 'r> for Session {
    type Error = ();

    fn from_request(
        request: &'a rocket::request::Request<'r>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let con = Rc::new(request.guard::<Connection>()?);

        SessionID::from_cookie(&mut request.cookies())
            .and_then(move |session_id| {
                // let ip = request.client_ip()?;
                Session::get_by_id(con, &session_id).ok() //?
                                                          // .check_ip(&ip)
                                                          // .ok()
            })
            .or_forward(())
    }
}
