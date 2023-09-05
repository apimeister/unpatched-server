use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
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

use crate::{
    db::{utc_from_str, utc_to_str},
    host::{get_hosts_from_db, ScheduleState},
    jwt::Claims,
};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Schedule {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub script_id: Uuid,
    #[serde(default)]
    pub target: Target,
    pub timer: Timer,
    pub active: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct ExtSchedule {
    id: Uuid,
    script_id: Uuid,
    target: Target,
    timer: Timer,
    active: bool,
    last_execution: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    Attributes(Vec<String>),
    HostId(Uuid),
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Timer {
    Cron(String),
    Timestamp(DateTime<Utc>),
}

impl Default for Target {
    fn default() -> Self {
        Target::Attributes(Vec::new())
    }
}

impl Default for Timer {
    fn default() -> Self {
        Timer::Timestamp(Utc::now())
    }
}

impl Schedule {
    /// Insert into or Replace `Schedule` in schedules table in SQLite database
    ///
    /// | Name | Type | Comment
    /// :--- | :--- | :---
    /// | id | TEXT | uuid
    /// | script_id | TEXT | uuid
    /// | target_attributes | TEXT | server label to execute on
    /// | target_host_id | TEXT | uuid
    /// | timer_cron | TEXT | cron pattern for execution
    /// | timer_ts | TEXT | cron pattern for execution
    /// | active | bool |
    #[allow(dead_code)]
    // FIXME: write test and remove dead_code
    pub async fn insert_into_db(
        self,
        mut connection: PoolConnection<Sqlite>,
    ) -> Result<SqliteQueryResult, sqlx::Error> {
        let target = match self.target {
            Target::Attributes(attr) => (Some(serde_json::to_string(&attr).unwrap()), None),
            Target::HostId(uuid) => (None, Some(uuid.to_string())),
        };
        let timer = match self.timer {
            Timer::Cron(c) => (Some(c), None),
            Timer::Timestamp(ts) => (None, Some(utc_to_str(ts))),
        };

        let q = r#"REPLACE INTO schedules( id, script_id, target_attributes, target_host_id, timer_cron, timer_ts, active ) VALUES ( ?, ?, ?, ?, ?, ?, ? )"#;
        query(q)
            .bind(self.id.to_string())
            .bind(self.script_id.to_string())
            .bind(target.0)
            .bind(target.1)
            .bind(timer.0)
            .bind(timer.1)
            .bind(self.active)
            .execute(&mut *connection)
            .await
    }

    /// list of attributes as comma-seperated `String`
    pub fn attributes(&self) -> String {
        if let Target::Attributes(attr) = &self.target {
            attr.join(",")
        } else {
            "".into()
        }
    }

    /// sorted list of attributes as comma-seperated `String`
    pub fn sorted_attributes(&self) -> String {
        if let Target::Attributes(ref attr) = &self.target {
            let mut att = attr.clone();
            att.sort();
            att.join(",")
        } else {
            "".into()
        }
    }
}

impl From<SqliteRow> for Schedule {
    fn from(s: SqliteRow) -> Self {
        let target = match s.get::<String, _>("target_host_id").parse() {
            Ok(x) => Target::HostId(x),
            Err(_) => Target::Attributes(
                serde_json::from_str(&s.get::<String, _>("target_attributes")).unwrap(),
            ),
        };
        let ts = s.get::<String, _>("timer_ts");
        let timer = if !ts.is_empty() {
            Timer::Timestamp(utc_from_str(&ts))
        } else {
            Timer::Cron(s.get::<String, _>("timer_cron"))
        };

        Schedule {
            id: s.get::<String, _>("id").parse().unwrap(),
            script_id: s.get::<String, _>("script_id").parse().unwrap(),
            target,
            timer,
            active: s.get::<bool, _>("active"),
        }
    }
}

/// API to get all schedules
pub async fn get_schedules_api(
    _claims: Claims,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let schedule_vec = get_schedules_from_db(None, pool.acquire().await.unwrap()).await;
    let mut sched_vec: Vec<ExtSchedule> = Vec::new();
    for sched in &schedule_vec {
        let now = utc_to_str(Utc::now());
        let q = format!("SELECT sched_id, response FROM executions WHERE response < '{now}' AND sched_id='{}' ORDER BY response desc LIMIT 1;", sched.id);
        let Ok(exe) = query(&q)
            .fetch_optional(&mut *pool.acquire().await.unwrap())
            .await
        else {
            debug!("Query for executions for {} failed. Skip", sched.id);
            continue;
        };
        let last_execution = exe.map(|a| utc_from_str(&a.get::<String, _>("response")));

        sched_vec.push(ExtSchedule {
            last_execution,
            id: sched.id,
            script_id: sched.script_id,
            target: sched.target.clone(),
            timer: sched.timer.clone(),
            active: sched.active,
        })
    }
    debug!("{:?}", sched_vec);
    Json(sched_vec)
}

#[derive(Debug, Deserialize)]
pub struct ScheduleStateParams {
    filter: Option<ScheduleState>,
}

/// API to get all schedules for a host
pub async fn get_host_schedules_api(
    _claims: Claims,
    Path(id): Path<Uuid>,
    axum::extract::Query(params): axum::extract::Query<ScheduleStateParams>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'");
    let hosts = get_hosts_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    let Some(host) = hosts.first() else {
        return Json(Vec::new());
    };
    let filter_state = params.filter.unwrap_or_default();
    let schedules = host
        .get_all_schedules(pool.acquire().await.unwrap(), filter_state)
        .await;
    let mut sched_vec: Vec<ExtSchedule> = Vec::new();
    for sched in &schedules {
        let now = utc_to_str(Utc::now());
        let q = format!("SELECT sched_id, response FROM executions WHERE response < '{now}' AND host_id='{id}' AND sched_id='{}' ORDER BY response desc LIMIT 1;", sched.id);
        let Ok(exe) = query(&q)
            .fetch_optional(&mut *pool.acquire().await.unwrap())
            .await
        else {
            debug!("Query for executions for {} failed. Skip", sched.id);
            continue;
        };
        let last_execution = exe.map(|a| utc_from_str(&a.get::<String, _>("response")));

        sched_vec.push(ExtSchedule {
            last_execution,
            id: sched.id,
            script_id: sched.script_id,
            target: sched.target.clone(),
            timer: sched.timer.clone(),
            active: sched.active,
        })
    }
    debug!("{:?}", sched_vec);
    Json(sched_vec)
}

/// API to get one schedule
pub async fn get_one_schedule_api(
    _claims: Claims,
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    let schedule_vec = get_schedules_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    Json(schedule_vec.first().cloned())
}

/// API to delete all schedules
pub async fn delete_schedules_api(
    _claims: Claims,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    delete_schedules_from_db(None, pool.acquire().await.unwrap()).await
}

/// API to delete one schedule
pub async fn delete_one_schedule_api(
    _claims: Claims,
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    delete_schedules_from_db(Some(&filter), pool.acquire().await.unwrap()).await
}

/// API to create a new schedule
pub async fn post_schedules_api(
    _claims: Claims,
    State(pool): State<SqlitePool>,
    Json(payload): Json<Schedule>,
) -> Response {
    debug!("{:?}", payload);
    let id = payload.id.to_string();
    let Ok(res) = payload.insert_into_db(pool.acquire().await.unwrap()).await else {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Script ID or Host ID not found, could not add Schedule",
        )
            .into_response();
    };
    if res.rows_affected() == 1 {
        (StatusCode::CREATED, Json(id)).into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong. Nothing added",
        )
            .into_response()
    }
}

