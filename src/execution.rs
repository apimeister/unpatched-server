use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, query, sqlite::SqliteQueryResult, Row, Sqlite, SqlitePool};
use uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Execution {
    pub id: Uuid,
    pub request: String,
    pub response: String,
    pub host_id: Uuid,
    pub script_id: Uuid,
    pub output: String,
}

impl Execution {
    /// Insert execution into Executions table
    ///
    /// | Name | Type | Comment
    /// :--- | :--- | :---
    /// | id | TEXT | uuid
    /// | request | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS.SSS")
    /// | response | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS.SSS")
    /// | host_id | TEXT | uuid
    /// | script_id | TEXT | uuid
    /// | output | TEXT |
    pub async fn insert_into_db(self, mut connection: PoolConnection<Sqlite>) -> SqliteQueryResult {
        query(r#"INSERT INTO executions( id, request, response, host_id, script_id, output ) VALUES ( ?, ?, ?, ?, ?, ? )"#)
            .bind(serde_json::to_string(&self.id).unwrap())
            .bind(self.request)
            .bind(self.response)
            .bind(serde_json::to_string(&self.host_id).unwrap())
            .bind(serde_json::to_string(&self.script_id).unwrap())
            .bind(self.output)
            .execute(&mut *connection)
            .await
            .unwrap()
    }
}

/// API to get all scripts
pub async fn get_executions_api(
    State(pool): State<SqlitePool>,
) -> (StatusCode, Json<Vec<Execution>>) {
    let execution_vec = get_executions_from_db(pool.acquire().await.unwrap()).await;
    if execution_vec.is_empty() {
        (StatusCode::NOT_FOUND, Json(execution_vec))
    } else {
        (StatusCode::OK, Json(execution_vec))
    }
}

/// API to get all scripts
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

pub async fn get_executions_from_db(mut connection: PoolConnection<Sqlite>) -> Vec<Execution> {
    let executions = match query("SELECT * FROM executions")
        .fetch_all(&mut *connection)
        .await
    {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let mut execution_vec: Vec<Execution> = Vec::new();

    for s in executions {
        let execution = Execution {
            id: serde_json::from_str(&s.get::<String, _>("id")).unwrap(),
            request: s.get::<String, _>("request"),
            response: s.get::<String, _>("response"),
            host_id: serde_json::from_str(&s.get::<String, _>("host_id")).unwrap(),
            script_id: serde_json::from_str(&s.get::<String, _>("script_id")).unwrap(),
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
