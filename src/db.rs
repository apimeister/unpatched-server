use std::{str::FromStr, time::Duration};

use crate::{
    schedule::{self, Schedule},
    script::{self, Script},
    user::{self, hash_password, User},
};
use chrono::{DateTime, ParseError, Utc};
use email_address::EmailAddress;
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
pub async fn init_database(
    pool: &SqlitePool,
    creds: Option<(EmailAddress, String)>,
) -> Result<(), sqlx::Error> {
    let _res = query(r#"PRAGMA foreign_keys = ON;"#)
        .execute(pool.acquire().await?.as_mut())
        .await?;

    create_hosts_table(pool.acquire().await?).await?;
    create_scripts_table(pool.acquire().await?).await?;
    create_executions_table(pool.acquire().await?).await?;
    create_schedules_table(pool.acquire().await?).await?;
    create_users_table(pool.acquire().await?).await?;
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
    let user_count = user::count_rows(pool.acquire().await?).await?;
    if user_count == 0 {
        let Some((email, password)) = creds else {
            warn!("No init user provided with empty database, skipping init user creation");
            return Ok(());
        };
        init_main_user(pool, email, password).await
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
/// | active | NUMERIC | bool
/// | last_checkin | TEXT | last checkin from agent
/// | created | TEXT | as rfc3339 string ("YYYY-MM-DDTHH:MM:SS.sssZ")
async fn create_hosts_table(mut connection: PoolConnection<Sqlite>) -> Result<(), sqlx::Error> {
    let _res = query(
        r#"CREATE TABLE IF NOT EXISTS 
        hosts(
            id TEXT PRIMARY KEY NOT NULL,
            alias TEXT,
            attributes TEXT,
            ip TEXT,
            active NUMERIC,
            last_checkin TEXT,
            created TEXT
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
/// | timeout_in_s | INT | timeout in seconds
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
            timeout_in_s INT,
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
            output TEXT,
            FOREIGN KEY(host_id) REFERENCES hosts(id) ON DELETE CASCADE,
            FOREIGN KEY(sched_id) REFERENCES schedules(id) ON DELETE CASCADE
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
/// | target_attributes | TEXT | server label to execute on
/// | target_host_id | TEXT | server uuid to execute on
/// | timer_cron | TEXT | cron pattern for execution
/// | timer_ts | TEXT | timestamp for execution
/// | active | NUMERIC | boolean
async fn create_schedules_table(mut connection: PoolConnection<Sqlite>) -> Result<(), sqlx::Error> {
    let _res = query(
        r#"CREATE TABLE IF NOT EXISTS 
        schedules(
            id TEXT PRIMARY KEY NOT NULL,
            script_id TEXT,
            target_attributes TEXT,
            target_host_id TEXT,
            timer_cron TEXT,
            timer_ts TEXT,
            active NUMERIC,
            FOREIGN KEY(script_id) REFERENCES scripts(id) ON DELETE CASCADE,
            FOREIGN KEY(target_host_id) REFERENCES hosts(id) ON DELETE CASCADE
        )"#,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Create Users Table in SQLite Database
///
/// | Name | Type | Comment
/// :--- | :--- | :---
/// | id | TEXT | uuid
/// | email | TEXT |
/// | password | TEXT |
/// | roles | TEXT |
/// | active | NUMERIC |
/// | created | TEXT | as rfc3339 string ("YYYY-MM-DDTHH:MM:SS.sssZ")
async fn create_users_table(mut connection: PoolConnection<Sqlite>) -> Result<(), sqlx::Error> {
    let _res = query(
        r#"CREATE TABLE IF NOT EXISTS 
        users(
            id TEXT NOT NULL,
            email TEXT PRIMARY KEY NOT NULL,
            password TEXT NOT NULL,
            roles TEXT,
            active NUMERIC NOT NULL,
            created TEXT NOT NULL
        )"#,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

async fn init_samples(pool: &Pool<Sqlite>) {
    let version = "0.0.1";
    let output_regex = ".*";
    let timeout = Duration::new(5, 0);
    let name = "uptime";

    let uptime_linux = Script {
        id: Uuid::new_v4(),
        name: name.to_string(),
        version: version.to_string(),
        output_regex: output_regex.to_string(),
        labels: vec!["linux".to_string(), "sample1".to_string()],
        timeout,
        script_content: r#"uptime -p"#.into(),
    };
    let uptime_mac = Script {
        id: Uuid::new_v4(),
        name: name.to_string(),
        version: version.to_string(),
        output_regex: output_regex.to_string(),
        labels: vec!["mac".to_string(), "sample3".to_string()],
        timeout,
        script_content: r#"uptime"#.into(),
    };

    let name = "os_version".to_string();

    let os_version_linux = Script {
        id: Uuid::new_v4(),
        name: name.to_string(),
        version: version.to_string(),
        output_regex: output_regex.to_string(),
        labels: vec!["linux".to_string(), "sample2".to_string()],
        timeout,
        script_content: r#"cat /etc/os-release"#.into(),
    };
    let os_version_mac = Script {
        id: Uuid::new_v4(),
        name: name.to_string(),
        version: version.to_string(),
        output_regex: output_regex.to_string(),
        labels: vec!["mac".to_string(), "sample4".to_string()],
        timeout,
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
            target: schedule::Target::Attributes(vec![s.labels[0].clone()]),
            timer: schedule::Timer::Cron("* * * * *".into()),
            active: true,
        };
        let Ok(sched_res) = sched
            .clone()
            .insert_into_db(pool.acquire().await.unwrap())
            .await
        else {
            warn!(
                    "DB init: sample schedule for script {} version {} with attributes {} could not be loaded",
                    s.name, s.version, sched.attributes()
                );
            continue;
        };

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
        target: schedule::Target::Attributes(vec![uptime_linux.labels[0].clone()]),
        timer: schedule::Timer::Timestamp(Utc::now()),
        active: true,
    };

    let Ok(sched_res) = sched
        .clone()
        .insert_into_db(pool.acquire().await.unwrap())
        .await
    else {
        warn!(
                    "DB init: sample schedule for script {} version {} with attributes {} could not be loaded",
                    uptime_linux.name, uptime_linux.version, sched.attributes()
                );
        return;
    };
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

async fn init_main_user(pool: &Pool<Sqlite>, email: EmailAddress, password: String) {
    let hashed_pw = hash_password(password.as_bytes()).unwrap();
    let new_user = User {
        id: Uuid::new_v4(),
        email,
        password: hashed_pw,
        roles: vec!["admin".to_string()],
        active: true,
        created: Utc::now(),
    };
    let user_res = match new_user.insert_into_db(pool.acquire().await.unwrap()).await {
        Ok(r) => r,
        Err(e) => {
            error!("DB init: main user could not be loaded. Error:\n{e}");
            return;
        }
    };
    if user_res.rows_affected() > 0 {
        info!("DB init: main user loaded");
    } else {
        error!("DB init: main user could not be loaded");
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
        init_database(&pool, None).await.unwrap();

        let tables = query("PRAGMA table_list;")
            .fetch_all(&mut *pool.acquire().await.unwrap())
            .await
            .unwrap();
        assert_eq!(tables.len(), 7);

        // run again to check already-present branch
        init_database(
            &pool,
            Some((
                EmailAddress::from_str("test@test.com").unwrap(),
                "1234".into(),
            )),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_update_text_field_error() {
        registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()))
            .with(fmt::layer())
            .try_init()
            .unwrap_or(());

        let pool = create_database("sqlite::memory:").await.unwrap();
        init_database(&pool, None).await.unwrap();

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
        init_database(&pool, None).await.unwrap();

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
