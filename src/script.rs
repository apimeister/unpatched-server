use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, query, sqlite::SqliteQueryResult, Row, Sqlite, SqlitePool};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Script {
    pub id: String,
    pub name: String,
    pub version: String,
    pub output_regex: String,
    pub labels: String,
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
        .bind(self.id)
        .bind(self.name)
        .bind(self.version)
        .bind(self.output_regex)
        .bind(self.labels)
        .bind(self.timeout)
        .bind(self.script_content)
        .execute(&mut *connection).await.unwrap()
    }
}

/// API to get all scripts
pub async fn get_scripts_api(State(pool): State<SqlitePool>) -> (StatusCode, Json<Vec<Script>>) {
    let script_vec = get_scripts_from_db(pool.acquire().await.unwrap()).await;
    if script_vec.is_empty() {
        (StatusCode::NOT_FOUND, Json(script_vec))
    } else {
        (StatusCode::OK, Json(script_vec))
    }
}

pub async fn get_scripts_from_db(mut connection: PoolConnection<Sqlite>) -> Vec<Script> {
    let scripts = match query("SELECT * FROM scripts")
        .fetch_all(&mut *connection)
        .await
    {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let mut script_vec: Vec<Script> = Vec::new();

    for s in scripts {
        let script = Script {
            id: s.get::<String, _>("id"),
            name: s.get::<String, _>("name"),
            version: s.get::<String, _>("version"),
            output_regex: s.get::<String, _>("output_regex"),
            labels: s.get::<String, _>("labels"),
            timeout: s.get::<String, _>("timeout"),
            script_content: s.get::<String, _>("script_content"),
        };
        script_vec.push(script);
    }
    script_vec
}

pub async fn get_script_id_by_name_from_db(
    name: String,
    mut connection: PoolConnection<Sqlite>,
) -> Option<String> {
    match query("SELECT id FROM scripts WHERE name = ?")
        .bind(name)
        .fetch_one(&mut *connection)
        .await
    {
        Ok(s) => Some(s.get::<String, _>("id")),
        Err(_) => None,
    }
}
