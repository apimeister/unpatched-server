use axum::{
    async_trait,
    extract::{FromRequestParts, State, TypedHeader},
    headers::{authorization::Bearer, Authorization},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json, RequestPartsExt,
};
use chrono::{Days, Utc};
use email_address::EmailAddress;
use headers::Cookie;

use hyper::header::SET_COOKIE;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;
use std::{
    fmt::{Debug, Display},
    str::FromStr,
};
use tracing::error;
use uuid::Uuid;

use crate::user::get_users_from_db;

pub static KEYS: Lazy<Keys> = Lazy::new(|| {
    let file = std::fs::read("jwt.pk8").unwrap();
    let rsa_file: &[u8] = &file;
    Keys::new(rsa_file)
});

//TODO: Remove or keep as test endpoint
pub async fn protected(claims: Claims) -> Result<String, AuthError> {
    // Send the protected data to the user
    Ok(format!(
        "Welcome to the protected area :)\nYour data:\n{}",
        claims
    ))
}

pub async fn api_authorize_user(
    State(pool): State<SqlitePool>,
    Json(payload): Json<AuthPayload>,
) -> Result<Response, AuthError> {
    // Check if the user sent the credentials
    if payload.client_id.is_empty() || payload.client_secret.is_empty() {
        return Err(AuthError::MissingCredentials);
    }
    let _validate_email =
        EmailAddress::from_str(&payload.client_id).map_err(|_| AuthError::InvalidEmail)?;
    // Here you can check the user credentials from a database
    let filter = format!("email='{}'", payload.client_id);
    let users = get_users_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    let Some(user) = users.first() else {
        return Err(AuthError::WrongCredentials);
    };
    if user
        .verify_password(payload.client_secret.as_bytes())
        .is_err()
    {
        return Err(AuthError::WrongCredentials);
    }
    let claims = Claims {
        sub: user.email.clone(),
        // FIXME: hardcoded value!
        exp: Utc::now()
            .checked_add_days(Days::new(30))
            .unwrap()
            .timestamp() as usize,
        iss: "unpatched-server".to_string(),
        aud: "unpatched-server-users".to_string(),
        nbf: None,
        iat: Utc::now().timestamp() as usize,
        jti: Uuid::new_v4(),
    };
    // Create the authorization token
    let token = encode(
        &Header::new(jsonwebtoken::Algorithm::RS256),
        &claims,
        &KEYS.encoding,
    )
    .map_err(|_| AuthError::TokenCreation)?;
    // Send the authorized token (as payload for apis and cookie header for webpage)
    let body = Json(AuthBody::new(token.clone()));
    let mut res = (StatusCode::OK, body).into_response();
    let cookie = format!("unpatched_token={token}; SameSite=Strict; Secure; Path=/; HttpOnly; max-age=max-age-in-seconds=31536000");
    res.headers_mut().insert(
        SET_COOKIE,
        cookie.parse().map_err(|_| AuthError::TokenCreation)?,
    );

    Ok(res)
}

impl Display for Claims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,"Email: {}\nExpiry: {}\nIssuer: {}\nAudience: {}\nNot-before: {}\nIssued-at-time: {}\nJWT ID: {}",
            self.sub, self.exp, self.iss, self.aud, self.nbf.unwrap_or(0), self.iat, self.jti
        )
    }
}

impl AuthBody {
    fn new(access_token: String) -> Self {
        Self {
            access_token,
            token_type: "Bearer".to_string(),
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let token = if parts
            .headers
            .contains_key(axum::http::header::AUTHORIZATION)
        {
            // Extract the token from the authorization header
            let TypedHeader(Authorization(bearer)) = parts
                .extract::<TypedHeader<Authorization<Bearer>>>()
                .await
                .map_err(|_| AuthError::InvalidToken)?;
            bearer.token().to_string()
        } else if parts.headers.contains_key(axum::http::header::COOKIE) {
            // Extract the token from cookie
            let TypedHeader(cookies) = parts
                .extract::<TypedHeader<Cookie>>()
                .await
                .map_err(|_| AuthError::InvalidToken)?;
            let Some(coo) = cookies.get("unpatched_token") else {
                return Err(AuthError::InvalidToken);
            };
            coo.to_string()
        } else {
            return Err(AuthError::InvalidToken);
        };
        // Decode the user data
        let token_data = match decode::<Claims>(&token, &KEYS.decoding, &Validation::default()) {
            Ok(a) => a,
            Err(e) => {
                error!("{e}");
                return Err(AuthError::InvalidToken);
            }
        };
        Ok(token_data.claims)
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation error"),
            AuthError::InvalidEmail => (StatusCode::NOT_ACCEPTABLE, "Not a valid email address"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

pub struct Keys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl Keys {
    fn new(rsa_file: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_rsa_der(rsa_file),
            decoding: DecodingKey::from_rsa_der(rsa_file),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// From https://auth0.com/docs/secure/tokens/json-web-tokens/json-web-token-claims#registered-claims
///
/// The JWT specification defines seven reserved claims that are not required, but are recommended to allow interoperability with third-party applications. These are:
/// - iss (issuer): Issuer of the JWT
/// - sub (subject): Subject of the JWT (the user)
/// - aud (audience): Recipient for which the JWT is intended
/// - exp (expiration time): Time after which the JWT expires
/// - nbf (not before time): Time before which the JWT must not be accepted for processing
/// - iat (issued at time): Time at which the JWT was issued; can be used to determine age of the JWT
/// - jti (JWT ID): Unique identifier; can be used to prevent the JWT from being replayed (allows a token to be used only once)
pub struct Claims {
    pub iss: String,
    pub sub: EmailAddress,
    pub aud: String,
    pub exp: usize,
    pub nbf: Option<usize>,
    pub iat: usize,
    pub jti: Uuid,
}

impl Default for Claims {
    fn default() -> Self {
        Claims {
            iss: Default::default(),
            sub: EmailAddress::new_unchecked("default@default.int"),
            aud: Default::default(),
            exp: Default::default(),
            nbf: Default::default(),
            iat: Default::default(),
            jti: Default::default(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AuthBody {
    pub access_token: String,
    pub token_type: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthPayload {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug)]
pub enum AuthError {
    WrongCredentials,
    MissingCredentials,
    TokenCreation,
    InvalidToken,
    InvalidEmail,
}

// FIXME: Make this work

// #[derive(Debug, Clone)]
// pub struct AuthLayer {
//     target: &'static str,
// }

// impl<S> Layer<S> for AuthLayer {
//     type Service = AuthService<S>;

//     fn layer(&self, service: S) -> Self::Service {
//         AuthService {
//             target: self.target,
//             service,
//         }
//     }
// }

// impl AuthLayer {
//     pub fn verify() -> Self {
//         error!("loaded auth layer");
//         AuthLayer { target: "auth" }
//     }
// }
// // This service implements the Log behavior
// #[derive(Clone)]
// pub struct AuthService<S> {
//     target: &'static str,
//     service: S,
// }

// impl<S, Request> Service<Request> for AuthService<S>
// where
//     S: Service<Request>,
//     Request: std::fmt::Debug,
// {
//     type Response = S::Response;
//     type Error = S::Error;
//     type Future = S::Future;

//     fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         self.service.poll_ready(cx)
//     }

//     fn call(&mut self, request: Request) -> Self::Future {
//         // Insert log statement here or other functionality
//         error!("request = {:?}, target = {:?}", request, self.target);
//         self.service.call(request)
//     }
// }
