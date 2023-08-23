use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{
    pool::PoolConnection,
    query,
    sqlite::{SqliteQueryResult, SqliteRow},
    Row, Sqlite, SqlitePool,
};
use tracing::debug;
use uuid::Uuid;

use crate::db::{get_option, utc_from_str, utc_to_str};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Execution {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub request: DateTime<Utc>,
    pub response: Option<DateTime<Utc>>,
    pub host_id: Uuid,
    #[serde(default = "Uuid::nil")]
    pub sched_id: Uuid,
    #[serde(default = "Utc::now")]
    pub created: DateTime<Utc>,
    #[serde(default = "String::new")]
    pub output: String,
}

impl Execution {
    /// Insert into or Replace `Execution` in executions table in SQLite database
    ///
    /// | Name | Type | Comment
    /// :--- | :--- | :---
    /// | id | TEXT | uuid
    /// | request | TEXT | as rfc3339 string ("YYYY-MM-DDTHH:MM:SS.sssZ")
    /// | response | TEXT | as rfc3339 string ("YYYY-MM-DDTHH:MM:SS.sssZ") <-- implemented by another call, always created as NULL
    /// | host_id | TEXT | uuid
    /// | sched_id | TEXT | uuid
    /// | created | TEXT | as rfc3339 string ("YYYY-MM-DDTHH:MM:SS.sssZ")
    /// | output | TEXT | <-- implemented by another call, always created as NULL
    pub async fn insert_into_db(self, mut connection: PoolConnection<Sqlite>) -> SqliteQueryResult {
        let q = r#"REPLACE INTO executions( id, request, host_id, sched_id, created ) VALUES( ?, ?, ?, ?, ? )"#;
        query(q)
            .bind(self.id.to_string())
            .bind(utc_to_str(self.request))
            .bind(self.host_id.to_string())
            .bind(self.sched_id.to_string())
            .bind(utc_to_str(Utc::now()))
            .execute(&mut *connection)
            .await
            .unwrap()
    }
}

impl From<SqliteRow> for Execution {
    fn from(s: SqliteRow) -> Self {
        let res = get_option(&s, "response");
        Execution {
            id: s.get::<String, _>("id").parse().unwrap(),
            request: utc_from_str(&s.get::<String, _>("request")),
            response: res.map(|r| utc_from_str(&r)),
            host_id: s.get::<String, _>("host_id").parse().unwrap(),
            sched_id: s.get::<String, _>("sched_id").parse().unwrap(),
            created: utc_from_str(&s.get::<String, _>("created")),
            output: s.get::<String, _>("output"),
        }
    }
}

/// API to get all executions
pub async fn get_executions_api(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let execution_vec = get_executions_from_db(None, pool.acquire().await.unwrap()).await;
    Json(execution_vec)
}

/// API to get one execution
pub async fn get_one_execution_api(
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    let execution_vec = get_executions_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    Json(execution_vec.first().cloned())
}

/// API to delete all executions
pub async fn delete_executions_api(State(pool): State<SqlitePool>) -> impl IntoResponse {
    delete_executions_from_db(None, pool.acquire().await.unwrap()).await
}

/// API to delete one execution
pub async fn delete_one_execution_api(
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    delete_executions_from_db(Some(&filter), pool.acquire().await.unwrap()).await
}

