use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, query, sqlite::SqliteQueryResult, Row, Sqlite, SqlitePool};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Execution {
    pub id: String,
    pub request: String,
    pub response: String,
    pub host_id: String,
    pub script_id: String,
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
            .bind(self.id)
            .bind(self.request)
            .bind(self.response)
            .bind(self.host_id)
            .bind(self.script_id)
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
            id: s.get::<String, _>("id"),
            request: s.get::<String, _>("request"),
            response: s.get::<String, _>("response"),
            host_id: s.get::<String, _>("host_id"),
            script_id: s.get::<String, _>("script_id"),
            output: s.get::<String, _>("output"),
        };
        execution_vec.push(execution);
    }
    execution_vec
}

pub async fn update_text_field(
    id: String,
    column: &str,
    data: String,
    connection: PoolConnection<Sqlite>,
) -> SqliteQueryResult {
    crate::db::update_text_field(id, column, data, "executions", connection).await
}

pub async fn update_timestamp(
    id: String,
    column: &str,
    connection: PoolConnection<Sqlite>,
) -> SqliteQueryResult {
    crate::db::update_timestamp(id, column, "executions", connection).await
}