/// API to create a new schedule for this host
pub async fn post_host_schedules_api(
    _claims: Claims,
    Path(host_id): Path<Uuid>,
    State(pool): State<SqlitePool>,
    Json(mut payload): Json<Schedule>,
) -> Response {
    payload.target = Target::HostId(host_id);
    debug!("{:?}", payload);
    let id = payload.id;
    let Ok(res) = payload.insert_into_db(pool.acquire().await.unwrap()).await else {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Script ID or Host ID not found, could not add Schedule",
        )
            .into_response();
    };
    if res.rows_affected() == 1 {
        (StatusCode::CREATED, Json(id)).into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong. Nothing added",
        )
            .into_response()
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
    use crate::{
        db::{create_database, init_database},
        host::Host,
    };
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

        init_database(&pool, None).await.unwrap();
        let schedules = get_schedules_from_db(None, pool.acquire().await.unwrap()).await;
        assert_eq!(schedules.len(), 5);

        let mut schedule = Schedule {
            script_id: schedules[0].script_id,
            ..Default::default()
        };
        let i1 = schedule
            .clone()
            .insert_into_db(pool.acquire().await.unwrap())
            .await;
        assert_eq!(i1.unwrap().rows_affected(), 1);
        schedule.id = Uuid::new_v4();
        let i2 = schedule
            .clone()
            .insert_into_db(pool.acquire().await.unwrap())
            .await;
        assert_eq!(i2.unwrap().rows_affected(), 1);
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
        let claims: Claims = Claims::default();
        init_database(&pool, None).await.unwrap();

        let api_get_all = get_schedules_api(claims.clone(), axum::extract::State(pool.clone()))
            .await
            .into_response();
        assert_eq!(api_get_all.status(), axum::http::StatusCode::OK);

        let schedules = hyper::body::to_bytes(api_get_all.into_body())
            .await
            .unwrap();
        let schedules: Vec<Schedule> = serde_json::from_slice(&schedules).unwrap();
        let new_schedule = Schedule {
            script_id: schedules[0].script_id,
            ..Default::default()
        };

        let api_post_success = post_schedules_api(
            claims.clone(),
            axum::extract::State(pool.clone()),
            Json(new_schedule.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_post_success.status(), axum::http::StatusCode::CREATED);

        let api_post_fail = post_schedules_api(
            claims.clone(),
            axum::extract::State(pool.clone()),
            Json(Schedule {
                script_id: Uuid::new_v4(),
                ..Default::default()
            }),
        )
        .await
        .into_response();
        assert_eq!(api_post_fail.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let api_get_one = get_one_schedule_api(
            claims.clone(),
            axum::extract::Path(new_schedule.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_get_one.status(), axum::http::StatusCode::OK);

        // prepare a host to reference in the execution (nil_id)
        let host = Host::default();
        let host_id = host.id;
        let _host = host.insert_into_db(pool.acquire().await.unwrap()).await;

        let post_host_schedules_api_success = post_host_schedules_api(
            claims.clone(),
            axum::extract::Path(host_id),
            axum::extract::State(pool.clone()),
            Json(new_schedule.clone()),
        )
        .await
        .into_response();
        assert_eq!(
            post_host_schedules_api_success.status(),
            axum::http::StatusCode::CREATED
        );

        let post_host_schedules_api_fail = post_host_schedules_api(
            claims.clone(),
            axum::extract::Path(host_id),
            axum::extract::State(pool.clone()),
            Json(Schedule {
                script_id: Uuid::new_v4(),
                ..Default::default()
            }),
        )
        .await
        .into_response();
        assert_eq!(
            post_host_schedules_api_fail.status(),
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );

        let get_host_schedules_api_all = get_host_schedules_api(
            claims.clone(),
            axum::extract::Path(host_id),
            axum::extract::Query(ScheduleStateParams {
                filter: Some(ScheduleState::All),
            }),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(
            get_host_schedules_api_all.status(),
            axum::http::StatusCode::OK
        );

        let get_host_schedules_api_inactive = get_host_schedules_api(
            claims.clone(),
            axum::extract::Path(host_id),
            axum::extract::Query(ScheduleStateParams {
                filter: Some(ScheduleState::Inactive),
            }),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(
            get_host_schedules_api_inactive.status(),
            axum::http::StatusCode::OK
        );

        let get_host_schedules_api_active = get_host_schedules_api(
            claims.clone(),
            axum::extract::Path(host_id),
            axum::extract::Query(ScheduleStateParams {
                filter: Some(ScheduleState::Active),
            }),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(
            get_host_schedules_api_active.status(),
            axum::http::StatusCode::OK
        );

        let api_del_one = delete_one_schedule_api(
            claims.clone(),
            axum::extract::Path(new_schedule.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_del_one.status(), axum::http::StatusCode::OK);

        let api_del_all = delete_schedules_api(claims.clone(), axum::extract::State(pool.clone()))
            .await
            .into_response();
        assert_eq!(api_del_all.status(), axum::http::StatusCode::OK);
    }
}
