use std::str::FromStr;

use crate::{
    schedule::{self, Schedule},
    script::{self, Script},
};
use chrono::{DateTime, ParseError, Utc};
use sqlx::{
    pool::PoolConnection,
    query,
    sqlite::{SqliteConnectOptions, SqliteQueryResult, SqliteRow},
    Pool, Row, Sqlite, SqlitePool,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub fn utc_from_str(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().into()
}

pub fn utc_to_str(s: DateTime<Utc>) -> String {
    s.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn try_utc_from_str(s: &str) -> Result<DateTime<Utc>, ParseError> {
    let step: DateTime<Utc> = DateTime::parse_from_rfc3339(s)?.into();
    Ok(step)
}

/// create database
/// * sqlite::memory: - Open an in-memory database
/// * sqlite:data.db - Open the file data.db in the current directory.
/// * sqlite://data.db - Open the file data.db in the current directory.
/// * sqlite:///data.db - Open the file data.db from the root (/) directory.
/// * sqlite://data.db?mode=ro - Open the file data.db for read-only access.
///
/// types: https://www.sqlite.org/datatype3.html
pub async fn create_database(connection: &str) -> Result<SqlitePool, sqlx::Error> {
    let connection_options = match SqliteConnectOptions::from_str(connection) {
        Ok(c) => c,
        Err(e) => {
            return Err(sqlx::Error::Protocol(format!(
                "Connection config could not be parsed!\n{e}"
            )))
        }
    };
    SqlitePool::connect_with(connection_options.create_if_missing(true)).await
}

/// Initialize Database with
/// * hosts table
/// * scripts table
/// * executions table
/// * schedules table
/// * sample scripts
/// * sample schedules
///
/// More info: [DB.md](DB.md)
pub async fn init_database(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    create_hosts_table(pool.acquire().await?).await?;
    create_scripts_table(pool.acquire().await?).await?;
    create_executions_table(pool.acquire().await?).await?;
    create_schedules_table(pool.acquire().await?).await?;
    let tables = query("PRAGMA table_list;")
        .fetch_all(&mut *pool.acquire().await.unwrap())
        .await?;
    info!("DB Init: created {} tables", tables.len());
    let script_count = script::count_rows(pool.acquire().await?).await?;
    let schedule_count = schedule::count_rows(pool.acquire().await?).await?;
    if script_count == 0 && schedule_count == 0 {
        init_samples(pool).await
    } else {
        debug!("DB init: script table or schedules table has data, samples not loaded");
    }
    Ok(())
}

/// Create hosts table in SQLite database
///
/// | Name | Type | Comment
/// :--- | :--- | :---
/// | id | TEXT | uuid
/// | alias | TEXT | host alias (name)
/// | attributes | TEXT | host labels
/// | ip | TEXT | host ip:port
/// | seed_key | TEXT | uuid
/// | api_key | TEXT | uuid
/// | api_key_ttl | TEXT | as rfc3339 string ("YYYY-MM-DDTHH:MM:SS.sssZ")
/// | last_pong | TEXT | last checkin from agent
async fn create_hosts_table(mut connection: PoolConnection<Sqlite>) -> Result<(), sqlx::Error> {
    let _res = query(
        r#"CREATE TABLE IF NOT EXISTS 
        hosts(
            id TEXT PRIMARY KEY NOT NULL,
            alias TEXT,
            attributes TEXT,
            ip TEXT,
            seed_key TEXT,
            api_key TEXT,
            api_key_ttl TEXT,
            last_pong TEXT
        )"#,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Create Scripts Table in SQLite Database
///
/// | Name | Type | Comment
/// :--- | :--- | :---
/// | id | TEXT | uuid
/// | name | TEXT |
/// | version | TEXT |
/// | output_regex | TEXT | regex for result parsing
/// | labels | TEXT | script labels
/// | timeout | TEXT | timeout (1s, 5m, 3h etc.)
/// | script_content | TEXT | original script
async fn create_scripts_table(mut connection: PoolConnection<Sqlite>) -> Result<(), sqlx::Error> {
    let _res = query(
        r#"CREATE TABLE IF NOT EXISTS 
        scripts(
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT,
            version TEXT,
            output_regex TEXT,
            labels TEXT,
            timeout TEXT,
            script_content TEXT
        )"#,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Create Executions Table in SQLite Database
///
/// | Name | Type | Comment
/// :--- | :--- | :---
/// | id | TEXT | uuid
/// | request | TEXT | as rfc3339 string ("YYYY-MM-DDTHH:MM:SS.sssZ")
/// | response | TEXT | as rfc3339 string ("YYYY-MM-DDTHH:MM:SS.sssZ")
/// | host_id | TEXT | uuid
/// | sched_id | TEXT | uuid
/// | created | TEXT | as rfc3339 string ("YYYY-MM-DDTHH:MM:SS.sssZ")
/// | output | TEXT |
async fn create_executions_table(
    mut connection: PoolConnection<Sqlite>,
) -> Result<(), sqlx::Error> {
    let _res = query(
        r#"CREATE TABLE IF NOT EXISTS 
        executions(
            id TEXT PRIMARY KEY NOT NULL,
            request TEXT,
            response TEXT,
            host_id TEXT,
            sched_id TEXT,
            created TEXT,
            output TEXT
        )"#,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Create Schedules Table in SQLite Database
///
/// | Name | Type | Comment
/// :--- | :--- | :---
/// | id | TEXT | uuid
/// | script_id | TEXT | uuid
/// | attributes | TEXT | server label to execute on
/// | cron | TEXT | cron pattern for execution
/// | active | NUMERIC | boolean
async fn create_schedules_table(mut connection: PoolConnection<Sqlite>) -> Result<(), sqlx::Error> {
    let _res = query(
        r#"CREATE TABLE IF NOT EXISTS 
        schedules(
            id TEXT PRIMARY KEY NOT NULL,
            script_id TEXT,
            attributes TEXT,
            cron TEXT,
            active NUMERIC
        )"#,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

async fn init_samples(pool: &Pool<Sqlite>) {
    let uptime_linux = Script {
        id: Uuid::new_v4(),
        name: "uptime".into(),
        version: "0.0.1".into(),
        output_regex: ".*".into(),
        labels: vec!["linux".to_string(), "sample1".to_string()],
        timeout: "5s".into(),
        script_content: r#"uptime -p"#.into(),
    };
    let os_version_linux = Script {
        id: Uuid::new_v4(),
        name: "os_version".into(),
        version: "0.0.1".into(),
        output_regex: ".*".into(),
        labels: vec!["linux".to_string(), "sample2".to_string()],
        timeout: "5s".into(),
        script_content: r#"cat /etc/os-release"#.into(),
    };
    let uptime_mac = Script {
        id: Uuid::new_v4(),
        name: "uptime".into(),
        version: "0.0.1".into(),
        output_regex: ".*".into(),
        labels: vec!["mac".to_string(), "sample3".to_string()],
        timeout: "5s".into(),
        script_content: r#"uptime"#.into(),
    };
    let os_version_mac = Script {
        id: Uuid::new_v4(),
        name: "os_version".into(),
        version: "0.0.1".into(),
        output_regex: ".*".into(),
        labels: vec!["mac".to_string(), "sample4".to_string()],
        timeout: "5s".into(),
        script_content: r#"sw_vers"#.into(),
    };
    let v = vec![
        uptime_linux.clone(),
        os_version_linux,
        uptime_mac,
        os_version_mac,
    ];
    for s in v {
        let res = s
            .clone()
            .insert_into_db(pool.acquire().await.unwrap())
            .await;
        if res.rows_affected() > 0 {
            info!(
                "DB init: sample script {} version {} with labels {} loaded",
                s.name,
                s.version,
                s.labels()
            );
        } else {
            warn!(
                "DB init: sample script {} version {} with labels {} could not be loaded",
                s.name,
                s.version,
                s.labels()
            );
        }
        let sched = Schedule {
            id: Uuid::new_v4(),
            script_id: s.id,
            attributes: vec![s.labels[0].clone()],
            cron: "0 * * * * * *".into(),
            active: true,
        };
        let sched_res = sched
            .clone()
            .insert_into_db(pool.acquire().await.unwrap())
            .await;
        if sched_res.rows_affected() > 0 {
            info!(
                "DB init: sample schedule for script {} version {} with attributes {} loaded",
                s.name,
                s.version,
                sched.attributes()
            );
        } else {
            warn!(
                "DB init: sample schedule for script {} version {} with attributes {} could not be loaded",
                s.name, s.version, sched.attributes()
            );
        }
    }
    let sched = Schedule {
        id: Uuid::new_v4(),
        script_id: uptime_linux.id,
        attributes: vec![uptime_linux.labels[0].clone()],
        cron: utc_to_str(Utc::now()),
        active: true,
    };

    let sched_res = sched
        .clone()
        .insert_into_db(pool.acquire().await.unwrap())
        .await;
    if sched_res.rows_affected() > 0 {
        info!(
            "DB init: sample one-time schedule for script {} version {} with attributes {} loaded",
            uptime_linux.name,
            uptime_linux.version,
            sched.attributes()
        );
    } else {
        warn!(
                "DB init: sample one-time schedule for script {} version {} with attributes {} could not be loaded",
                uptime_linux.name, uptime_linux.version, sched.attributes()
            );
    }
}

pub async fn update_text_field(
    id: Uuid,
    column: &str,
    data: String,
    table: &str,
    mut connection: PoolConnection<Sqlite>,
) -> SqliteQueryResult {
    let stmt = format!("UPDATE {} SET {} = ? WHERE id = ?", table, column);
    match query(&stmt)
        .bind(data)
        // extra quotes are needed since uuid.json results in "value" instead of value
        .bind(id.to_string())
        .execute(&mut *connection)
        .await
    {
        Ok(q) => q,
        Err(e) => {
            error!("Updating {column} for {id} in {table} failed\n{e}");
            SqliteQueryResult::default()
        }
    }
}

pub async fn count_rows(
    table: &str,
    mut connection: PoolConnection<Sqlite>,
) -> Result<i64, sqlx::Error> {
    let stmt = format!("SELECT count(_rowid_) as id_count FROM {table}");
    let script_count = query(&stmt).fetch_one(&mut *connection).await?;
    Ok(script_count.get::<i64, _>("id_count"))
}

pub fn get_option(row: &SqliteRow, column: &str) -> Option<String> {
    let res = row.get::<String, _>(column);
    if res.is_empty() {
        None
    } else {
        Some(res)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use tracing_subscriber::{
        fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
    };

    #[tokio::test]
    async fn test_init_database() {
        registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()))
            .with(fmt::layer())
            .try_init()
            .unwrap_or(());

        let pool = create_database("sqlite::memory:").await.unwrap();
        init_database(&pool).await.unwrap();

        let tables = query("PRAGMA table_list;")
            .fetch_all(&mut *pool.acquire().await.unwrap())
            .await
            .unwrap();
        assert_eq!(tables.len(), 6);

        // run again to check already-present branch
        init_database(&pool).await.unwrap();
    }

    #[tokio::test]
    async fn test_update_text_field_error() {
        registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()))
            .with(fmt::layer())
            .try_init()
            .unwrap_or(());

        let pool = create_database("sqlite::memory:").await.unwrap();
        init_database(&pool).await.unwrap();

        let up = update_text_field(
            Uuid::new_v4(),
            "fail-test",
            "fail-test".into(),
            "unknown",
            pool.acquire().await.unwrap(),
        )
        .await;
        assert_eq!(
            up.last_insert_rowid(),
            SqliteQueryResult::default().last_insert_rowid()
        );
        assert_eq!(
            up.rows_affected(),
            SqliteQueryResult::default().rows_affected()
        );
    }

    #[tokio::test]
    async fn test_init_samples() {
        registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()))
            .with(fmt::layer())
            .try_init()
            .unwrap_or(());

        let pool = create_database("sqlite::memory:").await.unwrap();
        init_database(&pool).await.unwrap();

        let scripts = script::get_scripts_from_db(None, pool.acquire().await.unwrap()).await;
        let schedules = schedule::get_schedules_from_db(None, pool.acquire().await.unwrap()).await;
        assert_eq!(scripts.len(), 4);
        assert_eq!(schedules.len(), 5);

        // run again to tests already-present branch
        init_samples(&pool).await;
        let scripts = script::get_scripts_from_db(None, pool.acquire().await.unwrap()).await;
        let schedules = schedule::get_schedules_from_db(None, pool.acquire().await.unwrap()).await;
        assert_eq!(scripts.len(), 8);
        assert_eq!(schedules.len(), 10);
    }
}
