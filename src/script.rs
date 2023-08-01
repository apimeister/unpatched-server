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
    pub script_content: String,
}

impl Script {
    //    pub fn new_with_id() -> Self {
    //         Script {
    //             id: new_id(),
    //             ..Default::default()
    //         }
    //     }

    /// Insert Script into scripts table
    ///
    /// | Name | Type | Comment
    /// :--- | :--- | :---
    /// | id | TEXT | uuid
    /// | name | TEXT |
    /// | version | TEXT |
    /// | output_regex | TEXT | regex for result parsing
    /// | labels | TEXT | script labels
    /// | script_content | TEXT | original script
    pub async fn insert_into_db(self, mut connection: PoolConnection<Sqlite>) -> SqliteQueryResult {
        query(r#"INSERT INTO scripts( id, name, version, output_regex, labels, script_content ) VALUES ( ?, ?, ?, ?, ?, ? )"#)
        .bind(self.id)
        .bind(self.name)
        .bind(self.version)
        .bind(self.output_regex)
        .bind(self.labels)
        .bind(self.script_content)
        .execute(&mut *connection).await.unwrap()
    }
}

/// API to get all scripts
pub async fn get_scripts_api(State(pool): State<SqlitePool>) -> (StatusCode, Json<Vec<Script>>) {
    let mut conn = pool.acquire().await.unwrap();
    let scripts = match query("SELECT * FROM scripts").fetch_all(&mut *conn).await {
        Ok(d) => d,
        Err(_) => return (StatusCode::NOT_FOUND, Json(Vec::new())),
    };

    let mut script_vec: Vec<Script> = Vec::new();

    for s in scripts {
        let script = Script {
            id: s.get::<String, _>("id"),
            name: s.get::<String, _>("name"),
            version: s.get::<String, _>("version"),
            output_regex: s.get::<String, _>("output_regex"),
            labels: s.get::<String, _>("labels"),
            script_content: s.get::<String, _>("script_content"),
        };
        script_vec.push(script);
    }

    (StatusCode::OK, Json(script_vec))
}
