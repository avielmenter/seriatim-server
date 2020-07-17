use data::memory::redis::Connection;
use data::user::UserID;

use r2d2_redis::redis;
use r2d2_redis::redis::Commands;

use uuid;

#[derive(TaggedID, CookieGuard)]
pub struct SessionID(uuid::Uuid);

pub struct Session {
    connection: Connection,
    pub session_id: SessionID,
    pub user_id: UserID,
}

impl<'a> Session {
    pub fn get_by_id(connection: Connection, p_session_id: &SessionID) -> redis::RedisResult<Self> {
        let user_id: UserID = connection
            .redis_connection
            .write()
            .unwrap()
            .get(p_session_id)?;

        //let user_id = UserID::from_str(&str_user_id).unwrap();
        let session_id: SessionID = p_session_id.clone();

        Ok(Session {
            connection,
            session_id,
            user_id,
        })
    }

    pub fn create(connection: Connection, p_user_id: &UserID) -> redis::RedisResult<Self> {
        let mut session = Session {
            connection,
            session_id: SessionID::generate(),
            user_id: UserID::generate(), //dummy
        };

        let _: () = session
            .connection
            .redis_connection
            .write()
            .and_then(|mut con| {
                con.set(&session.session_id, p_user_id)
                    .or(Err(std::sync::PoisonError::new(con)))
            })
            .or(Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "Could not get a lock on the Redis connection",
            )))?;

        session.user_id = p_user_id.clone();
        Ok(session)
    }
}

impl<'a, 'r> rocket::request::FromRequest<'a, 'r> for Session {
    type Error = ();

    fn from_request(
        request: &'a rocket::request::Request<'r>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let con = request.guard::<Connection>()?;

        SessionID::from_cookie(&mut request.cookies())
            .and_then(move |session_id| Session::get_by_id(con, &session_id).ok())
            .or_forward(())
    }
}
