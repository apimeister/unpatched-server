use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{
    pool::PoolConnection,
    query,
    sqlite::{SqliteQueryResult, SqliteRow},
    Row, Sqlite, SqlitePool,
};
use tracing::debug;
use uuid::Uuid;

use crate::db::{self, get_option, utc_from_str, utc_to_str};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Execution {
    #[serde(default = "db::new_id")]
    pub id: Uuid,
    pub request: DateTime<Utc>,
    pub response: Option<DateTime<Utc>>,
    pub host_id: Uuid,
    pub script_id: Uuid,
    #[serde(default = "db::nil_id")]
    pub sched_id: Uuid,
    #[serde(default = "Utc::now")]
    pub created: DateTime<Utc>,
    #[serde(default = "String::new")]
    pub output: String,
}

impl Execution {
    /// Insert execution into Executions table
    ///
    /// | Name | Type | Comment
    /// :--- | :--- | :---
    /// | id | TEXT | uuid
    /// | request | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS")
    /// | response | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS") <-- implemented by another call, always created as NULL
    /// | host_id | TEXT | uuid
    /// | script_id | TEXT | uuid
    /// | sched_id | TEXT | uuid
    /// | created | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS") <-- autocreated
    /// | output | TEXT | <-- implemented by another call, always created as NULL
    pub async fn insert_into_db(self, mut connection: PoolConnection<Sqlite>) -> SqliteQueryResult {
        query(r#"INSERT INTO executions( id, request, host_id, script_id, sched_id, created ) VALUES ( ?, ?, ?, ?, ?, datetime() )"#)
            .bind(self.id.to_string())
            .bind(utc_to_str(self.request))
            .bind(self.host_id.to_string())
            .bind(self.script_id.to_string())
            .bind(self.sched_id.to_string())
            .execute(&mut *connection)
            .await
            .unwrap()
    }
}

impl From<SqliteRow> for Execution {
    fn from(s: SqliteRow) -> Self {
        let res = get_option(&s, "response");
        Execution {
            id: s.get::<String, _>("id").parse().unwrap(),
            request: utc_from_str(&s.get::<String, _>("request")),
            response: res.map(|r| utc_from_str(&r)),
            host_id: s.get::<String, _>("host_id").parse().unwrap(),
            script_id: s.get::<String, _>("script_id").parse().unwrap(),
            sched_id: s.get::<String, _>("sched_id").parse().unwrap(),
            created: utc_from_str(&s.get::<String, _>("created")),
            output: s.get::<String, _>("output"),
        }
    }
}

/// API to get all executions
pub async fn get_executions_api(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let execution_vec = get_executions_from_db(None, pool.acquire().await.unwrap()).await;
    Json(execution_vec)
}

/// API to get one execution
pub async fn get_one_execution_api(
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    let execution_vec = get_executions_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    Json(execution_vec)
}

/// API to delete all executions
pub async fn delete_executions_api(State(pool): State<SqlitePool>) -> impl IntoResponse {
    delete_executions_from_db(None, pool.acquire().await.unwrap()).await
}

/// API to delete one execution
pub async fn delete_one_execution_api(
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    delete_executions_from_db(Some(&filter), pool.acquire().await.unwrap()).await
}

/// API to create a new execution
pub async fn post_executions_api(
    State(pool): State<SqlitePool>,
    Json(payload): Json<Execution>,
) -> impl IntoResponse {
    debug!("{:?}", payload);
    let id = payload.id.to_string();
    let res = payload.insert_into_db(pool.acquire().await.unwrap()).await;
    if res.rows_affected() == 1 {
        (StatusCode::CREATED, Json(id))
    } else {
        (StatusCode::BAD_REQUEST, Json("".into()))
    }
}

pub async fn get_executions_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> Vec<Execution> {
    let stmt = if let Some(f) = filter {
        format!("SELECT * FROM executions WHERE {f}")
    } else {
        "SELECT * FROM executions".into()
    };
    let executions = match query(&stmt).fetch_all(&mut *connection).await {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    executions.into_iter().map(|s| s.into()).collect()
}

pub async fn delete_executions_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> StatusCode {
    let stmt = if let Some(f) = filter {
        format!("DELETE FROM executions WHERE {f}")
    } else {
        "DELETE FROM executions".into()
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
    crate::db::update_text_field(id, column, data, "executions", connection).await
}

pub async fn update_timestamp(
    id: Uuid,
    column: &str,
    connection: PoolConnection<Sqlite>,
) -> SqliteQueryResult {
    crate::db::update_timestamp(id, column, "executions", connection).await
}
