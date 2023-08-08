use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, query, sqlite::SqliteQueryResult, Row, Sqlite, SqlitePool};
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
            .bind(serde_json::to_string(&self.id).unwrap())
            .bind(serde_json::to_string(&self.script_id).unwrap())
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

/// API to get all schedules
pub async fn get_schedules_api(
    State(pool): State<SqlitePool>,
) -> (StatusCode, Json<Vec<Schedule>>) {
    let schedule_vec = get_schedules_from_db(None, pool.acquire().await.unwrap()).await;
    if schedule_vec.is_empty() {
        (StatusCode::NOT_FOUND, Json(schedule_vec))
    } else {
        (StatusCode::OK, Json(schedule_vec))
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

    let mut schedule_vec: Vec<Schedule> = Vec::new();

    for s in schedules {
        let schedule = Schedule {
            id: serde_json::from_str(&s.get::<String, _>("id")).unwrap(),
            script_id: serde_json::from_str(&s.get::<String, _>("script_id")).unwrap(),
            attributes: serde_json::from_str(&s.get::<String, _>("attributes")).unwrap(),
            cron: s.get::<String, _>("cron"),
            active: s.get::<bool, _>("active"),
        };
        schedule_vec.push(schedule);
    }
    schedule_vec
}

pub async fn count_rows(connection: PoolConnection<Sqlite>) -> Result<i64, sqlx::Error> {
    crate::db::count_rows("schedules", connection).await
}
