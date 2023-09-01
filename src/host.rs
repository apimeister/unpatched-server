use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Days, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{
    pool::PoolConnection,
    query,
    sqlite::{SqliteQueryResult, SqliteRow},
    Row, Sqlite, SqlitePool,
};
use tracing::debug;
use uuid::Uuid;

use crate::{
    db::{try_utc_from_str, utc_to_str},
    jwt::Claims,
    schedule::{self, Schedule, Target},
};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Host {
    pub id: Uuid,
    pub alias: String,
    pub attributes: Vec<String>,
    pub ip: String,
    pub seed_key: Uuid,
    pub api_key: Option<Uuid>,
    pub api_key_ttl: Option<DateTime<Utc>>,
    pub last_pong: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleState {
    Active,
    Inactive,
    #[default]
    All,
}

impl Host {
    /// Insert into or Replace `Host` in hosts table in SQLite database
    ///
    /// | Name | Type | Comment | Extended Comment
    /// :--- | :--- | :--- | ---
    /// | id | TEXT | uuid |
    /// | alias | TEXT | host alias (name) |
    /// | attributes | TEXT | host labels |
    /// | ip | TEXT | host ip:port |
    /// | seed_key | TEXT | uuid
    /// | api_key | TEXT | uuid | implemented by another call, always created as NULL
    /// | api_key_ttl | TEXT | as rfc3339 string ("YYYY-MM-DDTHH:MM:SS.sssZ") | implemented by another call, always created as NULL
    /// | last_pong | TEXT | last checkin from agent | implemented by another call, always created as NULL
    pub async fn insert_into_db(self, mut connection: PoolConnection<Sqlite>) -> SqliteQueryResult {
        let q = r#"REPLACE INTO hosts(id, alias, attributes, seed_key, ip) VALUES(?, ?, ?, ?, ?)"#;
        query(q)
            .bind(self.id.to_string())
            .bind(self.alias)
            .bind(serde_json::to_string(&self.attributes).unwrap())
            .bind(self.seed_key.to_string())
            .bind(self.ip)
            .execute(&mut *connection)
            .await
            .unwrap()
    }

    pub async fn get_all_schedules(
        &self,
        connection: PoolConnection<Sqlite>,
        state: ScheduleState,
    ) -> Vec<Schedule> {
        let filter = match state {
            ScheduleState::Active => Some("active = 1"),
            ScheduleState::Inactive => Some("active = 0"),
            ScheduleState::All => None,
        };

        let schedules = schedule::get_schedules_from_db(filter, connection).await;

        // Add all schedules that fit via host_id or attribute to schedule list
        let mut found_schedules = Vec::new();
        let mut host_attributes = self.attributes.clone();
        host_attributes.sort();
        for sched in schedules {
            if let Target::HostId(h) = sched.target {
                if h == self.id {
                    found_schedules.push(sched.clone())
                }
            };
            if host_attributes.contains(&sched.sorted_attributes()) {
                found_schedules.push(sched)
            };
        }

        if !found_schedules.is_empty() {
            debug!("Found schedules for {}: {found_schedules:?}", self.alias,);
        }
        found_schedules
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
            seed_key: s.get::<String, _>("seed_key").parse().unwrap(),
            api_key: s.get::<String, _>("api_key").parse().ok(),
            api_key_ttl: try_utc_from_str(&s.get::<String, _>("api_key_ttl")).ok(),
            last_pong: try_utc_from_str(&s.get::<String, _>("last_pong")).ok(),
        }
    }
}

/// API to get all hosts
pub async fn get_hosts_api(_claims: Claims, State(pool): State<SqlitePool>) -> impl IntoResponse {
    let host_vec = get_hosts_from_db(None, pool.acquire().await.unwrap()).await;
    Json(host_vec)
}

/// API to get one host
pub async fn get_one_host_api(
    _claims: Claims,
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    let host_vec = get_hosts_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    Json(host_vec.first().cloned())
}

