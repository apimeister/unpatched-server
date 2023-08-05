use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, query, sqlite::SqliteQueryResult, Row, Sqlite, SqlitePool};
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
        .bind(serde_json::to_string(&self.id).unwrap())
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

    let mut script_vec: Vec<Script> = Vec::new();

    for s in scripts {
        let id = serde_json::from_str(&s.get::<String, _>("id")).unwrap();
        let script = Script {
            id,
            name: s.get::<String, _>("name"),
            version: s.get::<String, _>("version"),
            output_regex: s.get::<String, _>("output_regex"),
            labels: serde_json::from_str(&s.get::<String, _>("labels")).unwrap(),
            timeout: s.get::<String, _>("timeout"),
            script_content: s.get::<String, _>("script_content"),
        };
        script_vec.push(script);
    }
    script_vec
}
