use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, query, sqlite::SqliteQueryResult, Row, Sqlite, SqlitePool};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Host {
    pub id: String,
    pub alias: String,
    pub attributes: String,
    pub ip: String,
    pub last_pong: String,
}

impl Host {
    /// Insert Host into hosts table in SQLite database
    ///
    /// | Name | Type | Comment
    /// :--- | :--- | :---
    /// | id | TEXT | uuid
    /// | alias | TEXT | host alias (name)
    /// | attributes | TEXT | host labels
    /// | ip | TEXT | host ip:port
    /// | last_pong | TEXT | last checkin from agent
    #[allow(dead_code)]
    // FIXME: write test and remove dead_code
    pub async fn insert_into_db(self, mut connection: PoolConnection<Sqlite>) -> SqliteQueryResult {
        query(r#"INSERT INTO hosts( id, alias, attributes, ip, last_pong ) VALUES ( ?, ?, ?, ?, datetime() )"#)
            .bind(self.id)
            .bind(self.alias)
            .bind(self.attributes)
            .bind(self.ip)
            // .bind(self.last_pong) <- generated by insert
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
            ip: s.get::<String, _>("ip"),
            last_pong: s.get::<String, _>("last_pong"),
        };
        host_vec.push(host);
    }

    (StatusCode::OK, Json(host_vec))
}

// async fn single_agent_api(
//     Path(id): Path<Uuid>,
//     State(pool): State<SqlitePool>,
// ) -> (StatusCode, Json<AgentData>) {
//     let mut conn = pool.acquire().await.unwrap();
//     let show_data = match query("SELECT * FROM data WHERE id = ?")
//         .bind(id.to_string())
//         .fetch_one(&mut *conn)
//         .await
//     {
//         Ok(d) => d,
//         Err(_) => return (StatusCode::NOT_FOUND, Json(AgentData::default())),
//     };
//     let single_agent = AgentData {
//         id: show_data.get::<String, _>("id"),
//         alias: show_data.get::<String, _>("name"),
//         uptime: show_data.get::<i64, _>("uptime"),
//         os_release: show_data.get::<String, _>("os_release"),
//         memory: serde_json::from_str(show_data.get::<String, _>("memory").as_str()).unwrap(),
//         units: serde_json::from_str(show_data.get::<String, _>("units").as_str()).unwrap(),
//     };
//     (StatusCode::OK, Json(single_agent))
// }