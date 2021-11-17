use rocket::tokio::time;
use rocket::{fairing, http, request};
use std::collections;

type SessionMap = collections::HashMap<SessionToken, time::Instant>;
type SessionToken = String;

pub struct SessionManager(SessionMap);

impl SessionManager {
    pub fn fairing() -> impl fairing::Fairing {
        fairing::AdHoc::on_ignite("Session Manager", |rocket| async {
            rocket.manage(Self::new())
        })
    }

    pub fn new() -> Self {
        SessionManager(SessionMap::new())
    }

    pub fn generate_session(
        &mut self,
        validity_duration: time::Duration,
    ) -> (SessionToken, time::Instant) {
        let entry = (
            SessionToken::new(),
            time::Instant::now() + validity_duration,
        );
        self.0.insert(entry.0.clone(), entry.1.clone());
        entry
    }

    pub fn get_session<'a>(
        &self,
        key: &'a SessionToken,
    ) -> Option<(&'a SessionToken, &time::Instant)> {
        self.0.get(key).map(|i| (key, i))
    }

    pub fn is_session_valid(&self, key: &SessionToken) -> Option<bool> {
        self.get_session(key).map(|s| *s.1 > time::Instant::now())
    }
}

pub struct TokenAuth<M>(M);

#[derive(Debug)]
pub enum TokenAuthError {
    NoTokenSpecified,
    NoSuchToken,
    ExpiredToken,
    NoSessionManager,
}

impl<M: AuthMethod> From<TokenAuthError> for request::Outcome<TokenAuth<M>, TokenAuthError> {
    fn from(error: TokenAuthError) -> Self {
        let status = match error {
            TokenAuthError::NoTokenSpecified => http::Status::BadRequest,
            TokenAuthError::NoSuchToken => http::Status::Unauthorized,
            TokenAuthError::ExpiredToken => http::Status::Unauthorized,
            TokenAuthError::NoSessionManager => http::Status::InternalServerError,
        };
        request::Outcome::Failure((status, error))
    }
}

#[rocket::async_trait]
impl<'r, M: AuthMethod> request::FromRequest<'r> for TokenAuth<M> {
    type Error = TokenAuthError;

    async fn from_request(
        request: &'r request::Request<'_>,
    ) -> request::Outcome<Self, Self::Error> {
        match request.rocket().state::<SessionManager>() {
            Some(manager) => match M::retrieve_token(request) {
                Some(token) => match manager.is_session_valid(&token) {
                    Some(valid) => {
                        if !valid {
                            return Self::Error::ExpiredToken.into();
                        }
                        request::Outcome::Success(Self(M::default()))
                    }
                    None => Self::Error::NoSuchToken.into(),
                },
                None => Self::Error::NoTokenSpecified.into(),
            },
            None => Self::Error::NoSessionManager.into(),
        }
    }
}

pub trait AuthMethod: Default {
    fn retrieve_token(request: &request::Request) -> Option<SessionToken>;
}

#[derive(Default)]
pub struct WithCookie;

impl AuthMethod for WithCookie {
    fn retrieve_token(request: &request::Request) -> Option<SessionToken> {
        request
            .cookies()
            .get("_session_token")
            .map(|c| SessionToken::from(c.value()))
    }
}

#[derive(Default)]
pub struct WithHeader;

impl AuthMethod for WithHeader {
    fn retrieve_token(request: &request::Request) -> Option<SessionToken> {
        // Some(request.headers().get_one("Authorization")?.split("Basic ").sum())
        todo!()
    }
}
