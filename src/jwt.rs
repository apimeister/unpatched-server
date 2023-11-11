use axum::{
    async_trait,
    extract::{ConnectInfo, FromRequestParts, Path, State, TypedHeader},
    headers::{authorization::Bearer, Authorization},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json, RequestPartsExt,
};
use chrono::{DateTime, Days, Duration, Utc};
use email_address::EmailAddress;
use headers::Cookie;

use hyper::header::SET_COOKIE;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{
    pool::PoolConnection,
    query,
    sqlite::{SqliteQueryResult, SqliteRow},
    Row, Sqlite, SqlitePool,
};
use std::{
    fmt::{Debug, Display},
    net::{IpAddr, SocketAddr},
    str::FromStr,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    db::{utc_from_str, utc_to_str},
    user::get_users_from_db,
    JWT_SECRET,
};

const BLACKLIST_AFTER: u8 = 5;
static BLACKLIST_TTL: Lazy<Duration> = Lazy::new(|| Duration::minutes(5));

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct BlacklistItem {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub ip: IpAddr,
    pub tries: u8,
    #[serde(default = "Utc::now")]
    pub created: DateTime<Utc>,
    pub blocked: Option<DateTime<Utc>>,
    pub blocked_until: Option<DateTime<Utc>>,
}

impl Default for BlacklistItem {
    fn default() -> Self {
        BlacklistItem {
            id: Uuid::new_v4(),
            ip: "127.0.0.1".parse().unwrap(),
            tries: 0,
            created: Utc::now(),
            blocked: None,
            blocked_until: None,
        }
    }
}

impl BlacklistItem {
    /// Insert `BlacklistItem` into users table in SQLite database
    ///
    /// | Name | Type | Comment
    /// :--- | :--- | :---
    /// | id | TEXT | uuid
    /// | ip | TEXT | ip address
    /// | tries | NUMERIC | login tries
    /// | created | TEXT | as rfc3339 string ("YYYY-MM-DDTHH:MM:SS.sssZ")
    /// | blocked | TEXT | Optional - as rfc3339 string ("YYYY-MM-DDTHH:MM:SS.sssZ")
    /// | blocked_until | TEXT | Optional - as rfc3339 string ("YYYY-MM-DDTHH:MM:SS.sssZ")
    pub async fn insert_into_db(
        self,
        mut connection: PoolConnection<Sqlite>,
    ) -> Result<SqliteQueryResult, sqlx::Error> {
        let q = r#"REPLACE INTO blacklist( id, ip, tries, created, blocked, blocked_until ) VALUES ( ?, ?, ?, ?, ?, ? )"#;
        query(q)
            .bind(self.id.to_string())
            .bind(self.ip.to_string())
            .bind(self.tries)
            .bind(utc_to_str(self.created))
            .bind(self.blocked.map(utc_to_str))
            .bind(self.blocked_until.map(utc_to_str))
            .execute(&mut *connection)
            .await
    }
}

/// Convert `SqliteRow` in `BlacklistItem` struct
impl From<SqliteRow> for BlacklistItem {
    fn from(s: SqliteRow) -> Self {
        BlacklistItem {
            id: s.get::<String, _>("id").parse().unwrap(),
            ip: s.get::<String, _>("ip").parse().unwrap(),
            tries: s.get::<u8, _>("tries"),
            created: utc_from_str(&s.get::<String, _>("created")),
            blocked: s
                .get::<Option<String>, _>("blocked")
                .as_deref()
                .map(utc_from_str),
            blocked_until: s
                .get::<Option<String>, _>("blocked_until")
                .as_deref()
                .map(utc_from_str),
        }
    }
}

pub static KEYS: Lazy<Keys> = Lazy::new(|| {
    let sec = std::fs::read(JWT_SECRET).unwrap_or_else(|_| {
        info!("{JWT_SECRET} not found, generating new secret");
        let secret: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();
        if let Err(e) = std::fs::write(JWT_SECRET, &secret) {
            warn!("{JWT_SECRET} could not be saved on filesystem, server will get a new {JWT_SECRET} each restart. Error:\n{e}");
        };
        secret.into()
    });
    Keys::new(&sec)
});

