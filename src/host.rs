use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, query, sqlite::SqliteQueryResult, Row, Sqlite, SqlitePool};
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Host {
    pub id: Uuid,
    pub alias: String,
    pub attributes: Vec<String>,
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
    pub async fn insert_into_db(self, mut connection: PoolConnection<Sqlite>) -> SqliteQueryResult {
        let q = r#"INSERT INTO hosts(alias, attributes, ip, last_pong, id)
        VALUES(?, ?, ?, datetime(), ?)
        ON CONFLICT(id) DO UPDATE SET
        alias = ?, attributes = ?, ip = ?, last_pong = datetime()
        WHERE id = ?"#;
        match query(q)
            .bind(self.alias)
            .bind(serde_json::to_string(&self.attributes).unwrap())
            .bind(self.ip)
            .bind(&self.id.to_string())
            // .bind(self.last_pong) <- generated by insert
            .execute(&mut *connection)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                if let Some(er) = e.as_database_error() {
                    if er.is_unique_violation() {
                        debug!("Host already known");
                    } else {
                        error!(
                            "Inserting new host into host table failed. Reason: \n{}",
                            er.message()
                        );
                    };
                }
                SqliteQueryResult::default()
            }
        }
    }
}

/// API to get all hosts
pub async fn get_hosts_api(State(pool): State<SqlitePool>) -> (StatusCode, Json<Vec<Host>>) {
    let host_vec = get_hosts_from_db(None, pool.acquire().await.unwrap()).await;
    if host_vec.is_empty() {
        (StatusCode::NOT_FOUND, Json(host_vec))
    } else {
        (StatusCode::OK, Json(host_vec))
    }
}

pub async fn get_hosts_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> Vec<Host> {
    let stmt = if let Some(f) = filter {
        format!("SELECT * FROM hosts WHERE {f}")
    } else {
        "SELECT * FROM hosts".into()
    };
    let hosts = match query(&stmt).fetch_all(&mut *connection).await {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let mut host_vec: Vec<Host> = Vec::new();

    for s in hosts {
        let host = Host {
            id: s.get::<String, _>("id").parse().unwrap(),
            alias: s.get::<String, _>("alias"),
            attributes: serde_json::from_str(&s.get::<String, _>("attributes")).unwrap(),
            ip: s.get::<String, _>("ip"),
            last_pong: s.get::<String, _>("last_pong"),
        };
        host_vec.push(host);
    }
    host_vec
}

// pub async fn update_text_field(
//     id: String,
//     column: &str,
//     data: String,
//     connection: PoolConnection<Sqlite>,
// ) -> SqliteQueryResult {
//     crate::db::update_text_field(id, column, data, "hosts", connection).await
// }

pub async fn update_timestamp(
    id: Uuid,
    column: &str,
    connection: PoolConnection<Sqlite>,
) -> SqliteQueryResult {
    crate::db::update_timestamp(id, column, "hosts", connection).await
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
