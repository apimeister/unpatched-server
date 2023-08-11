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
    /// Insert schedule into Schedules table
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
        query(r#"INSERT INTO schedules( id, script_id, attributes, cron, active ) VALUES ( ?, ?, ?, ?, ? )"#)
            .bind(self.id.to_string())
            .bind(self.script_id.to_string())
            .bind(serde_json::to_string(&self.attributes).unwrap())
            .bind(self.cron)
            .bind(self.active)
            .execute(&mut *connection)
            .await
            .unwrap()
    }
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
    Json(schedule_vec)
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

#[allow(dead_code)]
// FIXME: make undead
pub async fn update_timestamp(
    id: Uuid,
    column: &str,
    connection: PoolConnection<Sqlite>,
) -> SqliteQueryResult {
    crate::db::update_timestamp(id, column, "schedules", connection).await
}

pub async fn count_rows(connection: PoolConnection<Sqlite>) -> Result<i64, sqlx::Error> {
    crate::db::count_rows("schedules", connection).await
}