//TODO: Remove or keep as test endpoint
pub async fn protected(claims: Claims) -> Result<String, AuthError> {
    // Send the protected data to the user
    Ok(format!(
        "Welcome to the protected area :)\nYour data:\n{}",
        claims
    ))
}

pub async fn logout(_claims: Claims) -> Response {
    let mut res = StatusCode::OK.into_response();
    let cookie = "unpatched_token=''; SameSite=Strict; Secure; Path=/; HttpOnly; Max-Age=0";
    res.headers_mut()
        .insert(SET_COOKIE, cookie.parse().unwrap());
    res
}

pub async fn login_status(_claims: Claims) -> impl IntoResponse {
    StatusCode::OK
}

pub async fn api_authorize_user(
    State(pool): State<SqlitePool>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<AuthPayload>,
) -> Result<Response, AuthError> {
    // blacklist
    let ip = addr.ip();
    let filter = format!("ip = '{ip}'");
    let blacklisted =
        get_blacklistitems_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    let mut bl_item = match blacklisted.first().cloned() {
        Some(b) => b,
        None => BlacklistItem {
            ip,
            ..Default::default()
        },
    };

    // Check if blacklisted and
    if let Some(block) = bl_item.blocked_until {
        if block > Utc::now() {
            error!("Login for {addr} failed multiple times, blacklisted until {block}");
            return Err(AuthError::WrongCredentials);
        } else {
            let filter = format!("id='{}'", bl_item.id);
            let _ =
                delete_blacklistitems_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
            bl_item = BlacklistItem {
                ip,
                ..Default::default()
            }
        }
    }

    // Check if the user sent the credentials
    if payload.client_id.is_empty() || payload.client_secret.is_empty() {
        return Err(AuthError::MissingCredentials);
    }
    let validate_email =
        EmailAddress::from_str(&payload.client_id).map_err(|_| AuthError::InvalidEmail)?;
    info!("Starting login for {validate_email}");
    // Here you can check the user credentials from a database
    let filter = format!("email='{}'", payload.client_id);
    let users = get_users_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    let Some(user) = users.first() else {
        error!("Login for {validate_email} failed. Wrong credentials");
        bl_item.tries += 1;
        if bl_item.tries >= BLACKLIST_AFTER {
            bl_item.blocked = Some(Utc::now());
            bl_item.blocked_until = Some(Utc::now().checked_add_signed(*BLACKLIST_TTL).unwrap());
        }
        let res = bl_item.insert_into_db(pool.acquire().await.unwrap()).await;
        debug!("{res:?}");
        return Err(AuthError::WrongCredentials);
    };
    if user
        .verify_password(payload.client_secret.as_bytes())
        .is_err()
    {
        error!("Login for {validate_email} failed. Wrong credentials");
        bl_item.tries += 1;
        if bl_item.tries >= BLACKLIST_AFTER {
            bl_item.blocked = Some(Utc::now());
            bl_item.blocked_until = Some(Utc::now().checked_add_signed(*BLACKLIST_TTL).unwrap());
        }
        let res = bl_item.insert_into_db(pool.acquire().await.unwrap()).await;
        debug!("{res:?}");
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
    let token = encode(&Header::default(), &claims, &KEYS.encoding)
        .map_err(|_| AuthError::TokenCreation)?;
    // Send the authorized token (as payload for apis and cookie header for webpage)
    let body = Json(AuthBody::new(token.clone()));
    let mut res = (StatusCode::OK, body).into_response();
    let cookie = format!(
        "unpatched_token={token}; SameSite=Strict; Secure; Path=/; HttpOnly; Max-Age=31536000"
    );
    res.headers_mut().insert(
        SET_COOKIE,
        cookie.parse().map_err(|_| AuthError::TokenCreation)?,
    );
    info!("Login for {validate_email} successful");
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
                .map_err(|_| {
                    eprintln!("bearer token not found");
                    AuthError::InvalidToken
                })?;
            bearer.token().to_string()
        } else if parts.headers.contains_key(axum::http::header::COOKIE) {
            // Extract the token from cookie
            let TypedHeader(cookies) =
                parts.extract::<TypedHeader<Cookie>>().await.map_err(|_| {
                    eprintln!("no cookie found in header");
                    AuthError::InvalidToken
                })?;
            let Some(coo) = cookies.get("unpatched_token") else {
                eprintln!("cookie name not found in header");
                return Err(AuthError::InvalidToken);
            };
            coo.to_string()
        } else {
            eprintln!("no auth header or cookie found");
            return Err(AuthError::InvalidToken);
        };
        // Decode the user data
        let mut validation = Validation::default();
        validation.set_audience(&["unpatched-server-users"]);
        let token_data = match decode::<Claims>(&token, &KEYS.decoding, &validation) {
            Ok(a) => a,
            Err(e) => {
                error!("token decode failed: {e:?}");
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
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
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
            aud: "unpatched-server-users".to_string(),
            exp: Default::default(),
            nbf: Default::default(),
            iat: Default::default(),
            jti: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthBody {
    pub access_token: String,
    pub token_type: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct AuthPayload {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, PartialEq)]
pub enum AuthError {
    WrongCredentials,
    MissingCredentials,
    TokenCreation,
    InvalidToken,
    InvalidEmail,
}

pub async fn get_blacklistitems_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> Vec<BlacklistItem> {
    let stmt = if let Some(f) = filter {
        format!("SELECT * FROM blacklist WHERE {f}")
    } else {
        "SELECT * FROM blacklist".into()
    };
    let blacklist_items = match query(&stmt).fetch_all(&mut *connection).await {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    blacklist_items.into_iter().map(|s| s.into()).collect()
}

pub async fn delete_blacklistitems_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> StatusCode {
    let stmt = if let Some(f) = filter {
        format!("DELETE FROM blacklist WHERE {f}")
    } else {
        "DELETE FROM blacklist".into()
    };
    let res = query(&stmt).execute(&mut *connection).await;
    if res.is_err() {
        StatusCode::FORBIDDEN
    } else {
        StatusCode::OK
    }
}

/// API to delete one ip from blacklist
pub async fn remove_ip_from_blacklist_api(
    _claims: Claims,
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    delete_blacklistitems_from_db(Some(&filter), pool.acquire().await.unwrap()).await
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{
        db::{create_database, init_database},
        user::{hash_password, User},
    };
    use hyper::{
        header::{AUTHORIZATION, COOKIE},
        Request,
    };
    use tracing_subscriber::{
        fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
    };

    #[tokio::test]
    async fn test_apis() {
        registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()))
            .with(fmt::layer())
            .try_init()
            .unwrap_or(());

        // prepare DB with user
        let pool = create_database("sqlite::memory:").await.unwrap();
        init_database(&pool, None).await.unwrap();
        let new_user = User {
            id: Uuid::new_v4(),
            email: EmailAddress::from_str("test@test.int").unwrap(),
            password: hash_password(b"test123").unwrap(),
            roles: vec!["test".into()],
            active: true,
            created: Utc::now(),
        };
        let _i1 = new_user.insert_into_db(pool.acquire().await.unwrap()).await;
        let payload = AuthPayload {
            client_id: "test@test.int".into(),
            client_secret: "test123".into(),
        };
        let auth = api_authorize_user(
            axum::extract::State(pool.clone()),
            axum::extract::ConnectInfo("127.0.0.1:3000".parse().unwrap()),
            Json(payload.clone()),
        )
        .await;
        assert_eq!(auth.unwrap().status(), axum::http::StatusCode::OK);
        let auth = api_authorize_user(
            axum::extract::State(pool.clone()),
            axum::extract::ConnectInfo("127.0.0.1:3000".parse().unwrap()),
            Json(AuthPayload::default()),
        )
        .await;
        assert_eq!(auth.as_ref().unwrap_err(), &AuthError::MissingCredentials);
        assert_eq!(
            auth.into_response().status(),
            axum::http::StatusCode::BAD_REQUEST
        );
        let auth = api_authorize_user(
            axum::extract::State(pool.clone()),
            axum::extract::ConnectInfo("127.0.0.1:3000".parse().unwrap()),
            Json(AuthPayload {
                client_id: "no@test.int".into(),
                client_secret: "no".into(),
            }),
        )
        .await;
        assert_eq!(auth.as_ref().unwrap_err(), &AuthError::WrongCredentials);
        assert_eq!(
            auth.into_response().status(),
            axum::http::StatusCode::UNAUTHORIZED
        );
        let auth = api_authorize_user(
            axum::extract::State(pool.clone()),
            axum::extract::ConnectInfo("127.0.0.1:3000".parse().unwrap()),
            Json(AuthPayload {
                client_id: "no-test.int".into(),
                client_secret: "no".into(),
            }),
        )
        .await;
        assert_eq!(auth.as_ref().unwrap_err(), &AuthError::InvalidEmail);
        assert_eq!(
            auth.into_response().status(),
            axum::http::StatusCode::NOT_ACCEPTABLE
        );

        // run into block!
        let _wrong_password = api_authorize_user(
            axum::extract::State(pool.clone()),
            axum::extract::ConnectInfo("127.0.0.1:3000".parse().unwrap()),
            Json(AuthPayload {
                client_id: "test@test.int".into(),
                client_secret: "no".into(),
            }),
        )
        .await;
        for _wrong_credentials in 0..3 {
            let _ = api_authorize_user(
                axum::extract::State(pool.clone()),
                axum::extract::ConnectInfo("127.0.0.1:3000".parse().unwrap()),
                Json(AuthPayload {
                    client_id: "no@test.int".into(),
                    client_secret: "no".into(),
                }),
            )
            .await;
        }
        let bl_items = get_blacklistitems_from_db(None, pool.acquire().await.unwrap()).await;
        assert_eq!(bl_items.len(), 1);
        let bl_item = bl_items.first().cloned().unwrap();
        assert_eq!(bl_item.tries, 5);
        assert!(bl_item.blocked.is_some());
        assert!(bl_item.blocked_until.is_some());

        // try login while blacklisted
        let claims: Claims = Claims::default();
        let blocked_auth = api_authorize_user(
            axum::extract::State(pool.clone()),
            axum::extract::ConnectInfo("127.0.0.1:3000".parse().unwrap()),
            Json(AuthPayload {
                client_id: "test@test.int".into(),
                client_secret: "test123".into(),
            }),
        )
        .await;
        info!("{blocked_auth:#?}");
        assert_eq!(
            blocked_auth.as_ref().unwrap_err(),
            &AuthError::WrongCredentials
        );
        assert_eq!(
            blocked_auth.into_response().status(),
            axum::http::StatusCode::UNAUTHORIZED
        );

        //unblock
        let rem = remove_ip_from_blacklist_api(
            claims,
            axum::extract::Path(bl_item.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(rem.status(), axum::http::StatusCode::OK);

        // JWT secret
        let _init_jwt_new_file = &KEYS;
        let _init_jwt_present = &KEYS;

        let good_auth = api_authorize_user(
            axum::extract::State(pool.clone()),
            axum::extract::ConnectInfo("127.0.0.1:3000".parse().unwrap()),
            Json(payload.clone()),
        )
        .await
        .unwrap();
        println!("{good_auth:?}");
        let cookies = good_auth
            .headers()
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap();
        let req = Request::builder()
            .uri("/")
            .header(COOKIE, cookies)
            .body(())
            .unwrap();
        let mut x = req.into_parts().0;
        println!("receiving parts: {x:?}");
        let claim = Claims::from_request_parts(&mut x, &axum::extract::State(())).await;
        println!("claim: {claim:?}");
        let claim = claim.unwrap();
        let prot = protected(claim.clone()).await;
        assert_eq!(
            prot.unwrap(),
            format!("Welcome to the protected area :)\nYour data:\n{claim}")
        );

        let bad_req = Request::builder()
            .uri("/")
            .header(COOKIE, "")
            .body(())
            .unwrap();
        let claim =
            Claims::from_request_parts(&mut bad_req.into_parts().0, &axum::extract::State(()))
                .await;
        assert_eq!(claim.as_ref().unwrap_err(), &AuthError::InvalidToken);

        let bytes = hyper::body::to_bytes(good_auth.into_body()).await.unwrap();
        let bearer: AuthBody = serde_json::from_slice(&bytes).unwrap();
        let req = Request::builder()
            .uri("/")
            .header(AUTHORIZATION, format!("Bearer {}", bearer.access_token))
            .body(())
            .unwrap();
        let claim = Claims::from_request_parts(&mut req.into_parts().0, &axum::extract::State(()))
            .await
            .unwrap();
        let prot = protected(claim.clone()).await;
        assert_eq!(
            prot.unwrap(),
            format!("Welcome to the protected area :)\nYour data:\n{claim}")
        );
    }
}
