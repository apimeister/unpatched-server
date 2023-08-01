use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, query, sqlite::SqliteQueryResult, Row, Sqlite, SqlitePool};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Host {
    pub id: String,
    pub alias: String,
    pub attributes: String,
}

impl Host {
    /// Insert Host into hosts table in SQLite database
    ///
    /// | Name | Type | Comment
    /// :--- | :--- | :---
    /// | id | TEXT | uuid
    /// | alias | TEXT | host alias (name)
    /// | attributes | TEXT | host labels
    #[allow(dead_code)]
    // FIXME: write test and remove dead_code
    pub async fn insert_into_db(self, mut connection: PoolConnection<Sqlite>) -> SqliteQueryResult {
        query(r#"INSERT INTO hosts( id, alias, attributes ) VALUES ( ?, ?, ? )"#)
            .bind(self.id)
            .bind(self.alias)
            .bind(self.attributes)
            .execute(&mut *connection)
            .await
            .unwrap()
    }
}

/// API to get all hosts
pub async fn get_hosts_api(State(pool): State<SqlitePool>) -> (StatusCode, Json<Vec<Host>>) {
    let mut conn = pool.acquire().await.unwrap();
    let hosts = match query("SELECT * FROM hosts").fetch_all(&mut *conn).await {
        Ok(d) => d,
        Err(_) => return (StatusCode::NOT_FOUND, Json(Vec::new())),
    };

    let mut host_vec: Vec<Host> = Vec::new();

    for s in hosts {
        let host = Host {
            id: s.get::<String, _>("id"),
            alias: s.get::<String, _>("alias"),
            attributes: s.get::<String, _>("attributes"),
        };
        host_vec.push(host);
    }

    (StatusCode::OK, Json(host_vec))
}
