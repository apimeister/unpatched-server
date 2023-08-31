use axum::{
    async_trait,
    extract::{FromRequestParts, State, TypedHeader},
    headers::{authorization::Bearer, Authorization},
    http::{request::Parts, StatusCode},
    response::{Html, IntoResponse, Response},
    Json, RequestPartsExt,
};
use chrono::{Utc, Days};
use email_address::EmailAddress;
use headers::Cookie;

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;
use tower::{Service, Layer};
use std::{fmt::{Display}};
use tracing::{info, error};
use futures_util::task::Context;
use futures_util::task::Poll;   
use uuid::Uuid;

use crate::{user::get_users_from_db};

pub static KEYS: Lazy<Keys> = Lazy::new(|| {
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    Keys::new(secret.as_bytes())
});

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
) -> Result<Json<AuthBody>, AuthError> {
    // Check if the user sent the credentials
    if payload.client_id.is_empty() || payload.client_secret.is_empty() {
        return Err(AuthError::MissingCredentials);
    }
    // Here you can check the user credentials from a database
    let filter = format!("email='{}'", payload.client_id);
    let users = get_users_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    let Some(user) = users.first() else {
        return Err(AuthError::MissingCredentials);
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
        exp: Utc::now().checked_add_days(Days::new(30)).unwrap().timestamp() as usize,
        iss: "unpatched-server".to_string(),
        aud: "unpatched-server-users".to_string(),
        nbf: None,
        iat: Utc::now().timestamp() as usize,
        jti: Uuid::new_v4(),
    };
    // Create the authorization token
    let token = encode(&Header::default(), &claims, &KEYS.encoding)
        .map_err(|_| AuthError::TokenCreation)?;

    // Send the authorized token
    Ok(Json(AuthBody::new(token)))
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
            info!("{:?}", cookies);
            let Some(coo) = cookies.get("unpatched_token") else {
                return Err(AuthError::InvalidToken);
            };
            info!("{:?}", coo);
            coo.to_string()
        } else {
            return Err(AuthError::InvalidToken);
        };
        // Decode the user data
        let token_data = match decode::<Claims>(&token, &KEYS.decoding, &Validation::default()){
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
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
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
}

pub async fn login_ui() -> impl IntoResponse {
    let html = r##"<!DOCTYPE html>
    <html lang="en">
    <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1, viewport-fit=cover">
        <title>Unpatched Server</title>
        <!-- bootstrap -->
        <link href="/bootstrap/5.2.3/css/bootstrap.min.css" rel="stylesheet">
        <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bootstrap-icons@1.10.5/font/bootstrap-icons.css">
        <script src="/bootstrap/5.2.3/js/bootstrap.bundle.min.js"></script>
        <link rel="icon" href="/bandaid.svg">
    </head>
    <body>
    <div class="container min-vh-100">
    <div class="row align-items-center min-vh-100">
    <div class="col"></div>
    <div class="col-md-6">
    <h1><i class="bi bi-bandaid"></i> Unpatched Server Login</h1>
    <form>
    <div class="form-outline mb-4">
      <input type="email" id="loginEmail1" class="form-control" name="client_id" required />
      <label class="form-label" for="loginEmail1">Email address</label>
    </div>
      <div class="form-outline mb-4">
      <input type="password" id="loginPw1" class="form-control" name="client_secret" required />
      <label class="form-label" for="loginPw1">Password</label>
    </div>
    <div class="row mb-4">
      <div class="col d-flex justify-content-center">
        <div class="form-check">
          <input class="form-check-input" type="checkbox" value="" id="loginremember1" checked />
          <label class="form-check-label" for="loginremember1"> Remember me (not implemented!)</label>
        </div>
      </div>
      <div class="col">
        <a href="#!">Forgot password? (not implemented!)</a>
      </div>
    </div>
    <button type="button" class="btn btn-primary btn-block mb-4" onClick="login(this.form)">Sign in</button>
    <div class="text-center">
      <p>Not a member? (not implemented!)<a href="#!">Register</a></p>
      <p>or sign up with: (not implemented!)</p>
      <button type="button" class="btn btn-link btn-floating mx-1">
        <i class="bi bi-facebook"></i>
      </button>
      <button type="button" class="btn btn-link btn-floating mx-1">
        <i class="bi bi-google"></i>
      </button>
      <button type="button" class="btn btn-link btn-floating mx-1">
        <i class="bi bi-twitter"></i>
      </button>
      <button type="button" class="btn btn-link btn-floating mx-1">
        <i class="bi bi-github"></i>
      </button>
    </div>
    </form>
    </div>
    <div class="col"></div>
    </div></div>
    </body>
    <script>
    async function login(form){
        let formData = new FormData(form);
        let formDataObject = Object.fromEntries(formData.entries());
        let formDataJsonString = JSON.stringify(formDataObject);
        console.log(formDataJsonString);
        let fetchOptions = {
            method: "POST",
            headers: {
            "Content-Type": "application/json",
            Accept: "application/json",
            },
            body: formDataJsonString,
        };
        let res = await fetch('/api/v1/authorize', fetchOptions);
        if (!res.ok) {
            let error = await res.text();
            throw new Error(error);
        } else {
            res = await res.json()  
        }
        document.cookie = `unpatched_token=${res.access_token}; SameSite=Strict; Secure; max-age=max-age-in-seconds=31536000`;
        window.location.href = "/protected";
    }
    </script>
    </html>"##;
    (StatusCode::OK, Html(html))
}

// FIXME: Make this work

#[derive(Debug, Clone)]
pub struct AuthLayer {
    target: &'static str,
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, service: S) -> Self::Service {
        AuthService {
            target: self.target,
            service
        }
    }
}

impl AuthLayer {
    pub fn verify() -> Self {
        error!("loaded auth layer");
        AuthLayer { target: "auth" }
    }
}
// This service implements the Log behavior
#[derive(Clone)]
pub struct AuthService<S> {
    target: &'static str,
    service: S,
}

impl<S, Request> Service<Request> for AuthService<S>
where
    S: Service<Request>,
    Request: std::fmt::Debug
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        // Insert log statement here or other functionality
        error!("request = {:?}, target = {:?}", request, self.target);
        self.service.call(request)
    }
}