use rand::Rng;
use rocket::tokio::sync;
use rocket::{fairing, http, request};
use std::{collections, marker, time};

pub const SESSION_TOKEN_COOKIE: &str = "_session_token";

type SessionMap = collections::HashMap<String, time::Instant>;

pub struct SessionManager(SessionMap);

pub type SafeSessionManager = sync::Mutex<SessionManager>;

impl SessionManager {
    pub fn fairing() -> impl fairing::Fairing {
        fairing::AdHoc::on_ignite("Session Manager", |rocket| async {
            rocket.manage(SafeSessionManager::new(Self::new()))
        })
    }

    pub fn new() -> Self {
        SessionManager(SessionMap::new())
    }

    pub fn generate_session(
        &mut self,
        token_len: usize,
        validity_duration: time::Duration,
    ) -> (String, time::Instant) {
        let entry = (
            gen_random_token(token_len),
            time::Instant::now() + validity_duration,
        );
        self.0.insert(entry.0.clone(), entry.1);
        entry
    }

    pub fn get_session<'a>(&self, key: &'a str) -> Option<(&'a str, &time::Instant)> {
        self.0.get(key).map(|i| (key, i))
    }

    pub fn is_session_valid(&self, key: &str) -> Option<bool> {
        self.get_session(key).map(|s| *s.1 > time::Instant::now())
    }
}

fn gen_random_token(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

pub struct TokenAuth<M>(marker::PhantomData<M>);

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
        match request.rocket().state::<SafeSessionManager>() {
            Some(manager) => match M::retrieve_token(request) {
                Some(token) => match manager.lock().await.is_session_valid(&token) {
                    Some(valid) => {
                        if !valid {
                            return Self::Error::ExpiredToken.into();
                        }
                        request::Outcome::Success(Self(marker::PhantomData))
                    }
                    None => Self::Error::NoSuchToken.into(),
                },
                None => Self::Error::NoTokenSpecified.into(),
            },
            None => Self::Error::NoSessionManager.into(),
        }
    }
}

pub trait AuthMethod {
    fn retrieve_token(request: &request::Request) -> Option<String>;
}

pub struct WithCookie;

impl AuthMethod for WithCookie {
    fn retrieve_token(request: &request::Request) -> Option<String> {
        request
            .cookies()
            .get(SESSION_TOKEN_COOKIE)
            .map(|c| c.value().to_string())
    }
}

pub struct WithHeader;

impl AuthMethod for WithHeader {
    fn retrieve_token(request: &request::Request) -> Option<String> {
        request
            .headers()
            .get_one("Authorization")?
            .split("Basic ")
            .collect::<Vec<&str>>()
            .first()
            .map(|s| s.to_string())
    }
}
