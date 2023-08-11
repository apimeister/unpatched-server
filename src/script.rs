use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{
    pool::PoolConnection,
    query,
    sqlite::{SqliteQueryResult, SqliteRow},
    Row, Sqlite, SqlitePool,
};
use tracing::debug;
use uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Script {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub output_regex: String,
    pub labels: Vec<String>,
    pub timeout: String,
    pub script_content: String,
}

impl Script {
    /// Insert Script into scripts table
    ///
    /// | Name | Type | Comment
    /// :--- | :--- | :---
    /// | id | TEXT | uuid
    /// | name | TEXT |
    /// | version | TEXT |
    /// | output_regex | TEXT | regex for result parsing
    /// | labels | TEXT | script labels
    /// | timeout | TEXT | timeout (1s, 5m, 3h etc.)
    /// | script_content | TEXT | original script
    pub async fn insert_into_db(self, mut connection: PoolConnection<Sqlite>) -> SqliteQueryResult {
        query(r#"INSERT INTO scripts( id, name, version, output_regex, labels, timeout, script_content ) VALUES ( ?, ?, ?, ?, ?, ?, ? )"#)
        .bind(self.id.to_string())
        .bind(self.name)
        .bind(self.version)
        .bind(self.output_regex)
        .bind(serde_json::to_string(&self.labels).unwrap())
        .bind(self.timeout)
        .bind(self.script_content)
        .execute(&mut *connection).await.unwrap()
    }
    pub fn labels(&self) -> String {
        self.labels.join(",")
    }
}

impl From<SqliteRow> for Script {
    fn from(s: SqliteRow) -> Self {
        Script {
            id: s.get::<String, _>("id").parse().unwrap(),
            name: s.get::<String, _>("name"),
            version: s.get::<String, _>("version"),
            output_regex: s.get::<String, _>("output_regex"),
            labels: serde_json::from_str(&s.get::<String, _>("labels")).unwrap(),
            timeout: s.get::<String, _>("timeout"),
            script_content: s.get::<String, _>("script_content"),
        }
    }
}

pub async fn count_rows(connection: PoolConnection<Sqlite>) -> Result<i64, sqlx::Error> {
    crate::db::count_rows("scripts", connection).await
}

/// API to get all scripts
pub async fn get_scripts_api(State(pool): State<SqlitePool>) -> (StatusCode, Json<Vec<Script>>) {
    let script_vec = get_scripts_from_db(None, pool.acquire().await.unwrap()).await;
    if script_vec.is_empty() {
        (StatusCode::NOT_FOUND, Json(script_vec))
    } else {
        (StatusCode::OK, Json(script_vec))
    }
}

/// API to get one script
pub async fn get_one_script_api(
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> (StatusCode, Json<Script>) {
    let script = query("SELECT * FROM scripts WHERE id = ?")
        .bind(id.to_string())
        .fetch_one(&mut *pool.acquire().await.unwrap())
        .await;
    match script {
        Ok(ex) => (StatusCode::OK, Json(Script::from(ex))),
        Err(_) => (StatusCode::NOT_FOUND, Json(Script::default())),
    }
}

/// API to delete all scripts
pub async fn delete_scripts_api(State(pool): State<SqlitePool>) -> StatusCode {
    let scripts = query("DELETE FROM scripts")
        .execute(&mut *pool.acquire().await.unwrap())
        .await;
    if scripts.is_err() {
        StatusCode::FORBIDDEN
    } else {
        StatusCode::OK
    }
}

/// API to delete one script
pub async fn delete_one_script_api(
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> StatusCode {
    let script = query("DELETE FROM scripts WHERE id = ?")
        .bind(id.to_string())
        .execute(&mut *pool.acquire().await.unwrap())
        .await;
    if script.is_err() {
        StatusCode::FORBIDDEN
    } else {
        StatusCode::OK
    }
}

/// API to create a new script
pub async fn post_scripts_api(
    State(pool): State<SqlitePool>,
    Json(payload): Json<Script>,
) -> (StatusCode, Json<String>) {
    debug!("{:?}", payload);
    let id = payload.id.to_string();
    let res = payload.insert_into_db(pool.acquire().await.unwrap()).await;
    if res.rows_affected() == 1 {
        (StatusCode::CREATED, Json(id))
    } else {
        (StatusCode::BAD_REQUEST, Json("".into()))
    }
}

pub async fn get_scripts_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> Vec<Script> {
    let stmt = if let Some(f) = filter {
        format!("SELECT * FROM scripts WHERE {f}")
    } else {
        "SELECT * FROM scripts".into()
    };
    let scripts = match query(&stmt).fetch_all(&mut *connection).await {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    scripts.into_iter().map(|s| s.into()).collect()
}
