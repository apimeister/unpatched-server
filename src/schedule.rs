use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
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
pub struct Schedule {
    pub id: Uuid,
    pub script_id: Uuid,
    pub attributes: Vec<String>,
    pub cron: String,
    pub active: bool,
}

impl Schedule {
    /// Insert into or Replace `Schedule` in schedules table in SQLite database
    ///
    /// | Name | Type | Comment
    /// :--- | :--- | :---
    /// | id | TEXT | uuid
    /// | script_id | TEXT | uuid
    /// | attributes | TEXT | server label to execute on
    /// | cron | TEXT | cron pattern for execution
    /// | active | bool |
    #[allow(dead_code)]
    // FIXME: write test and remove dead_code
    pub async fn insert_into_db(self, mut connection: PoolConnection<Sqlite>) -> SqliteQueryResult {
        let q = r#"REPLACE INTO schedules( id, script_id, attributes, cron, active ) VALUES ( ?, ?, ?, ?, ? )"#;
        query(q)
            .bind(self.id.to_string())
            .bind(self.script_id.to_string())
            .bind(serde_json::to_string(&self.attributes).unwrap())
            .bind(self.cron)
            .bind(self.active)
            .execute(&mut *connection)
            .await
            .unwrap()
    }

    /// list attributes as comma-seperated `String`
    pub fn attributes(&self) -> String {
        self.attributes.join(",")
    }
}

impl From<SqliteRow> for Schedule {
    fn from(s: SqliteRow) -> Self {
        Schedule {
            id: s.get::<String, _>("id").parse().unwrap(),
            script_id: s.get::<String, _>("script_id").parse().unwrap(),
            attributes: serde_json::from_str(&s.get::<String, _>("attributes")).unwrap(),
            cron: s.get::<String, _>("cron"),
            active: s.get::<bool, _>("active"),
        }
    }
}

/// API to get all schedules
pub async fn get_schedules_api(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let schedule_vec = get_schedules_from_db(None, pool.acquire().await.unwrap()).await;
    Json(schedule_vec)
}

/// API to get one schedule
pub async fn get_one_schedule_api(
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    let schedule_vec = get_schedules_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    Json(schedule_vec.first().cloned())
}

/// API to delete all schedules
pub async fn delete_schedules_api(State(pool): State<SqlitePool>) -> impl IntoResponse {
    delete_schedules_from_db(None, pool.acquire().await.unwrap()).await
}

/// API to delete one schedule
pub async fn delete_one_schedule_api(
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    delete_schedules_from_db(Some(&filter), pool.acquire().await.unwrap()).await
}

/// API to create a new schedule
pub async fn post_schedules_api(
    State(pool): State<SqlitePool>,
    Json(payload): Json<Schedule>,
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

pub async fn get_schedules_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> Vec<Schedule> {
    let stmt = if let Some(f) = filter {
        format!("SELECT * FROM schedules WHERE {f}")
    } else {
        "SELECT * FROM schedules".into()
    };
    let schedules = match query(&stmt).fetch_all(&mut *connection).await {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    schedules.into_iter().map(|s| s.into()).collect()
}

pub async fn delete_schedules_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> StatusCode {
    let stmt = if let Some(f) = filter {
        format!("DELETE FROM schedules WHERE {f}")
    } else {
        "DELETE FROM schedules".into()
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
    crate::db::update_text_field(id, column, data, "schedules", connection).await
}

pub async fn count_rows(connection: PoolConnection<Sqlite>) -> Result<i64, sqlx::Error> {
    crate::db::count_rows("schedules", connection).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_database, init_database, new_id};
    use tracing_subscriber::{
        fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
    };

    #[tokio::test]
    async fn test_schedules() {
        registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()))
            .with(fmt::layer())
            .try_init()
            .unwrap_or(());

        let pool = create_database("sqlite::memory:").await.unwrap();

        init_database(&pool).await.unwrap();
        let schedules = get_schedules_from_db(None, pool.acquire().await.unwrap()).await;
        assert_eq!(schedules.len(), 5);

        let mut schedule = Schedule {
            id: new_id(),
            ..Default::default()
        };

        let _i1 = schedule
            .clone()
            .insert_into_db(pool.acquire().await.unwrap())
            .await;
        schedule.id = new_id();
        let _i2 = schedule
            .clone()
            .insert_into_db(pool.acquire().await.unwrap())
            .await;

        let schedules = count_rows(pool.acquire().await.unwrap()).await.unwrap();
        assert_eq!(schedules, 7);

        let err_schedules =
            get_schedules_from_db(Some("this-doesnt-work"), pool.acquire().await.unwrap()).await;
        assert_eq!(err_schedules.len(), 0);

        let _upd = update_text_field(
            schedule.id,
            "active",
            "1".to_string(),
            pool.acquire().await.unwrap(),
        )
        .await;
        let schedules = get_schedules_from_db(
            Some(format!("id='{}'", schedule.id).as_str()),
            pool.acquire().await.unwrap(),
        )
        .await;
        assert_eq!(schedules.len(), 1);
        assert!(schedules[0].active);

        let single_del = delete_schedules_from_db(
            Some(format!("id='{}'", schedule.id).as_str()),
            pool.acquire().await.unwrap(),
        )
        .await;
        assert_eq!(single_del, axum::http::StatusCode::OK);
        let schedules = count_rows(pool.acquire().await.unwrap()).await.unwrap();
        assert_eq!(schedules, 6);

        let del_fail =
            delete_schedules_from_db(Some("this-doesnt-work"), pool.acquire().await.unwrap()).await;
        assert_eq!(del_fail, axum::http::StatusCode::FORBIDDEN);

        let del = delete_schedules_from_db(None, pool.acquire().await.unwrap()).await;
        assert_eq!(del, axum::http::StatusCode::OK);
        let schedules = count_rows(pool.acquire().await.unwrap()).await.unwrap();
        assert_eq!(schedules, 0);
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
        let new_schedule = Schedule {
            id: new_id(),
            ..Default::default()
        };

        let api_post = post_schedules_api(
            axum::extract::State(pool.clone()),
            Json(new_schedule.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_post.status(), axum::http::StatusCode::CREATED);

        let api_get_all = get_schedules_api(axum::extract::State(pool.clone()))
            .await
            .into_response();
        assert_eq!(api_get_all.status(), axum::http::StatusCode::OK);

        let api_get_one = get_one_schedule_api(
            axum::extract::Path(new_schedule.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_get_one.status(), axum::http::StatusCode::OK);

        let api_del_one = delete_one_schedule_api(
            axum::extract::Path(new_schedule.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_del_one.status(), axum::http::StatusCode::OK);

        let api_del_all = delete_schedules_api(axum::extract::State(pool.clone()))
            .await
            .into_response();
        assert_eq!(api_del_all.status(), axum::http::StatusCode::OK);
    }
}
