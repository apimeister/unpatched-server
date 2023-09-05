use std::{collections::HashMap, str::FromStr};

use argon2::{
    password_hash::{rand_core::OsRng, Error, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::{
    pool::PoolConnection,
    query,
    sqlite::{SqliteQueryResult, SqliteRow},
    Row, Sqlite, SqlitePool,
};
use uuid::Uuid;

use crate::{
    db::{utc_from_str, utc_to_str},
    jwt::Claims,
};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct User {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub email: EmailAddress,
    #[serde(skip_serializing)]
    pub password: String,
    pub roles: Vec<String>,
    pub active: bool,
    #[serde(default = "Utc::now")]
    pub created: DateTime<Utc>,
}

impl User {
    /// Insert `User` into users table in SQLite database
    ///
    /// | Name | Type | Comment
    /// :--- | :--- | :---
    /// | id | TEXT | uuid
    /// | email | TEXT | Email Address
    /// | password | TEXT |
    /// | roles | TEXT |
    /// | active | NUMERIC |
    /// | created | TEXT | as rfc3339 string ("YYYY-MM-DDTHH:MM:SS.sssZ")
    pub async fn insert_into_db(
        self,
        mut connection: PoolConnection<Sqlite>,
    ) -> Result<SqliteQueryResult, sqlx::Error> {
        let q = r#"INSERT INTO users(id, email, password, roles, active, created) VALUES(?, ?, ?, ?, ?, ?)"#;
        query(q)
            .bind(self.id.to_string())
            .bind(self.email.to_string())
            .bind(self.password)
            .bind(serde_json::to_string(&self.roles).unwrap())
            .bind(self.active)
            .bind(utc_to_str(self.created))
            .execute(&mut *connection)
            .await
    }
    pub fn verify_password(&self, password: &[u8]) -> Result<(), Error> {
        let parsed_hash = PasswordHash::new(&self.password)?;
        Argon2::default().verify_password(password, &parsed_hash)
    }
}

/// Hash password for storage
pub fn hash_password(password: &[u8]) -> Result<String, Error> {
    let salt = SaltString::generate(&mut OsRng);
    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();
    // Hash password to PHC string ($argon2id$v=19$...)
    Ok(argon2.hash_password(password, &salt)?.to_string())
    // TODO: Add Pepper!
}

/// Convert `SqliteRow` in `User` struct
impl From<SqliteRow> for User {
    fn from(s: SqliteRow) -> Self {
        User {
            id: s.get::<String, _>("id").parse().unwrap(),
            email: EmailAddress::from_str(&s.get::<String, _>("email")).unwrap(),
            password: s.get::<String, _>("password"),
            roles: serde_json::from_str(&s.get::<String, _>("roles")).unwrap(),
            active: s.get::<bool, _>("active"),
            created: utc_from_str(&s.get::<String, _>("created")),
        }
    }
}

/// API to get all users
pub async fn get_users_api(_claims: Claims, State(pool): State<SqlitePool>) -> impl IntoResponse {
    let user_vec = get_users_from_db(None, pool.acquire().await.unwrap()).await;
    Json(user_vec)
}

/// API to get one user
pub async fn get_one_user_api(
    _claims: Claims,
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    let user_vec = get_users_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    Json(user_vec.first().cloned())
}

/// API to delete all users
pub async fn delete_users_api(
    _claims: Claims,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    delete_users_from_db(None, pool.acquire().await.unwrap()).await
}

/// API to delete one user
pub async fn delete_one_user_api(
    _claims: Claims,
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    delete_users_from_db(Some(&filter), pool.acquire().await.unwrap()).await
}

/// API to create a new user
pub async fn post_users_api(
    _claims: Claims,
    State(pool): State<SqlitePool>,
    Json(mut payload): Json<User>,
) -> Response {
    let id = payload.id.to_string();
    let Ok(hash) = hash_password(payload.password.as_bytes()) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    payload.password = hash;
    let Ok(res) = payload.insert_into_db(pool.acquire().await.unwrap()).await else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if res.rows_affected() == 1 {
        (StatusCode::CREATED, Json(id)).into_response()
    } else {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

/// API to update one user
pub async fn update_one_user_api(
    _claims: Claims,
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
    Json(payload): Json<HashMap<String, String>>,
) -> Response {
    if let Some((column, data)) = payload.into_iter().next() {
        let safe_data = if column == "password" {
            let Ok(hash) = hash_password(data.as_bytes()) else {
                return StatusCode::BAD_REQUEST.into_response();
            };
            hash
        } else {
            data
        };
        let up = update_text_field(id, &column, safe_data, pool.acquire().await.unwrap()).await;
        if up.rows_affected() == 1 {
            (StatusCode::OK, Json(id)).into_response()
        } else {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    } else {
        StatusCode::BAD_REQUEST.into_response()
    }
}

pub async fn get_users_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> Vec<User> {
    let stmt = if let Some(f) = filter {
        format!("SELECT * FROM users WHERE {f}")
    } else {
        "SELECT * FROM users".into()
    };
    let users = match query(&stmt).fetch_all(&mut *connection).await {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    users.into_iter().map(|s| s.into()).collect()
}

pub async fn delete_users_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> StatusCode {
    let stmt = if let Some(f) = filter {
        format!("DELETE FROM users WHERE {f}")
    } else {
        "DELETE FROM users".into()
    };
    let res = query(&stmt).execute(&mut *connection).await;
    if res.is_err() {
        StatusCode::FORBIDDEN
    } else {
        StatusCode::OK
    }
}

pub async fn update_text_field(
    id: Uuid,
    column: &str,
    data: String,
    connection: PoolConnection<Sqlite>,
) -> SqliteQueryResult {
    crate::db::update_text_field(id, column, data, "users", connection).await
}

pub async fn count_rows(connection: PoolConnection<Sqlite>) -> Result<i64, sqlx::Error> {
    crate::db::count_rows("users", connection).await
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::db::{create_database, init_database};
    use tracing_subscriber::{
        fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
    };

    #[tokio::test]
    async fn test_users() {
        registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()))
            .with(fmt::layer())
            .try_init()
            .unwrap_or(());

        let pool = create_database("sqlite::memory:").await.unwrap();

        init_database(&pool, None).await.unwrap();

        // init new testuser
        let password = b"test123";
        let hashed_pw = hash_password(password).unwrap();
        let new_user = User {
            id: Uuid::new_v4(),
            email: EmailAddress::from_str("test@test.int").unwrap(),
            password: hashed_pw,
            roles: vec!["test".into()],
            active: true,
            created: Utc::now(),
        };
        let _i1 = new_user.insert_into_db(pool.acquire().await.unwrap()).await;
        assert_eq!(
            count_rows(pool.acquire().await.unwrap()).await.unwrap_or(0),
            1
        );

        // get all users
        let users = get_users_from_db(None, pool.acquire().await.unwrap()).await;
        assert_eq!(users.len(), 1);

        // get testuser and verify pw
        let users =
            get_users_from_db(Some("email='test@test.int'"), pool.acquire().await.unwrap()).await;
        let user = users.first().unwrap();
        assert!(user.verify_password(password).is_ok());
    }

    #[tokio::test]
    async fn test_apis() {
        registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()))
            .with(fmt::layer())
            .try_init()
            .unwrap_or(());

        let pool = create_database("sqlite::memory:").await.unwrap();
        let claims: Claims = Claims::default();
        init_database(&pool, None).await.unwrap();

        let new_user = User {
            id: Uuid::new_v4(),
            email: EmailAddress::from_str("test@test.int").unwrap(),
            password: hash_password(b"test123").unwrap(),
            roles: vec!["test".into()],
            active: true,
            created: Utc::now(),
        };

        let api_post = post_users_api(
            claims.clone(),
            axum::extract::State(pool.clone()),
            Json(new_user.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_post.status(), axum::http::StatusCode::CREATED);

        let mut api_update = HashMap::new();
        api_update.insert("created".to_string(), utc_to_str(Utc::now()));

        let api_update = update_one_user_api(
            claims.clone(),
            axum::extract::Path(new_user.id),
            axum::extract::State(pool.clone()),
            Json(api_update),
        )
        .await
        .into_response();
        assert_eq!(api_update.status(), axum::http::StatusCode::OK);

        let api_get_all = get_users_api(claims.clone(), axum::extract::State(pool.clone()))
            .await
            .into_response();
        assert_eq!(api_get_all.status(), axum::http::StatusCode::OK);

        let api_get_one = get_one_user_api(
            claims.clone(),
            axum::extract::Path(new_user.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_get_one.status(), axum::http::StatusCode::OK);

        let api_del_one = delete_one_user_api(
            claims.clone(),
            axum::extract::Path(new_user.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_del_one.status(), axum::http::StatusCode::OK);

        let api_del_all = delete_users_api(claims.clone(), axum::extract::State(pool.clone()))
            .await
            .into_response();
        assert_eq!(api_del_all.status(), axum::http::StatusCode::OK);
    }
}
