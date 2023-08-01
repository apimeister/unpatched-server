use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, query, sqlite::SqliteQueryResult, Row, Sqlite, SqlitePool};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Schedule {
    pub id: String,
    pub script_id: String,
    pub attributes: String,
    pub cron: String,
    pub active: bool,
}

impl Schedule {
    /// Insert schedule into Scheduling table
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
        query(r#"INSERT INTO scheduling( id, script_id, attributes, cron, active ) VALUES ( ?, ?, ?, ?, ? )"#)
            .bind(self.id)
            .bind(self.script_id)
            .bind(self.attributes)
            .bind(self.cron)
            .bind(self.active)
            .execute(&mut *connection)
            .await
            .unwrap()
    }
}

/// API to get all scripts
pub async fn get_schedules_api(
    State(pool): State<SqlitePool>,
) -> (StatusCode, Json<Vec<Schedule>>) {
    let schedule_vec = get_schedules_from_db(pool.acquire().await.unwrap()).await;
    if schedule_vec.is_empty() {
        (StatusCode::NOT_FOUND, Json(schedule_vec))
    } else {
        (StatusCode::OK, Json(schedule_vec))
    }
}

pub async fn get_schedules_from_db(mut connection: PoolConnection<Sqlite>) -> Vec<Schedule> {
    let schedules = match query("SELECT * FROM scheduling")
        .fetch_all(&mut *connection)
        .await
    {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let mut schedule_vec: Vec<Schedule> = Vec::new();

    for s in schedules {
        let schedule = Schedule {
            id: s.get::<String, _>("id"),
            script_id: s.get::<String, _>("script_id"),
            attributes: s.get::<String, _>("attributes"),
            cron: s.get::<String, _>("cron"),
            active: s.get::<bool, _>("active"),
        };
        schedule_vec.push(schedule);
    }
    schedule_vec
}