/// API to create a new execution
pub async fn post_executions_api(
    State(pool): State<SqlitePool>,
    Json(payload): Json<Execution>,
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

pub async fn get_executions_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> Vec<Execution> {
    let stmt = if let Some(f) = filter {
        format!("SELECT * FROM executions WHERE {f}")
    } else {
        "SELECT * FROM executions".into()
    };
    let executions = match query(&stmt).fetch_all(&mut *connection).await {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    executions.into_iter().map(|s| s.into()).collect()
}

pub async fn delete_executions_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> StatusCode {
    let stmt = if let Some(f) = filter {
        format!("DELETE FROM executions WHERE {f}")
    } else {
        "DELETE FROM executions".into()
    };
    let res = query(&stmt).execute(&mut *connection).await;
    if res.is_err() {
        StatusCode::FORBIDDEN
    } else {
        StatusCode::OK
    }
}

pub async fn update_text_field(
    id: Uuid,
    column: &str,
    data: String,
    connection: PoolConnection<Sqlite>,
) -> SqliteQueryResult {
    crate::db::update_text_field(id, column, data, "executions", connection).await
}

#[allow(dead_code)]
// FIXME: make undead
pub async fn count_rows(connection: PoolConnection<Sqlite>) -> Result<i64, sqlx::Error> {
    crate::db::count_rows("executions", connection).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_database, init_database};
    use tracing_subscriber::{
        fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
    };

    #[tokio::test]
    async fn test_executions() {
        registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()))
            .with(fmt::layer())
            .try_init()
            .unwrap_or(());

        let pool = create_database("sqlite::memory:").await.unwrap();

        init_database(&pool).await.unwrap();
        let executions = get_executions_from_db(None, pool.acquire().await.unwrap()).await;
        assert_eq!(executions.len(), 0);

        let mut execution = Execution {
            id: Uuid::new_v4(),
            ..Default::default()
        };
        let _i1 = execution
            .clone()
            .insert_into_db(pool.acquire().await.unwrap())
            .await;
        execution.id = Uuid::new_v4();
        let _i2 = execution
            .clone()
            .insert_into_db(pool.acquire().await.unwrap())
            .await;

        let executions = count_rows(pool.acquire().await.unwrap()).await.unwrap();
        assert_eq!(executions, 2);

        let err_executions =
            get_executions_from_db(Some("this-doesnt-work"), pool.acquire().await.unwrap()).await;
        assert_eq!(err_executions.len(), 0);

        let new_sched_id = Uuid::new_v4();
        let _upd = update_text_field(
            execution.id,
            "sched_id",
            new_sched_id.to_string(),
            pool.acquire().await.unwrap(),
        )
        .await;
        let executions = get_executions_from_db(
            Some(format!("id='{}'", execution.id).as_str()),
            pool.acquire().await.unwrap(),
        )
        .await;
        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].sched_id, new_sched_id);

        let single_del = delete_executions_from_db(
            Some(format!("id='{}'", execution.id).as_str()),
            pool.acquire().await.unwrap(),
        )
        .await;
        assert_eq!(single_del, axum::http::StatusCode::OK);
        let executions = count_rows(pool.acquire().await.unwrap()).await.unwrap();
        assert_eq!(executions, 1);

        let del_fail =
            delete_executions_from_db(Some("this-doesnt-work"), pool.acquire().await.unwrap())
                .await;
        assert_eq!(del_fail, axum::http::StatusCode::FORBIDDEN);

        let del = delete_executions_from_db(None, pool.acquire().await.unwrap()).await;
        assert_eq!(del, axum::http::StatusCode::OK);
        let executions = count_rows(pool.acquire().await.unwrap()).await.unwrap();
        assert_eq!(executions, 0);
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
        let new_execution = Execution {
            id: Uuid::new_v4(),
            ..Default::default()
        };

        let api_post = post_executions_api(
            axum::extract::State(pool.clone()),
            Json(new_execution.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_post.status(), axum::http::StatusCode::CREATED);

        let api_get_all = get_executions_api(axum::extract::State(pool.clone()))
            .await
            .into_response();
        assert_eq!(api_get_all.status(), axum::http::StatusCode::OK);

        let api_get_one = get_one_execution_api(
            axum::extract::Path(new_execution.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_get_one.status(), axum::http::StatusCode::OK);

        let api_del_one = delete_one_execution_api(
            axum::extract::Path(new_execution.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_del_one.status(), axum::http::StatusCode::OK);

        let api_del_all = delete_executions_api(axum::extract::State(pool.clone()))
            .await
            .into_response();
        assert_eq!(api_del_all.status(), axum::http::StatusCode::OK);
    }
}
