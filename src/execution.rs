use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, query, sqlite::SqliteQueryResult, Row, Sqlite, SqlitePool};
use tracing::debug;
use uuid::Uuid;

use crate::db;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Execution {
    pub id: Option<Uuid>,
    pub request: String,
    pub response: Option<String>,
    pub host_id: Uuid,
    pub script_id: Uuid,
    pub output: Option<String>,
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
    /// | output | TEXT | <-- implemented by another call, always created as NULL
    pub async fn insert_into_db(self, mut connection: PoolConnection<Sqlite>) -> SqliteQueryResult {
        query(r#"INSERT INTO executions( id, request, host_id, script_id ) VALUES ( ?, ?, ?, ? )"#)
            .bind(serde_json::to_string(&self.id).unwrap())
            .bind(self.request)
            .bind(serde_json::to_string(&self.host_id).unwrap())
            .bind(serde_json::to_string(&self.script_id).unwrap())
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
    Json(mut payload): Json<Execution>,
) -> StatusCode {
    debug!("{:?}", payload);
    payload.id = Some(crate::new_id());
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
            id: serde_json::from_str(&s.get::<String, _>("id")).unwrap(),
            request: s.get::<String, _>("request"),
            response: db::get_option(&s, "response"),
            host_id: serde_json::from_str(&s.get::<String, _>("host_id")).unwrap(),
            script_id: serde_json::from_str(&s.get::<String, _>("script_id")).unwrap(),
            output: db::get_option(&s, "output"),
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
