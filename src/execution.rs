use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, query, sqlite::SqliteQueryResult, Row, Sqlite, SqlitePool};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::db;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Execution {
    #[serde(default = "db::new_id")]
    pub id: Uuid,
    pub request: String,
    #[serde(default = "String::new")]
    pub response: String,
    pub host_id: Uuid,
    pub script_id: Uuid,
    #[serde(default = "db::nil_id")]
    pub sched_id: Uuid,
    #[serde(default = "String::new")]
    pub output: String,
}

impl Execution {
    /// Insert execution into Executions table
    ///
    /// | Name | Type | Comment
    /// :--- | :--- | :---
    /// | id | TEXT | uuid
    /// | request | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS.SSS")
    /// | response | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS.SSS") <-- implemented by another call, always created as NULL
    /// | host_id | TEXT | uuid
    /// | script_id | TEXT | uuid
    /// | sched_id | TEXT | uuid
    /// | output | TEXT | <-- implemented by another call, always created as NULL
    pub async fn insert_into_db(self, mut connection: PoolConnection<Sqlite>) -> SqliteQueryResult {
        query(r#"INSERT INTO executions( id, request, host_id, script_id, sched_id ) VALUES ( ?, ?, ?, ?, ? )"#)
            .bind(self.id.to_string())
            .bind(self.request)
            .bind(self.host_id.to_string())
            .bind(self.script_id.to_string())
            .bind(self.sched_id.to_string())
            .execute(&mut *connection)
            .await
            .unwrap()
    }
}

/// API to get all executions
pub async fn get_executions_api(
    State(pool): State<SqlitePool>,
) -> (StatusCode, Json<Vec<Execution>>) {
    let execution_vec = get_executions_from_db(None, pool.acquire().await.unwrap()).await;
    if execution_vec.is_empty() {
        (StatusCode::NOT_FOUND, Json(execution_vec))
    } else {
        (StatusCode::OK, Json(execution_vec))
    }
}

/// API to delete an execution
pub async fn delete_executions_api(State(pool): State<SqlitePool>) -> StatusCode {
    let executions = query("DELETE FROM executions")
        .execute(&mut *pool.acquire().await.unwrap())
        .await;
    if executions.is_err() {
        StatusCode::FORBIDDEN
    } else {
        StatusCode::OK
    }
}

/// API to create a new execution
pub async fn post_executions_api(
    State(pool): State<SqlitePool>,
    Json(payload): Json<Execution>,
) -> StatusCode {
    warn!("{:?}", payload);
    let res = payload.insert_into_db(pool.acquire().await.unwrap()).await;
    if res.rows_affected() == 1 {
        StatusCode::CREATED
    } else {
        StatusCode::BAD_REQUEST
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

    let mut execution_vec: Vec<Execution> = Vec::new();

    for s in executions {
        let execution = Execution {
            id: s.get::<String, _>("id").parse().unwrap(),
            request: s.get::<String, _>("request"),
            response: s.get::<String, _>("response"),
            host_id: s.get::<String, _>("host_id").parse().unwrap(),
            script_id: s.get::<String, _>("script_id").parse().unwrap(),
            sched_id: s.get::<String, _>("sched_id").parse().unwrap(),
            output: s.get::<String, _>("output"),
        };
        execution_vec.push(execution);
    }
    execution_vec
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
