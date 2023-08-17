use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{
    pool::PoolConnection,
    query,
    sqlite::{SqliteQueryResult, SqliteRow},
    Row, Sqlite, SqlitePool,
};
use tracing::debug;
use uuid::Uuid;

use crate::db::utc_to_str;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Host {
    pub id: Uuid,
    pub alias: String,
    pub attributes: Vec<String>,
    pub ip: String,
    pub last_pong: String,
}

impl Host {
    /// Insert into or Replace `Host` in hosts table in SQLite database
    ///
    /// | Name | Type | Comment
    /// :--- | :--- | :---
    /// | id | TEXT | uuid
    /// | alias | TEXT | host alias (name)
    /// | attributes | TEXT | host labels
    /// | ip | TEXT | host ip:port
    /// | last_pong | TEXT | last checkin from agent
    pub async fn insert_into_db(self, mut connection: PoolConnection<Sqlite>) -> SqliteQueryResult {
        let q = r#"REPLACE INTO hosts(id, alias, attributes, ip, last_pong) VALUES(?, ?, ?, ?, ?)"#;
        query(q)
            .bind(&self.id.to_string())
            .bind(self.alias)
            .bind(serde_json::to_string(&self.attributes).unwrap())
            .bind(self.ip)
            .bind(utc_to_str(Utc::now()))
            .execute(&mut *connection)
            .await
            .unwrap()
    }
}

/// Convert `SqliteRow` in `Host` struct
impl From<SqliteRow> for Host {
    fn from(s: SqliteRow) -> Self {
        Host {
            id: s.get::<String, _>("id").parse().unwrap(),
            alias: s.get::<String, _>("alias"),
            attributes: serde_json::from_str(&s.get::<String, _>("attributes")).unwrap(),
            ip: s.get::<String, _>("ip"),
            last_pong: s.get::<String, _>("last_pong"),
        }
    }
}

/// API to get all hosts
pub async fn get_hosts_api(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let host_vec = get_hosts_from_db(None, pool.acquire().await.unwrap()).await;
    Json(host_vec)
}

/// API to get one host
pub async fn get_one_host_api(
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    let host_vec = get_hosts_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    Json(host_vec.first().cloned())
}

/// API to delete all hosts
pub async fn delete_hosts_api(State(pool): State<SqlitePool>) -> impl IntoResponse {
    delete_hosts_from_db(None, pool.acquire().await.unwrap()).await
}

/// API to delete one host
pub async fn delete_one_host_api(
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    delete_hosts_from_db(Some(&filter), pool.acquire().await.unwrap()).await
}

/// API to create a new host
pub async fn post_hosts_api(
    State(pool): State<SqlitePool>,
    Json(payload): Json<Host>,
) -> impl IntoResponse {
    debug!("{:?}", payload);
    let id = payload.id.to_string();
    let res = payload.insert_into_db(pool.acquire().await.unwrap()).await;
    if res.rows_affected() == 1 {
        (StatusCode::CREATED, Json(id))
    } else {
        (StatusCode::BAD_REQUEST, Json("".into()))
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

    hosts.into_iter().map(|s| s.into()).collect()
}

pub async fn delete_hosts_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> StatusCode {
    let stmt = if let Some(f) = filter {
        format!("DELETE FROM hosts WHERE {f}")
    } else {
        "DELETE FROM hosts".into()
    };
    let res = query(&stmt).execute(&mut *connection).await;
    if res.is_err() {
        StatusCode::FORBIDDEN
    } else {
        StatusCode::OK
    }
}
#[allow(dead_code)]
// FIXME: make undead
pub async fn update_text_field(
    id: Uuid,
    column: &str,
    data: String,
    connection: PoolConnection<Sqlite>,
) -> SqliteQueryResult {
    crate::db::update_text_field(id, column, data, "hosts", connection).await
}

#[allow(dead_code)]
// FIXME: make undead
pub async fn count_rows(connection: PoolConnection<Sqlite>) -> Result<i64, sqlx::Error> {
    crate::db::count_rows("hosts", connection).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_database, init_database, new_id};
    use tracing_subscriber::{
        fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
    };

    #[tokio::test]
    async fn test_hosts() {
        registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()))
            .with(fmt::layer())
            .try_init()
            .unwrap_or(());

        let pool = create_database("sqlite::memory:").await.unwrap();

        init_database(&pool).await.unwrap();
        let hosts = get_hosts_from_db(None, pool.acquire().await.unwrap()).await;
        assert_eq!(hosts.len(), 0);

        let mut host = Host {
            id: new_id(),
            ..Default::default()
        };

        let _i1 = host
            .clone()
            .insert_into_db(pool.acquire().await.unwrap())
            .await;
        host.id = new_id();
        let _i2 = host
            .clone()
            .insert_into_db(pool.acquire().await.unwrap())
            .await;
        let _i3 = host
            .clone()
            .insert_into_db(pool.acquire().await.unwrap())
            .await;

        let hosts = count_rows(pool.acquire().await.unwrap()).await.unwrap();
        assert_eq!(hosts, 2);

        let err_hosts =
            get_hosts_from_db(Some("this-doesnt-work"), pool.acquire().await.unwrap()).await;
        assert_eq!(err_hosts.len(), 0);

        let _upd = update_text_field(
            host.id,
            "alias",
            "cargo-test".to_string(),
            pool.acquire().await.unwrap(),
        )
        .await;
        let hosts = get_hosts_from_db(
            Some(format!("id='{}'", host.id).as_str()),
            pool.acquire().await.unwrap(),
        )
        .await;
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].alias, "cargo-test");

        let single_del = delete_hosts_from_db(
            Some(format!("id='{}'", host.id).as_str()),
            pool.acquire().await.unwrap(),
        )
        .await;
        assert_eq!(single_del, axum::http::StatusCode::OK);
        let hosts = count_rows(pool.acquire().await.unwrap()).await.unwrap();
        assert_eq!(hosts, 1);

        let del_fail =
            delete_hosts_from_db(Some("this-doesnt-work"), pool.acquire().await.unwrap()).await;
        assert_eq!(del_fail, axum::http::StatusCode::FORBIDDEN);

        let del = delete_hosts_from_db(None, pool.acquire().await.unwrap()).await;
        assert_eq!(del, axum::http::StatusCode::OK);
        let hosts = count_rows(pool.acquire().await.unwrap()).await.unwrap();
        assert_eq!(hosts, 0);
    }

    #[tokio::test]
    async fn test_apis() {
        registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()))
            .with(fmt::layer())
            .try_init()
            .unwrap_or(());

        let pool = create_database("sqlite::memory:").await.unwrap();

        init_database(&pool).await.unwrap();
        let new_host = Host {
            id: new_id(),
            ..Default::default()
        };

        let api_post = post_hosts_api(axum::extract::State(pool.clone()), Json(new_host.clone()))
            .await
            .into_response();
        assert_eq!(api_post.status(), axum::http::StatusCode::CREATED);

        let api_get_all = get_hosts_api(axum::extract::State(pool.clone()))
            .await
            .into_response();
        assert_eq!(api_get_all.status(), axum::http::StatusCode::OK);

        let api_get_one = get_one_host_api(
            axum::extract::Path(new_host.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_get_one.status(), axum::http::StatusCode::OK);

        let api_del_one = delete_one_host_api(
            axum::extract::Path(new_host.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_del_one.status(), axum::http::StatusCode::OK);

        let api_del_all = delete_hosts_api(axum::extract::State(pool.clone()))
            .await
            .into_response();
        assert_eq!(api_del_all.status(), axum::http::StatusCode::OK);
    }
}