/// API to approve host
pub async fn approve_one_host_api(
    _claims: Claims,
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let ttl = Utc::now().checked_add_days(Days::new(30)).unwrap();
    let _up = update_text_field(
        id,
        "api_key",
        Uuid::new_v4().to_string(),
        pool.acquire().await.unwrap(),
    )
    .await;
    let _up = update_text_field(
        id,
        "api_key_ttl",
        utc_to_str(ttl),
        pool.acquire().await.unwrap(),
    )
    .await;
    StatusCode::OK
}

/// API to lock host
pub async fn lock_one_host_api(
    _claims: Claims,
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let _up = update_text_field(
        id,
        "api_key",
        Uuid::nil().to_string(),
        pool.acquire().await.unwrap(),
    )
    .await;
    StatusCode::OK
}

/// API to delete all hosts
pub async fn delete_hosts_api(
    _claims: Claims,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    delete_hosts_from_db(None, pool.acquire().await.unwrap()).await
}

/// API to delete one host
pub async fn delete_one_host_api(
    _claims: Claims,
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    delete_hosts_from_db(Some(&filter), pool.acquire().await.unwrap()).await
}

/// API to create a new host
pub async fn post_hosts_api(
    _claims: Claims,
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

/// API to update one host
pub async fn update_one_host_api(
    _claims: Claims,
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
    Json(payload): Json<HashMap<String, String>>,
) -> impl IntoResponse {
    debug!("{payload:?}");
    if let Some((column, data)) = payload.into_iter().next() {
        let _up = update_text_field(id, &column, data, pool.acquire().await.unwrap()).await;
    };
    let filter = format!("id='{id}'",);
    let host_vec = get_hosts_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    Json(host_vec.first().cloned())
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
    use crate::db::{create_database, init_database};
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

        init_database(&pool, "foo@foo.foo".into(), "bar".into())
            .await
            .unwrap();
        let hosts = get_hosts_from_db(None, pool.acquire().await.unwrap()).await;
        assert_eq!(hosts.len(), 0);

        let mut host = Host {
            id: Uuid::new_v4(),
            ..Default::default()
        };

        let _i1 = host
            .clone()
            .insert_into_db(pool.acquire().await.unwrap())
            .await;
        host.id = Uuid::new_v4();
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
        let claims: Claims = Claims::default();
        init_database(&pool, "foo@foo.foo".into(), "bar".into())
            .await
            .unwrap();
        let new_host = Host {
            id: Uuid::new_v4(),
            ..Default::default()
        };

        let api_post = post_hosts_api(
            claims.clone(),
            axum::extract::State(pool.clone()),
            Json(new_host.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_post.status(), axum::http::StatusCode::CREATED);

        let mut api_update = HashMap::new();
        api_update.insert("last_pong".to_string(), utc_to_str(Utc::now()));

        let api_update = update_one_host_api(
            claims.clone(),
            axum::extract::Path(new_host.id),
            axum::extract::State(pool.clone()),
            Json(api_update),
        )
        .await
        .into_response();
        assert_eq!(api_update.status(), axum::http::StatusCode::OK);

        let lock_host = lock_one_host_api(
            claims.clone(),
            axum::extract::Path(new_host.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(lock_host.status(), axum::http::StatusCode::OK);

        let approve_host = approve_one_host_api(
            claims.clone(),
            axum::extract::Path(new_host.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(approve_host.status(), axum::http::StatusCode::OK);

        let api_get_all = get_hosts_api(claims.clone(), axum::extract::State(pool.clone()))
            .await
            .into_response();
        assert_eq!(api_get_all.status(), axum::http::StatusCode::OK);

        let api_get_one = get_one_host_api(
            claims.clone(),
            axum::extract::Path(new_host.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_get_one.status(), axum::http::StatusCode::OK);

        let api_del_one = delete_one_host_api(
            claims.clone(),
            axum::extract::Path(new_host.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_del_one.status(), axum::http::StatusCode::OK);

        let api_del_all = delete_hosts_api(claims.clone(), axum::extract::State(pool.clone()))
            .await
            .into_response();
        assert_eq!(api_del_all.status(), axum::http::StatusCode::OK);
    }
}
