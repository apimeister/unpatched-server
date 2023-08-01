use std::str::FromStr;

use crate::{
    new_id,
    schedule::Schedule,
    script::{get_script_id_by_name_from_db, Script},
    SQLITE_DB,
};
use sqlx::{
    pool::PoolConnection, query, sqlite::SqliteConnectOptions, Pool, Row, Sqlite, SqlitePool,
};
use tracing::{debug, info, warn};

/// create database
/// sqlite::memory: - Open an in-memory database.
/// sqlite:data.db - Open the file data.db in the current directory.
/// sqlite://data.db - Open the file data.db in the current directory.
/// sqlite:///data.db - Open the file data.db from the root (/) directory.
/// sqlite://data.db?mode=ro - Open the file data.db for read-only access.
/// types: https://www.sqlite.org/datatype3.html
pub async fn create_datase() -> SqlitePool {
    let connection_options = SqliteConnectOptions::from_str(SQLITE_DB)
        .unwrap()
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(connection_options).await.unwrap();
    let _t = create_data_table(pool.acquire().await.unwrap()).await;
    let _t = create_hosts_table(pool.acquire().await.unwrap()).await;
    let _t = create_scripts_table(pool.acquire().await.unwrap()).await;
    let _t = create_executions_table(pool.acquire().await.unwrap()).await;
    let _t = create_scheduling_table(pool.acquire().await.unwrap()).await;

    let script_count = query("SELECT count(id) as id_count FROM scripts")
        .fetch_one(&mut *pool.acquire().await.unwrap())
        .await
        .unwrap();
    if script_count.get::<i32, _>("id_count") == 0 {
        init_scripts_table(&pool).await
    } else {
        debug!("DB init: script table has scripts, samples not loaded");
    }

    let schedule_count = query("SELECT count(id) as id_count FROM scheduling")
        .fetch_one(&mut *pool.acquire().await.unwrap())
        .await
        .unwrap();
    if schedule_count.get::<i32, _>("id_count") == 0 {
        match init_scheduling_table(&pool).await {
            Some(_) => info!("DB init: sample schedules loaded"),
            None => warn!("DB init: Could not initialize sample schedules, starting with empty schedule table")
        }
    } else {
        debug!("DB init: scheduling table has schedules, samples not loaded");
    }
    pool
}

/// (deprecated) Create Data Table in SQLite Database
///
/// | Name | Type | Comment
/// :--- | :--- | :---
/// id | TEXT | uuid
/// | name | TEXT |
/// | uptime | INTEGER |
/// | os_release | TEXT |
/// | memory | TEXT |
/// | units | TEXT |
async fn create_data_table(mut connection: PoolConnection<Sqlite>) -> Result<(), sqlx::Error> {
    let res = query(
        r#"CREATE TABLE IF NOT EXISTS 
            data(
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT,
                uptime INTEGER,
                os_release TEXT,
                memory TEXT,
                units TEXT
            )"#,
    )
    .execute(&mut *connection)
    .await?;
    if res.rows_affected() > 0 {
        info!("DB init: created data table");
    } else {
        debug!("DB init: data table already present");
    };
    Ok(())
}

/// Create hosts table in SQLite database
///
/// | Name | Type | Comment
/// :--- | :--- | :---
/// | id | TEXT | uuid
/// | alias | TEXT | host alias (name)
/// | attributes | TEXT | host labels
/// | last_pong | TEXT | last checkin from agent
async fn create_hosts_table(mut connection: PoolConnection<Sqlite>) -> Result<(), sqlx::Error> {
    let res = query(
        r#"CREATE TABLE IF NOT EXISTS 
        hosts(
            id TEXT PRIMARY KEY NOT NULL,
            alias TEXT,
            attributes TEXT,
            last_pong TEXT
        )"#,
    )
    .execute(&mut *connection)
    .await?;
    if res.rows_affected() > 0 {
        info!("DB init: created hosts table");
    } else {
        debug!("DB init: hosts table already present");
    };
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
    let res = query(
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
    if res.rows_affected() > 0 {
        info!("DB init: created scripts table");
    } else {
        debug!("DB init: scripts table already present");
    };
    Ok(())
}

/// Create Executions Table in SQLite Database
///
/// | Name | Type | Comment
/// :--- | :--- | :---
/// | id | TEXT | uuid
/// | timestamp | TEXT | timestamp as ISO8601 strings ("YYYY-MM-DD HH:MM:SS.SSS")
/// | host_id | TEXT | uuid
/// | script_id | TEXT | uuid
/// | output | TEXT | return value from script
async fn create_executions_table(
    mut connection: PoolConnection<Sqlite>,
) -> Result<(), sqlx::Error> {
    let res = query(
        r#"CREATE TABLE IF NOT EXISTS 
        executions(
            id TEXT PRIMARY KEY NOT NULL,
            timestamp TEXT,
            host_id TEXT,
            script_id TEXT,
            output TEXT
        )"#,
    )
    .execute(&mut *connection)
    .await?;
    if res.rows_affected() > 0 {
        info!("DB init: created executions table");
    } else {
        debug!("DB init: executions table already present");
    };
    Ok(())
}

/// Create Scheduling Table in SQLite Database
///
/// | Name | Type | Comment
/// :--- | :--- | :---
/// | id | TEXT | uuid
/// | script_id | TEXT | uuid
/// | attributes | TEXT | server label to execute on
/// | cron | TEXT | cron pattern for execution
/// | active | NUMERIC | boolean
async fn create_scheduling_table(
    mut connection: PoolConnection<Sqlite>,
) -> Result<(), sqlx::Error> {
    let res = query(
        r#"CREATE TABLE IF NOT EXISTS 
        scheduling(
            id TEXT PRIMARY KEY NOT NULL,
            script_id TEXT,
            attributes TEXT,
            cron TEXT,
            active NUMERIC
        )"#,
    )
    .execute(&mut *connection)
    .await?;
    if res.rows_affected() > 0 {
        info!("DB init: created scheduling table");
        info!("{res:?}");
    } else {
        debug!("DB init: scheduling table already present");
    };
    Ok(())
}

async fn init_scripts_table(pool: &Pool<Sqlite>) {
    let uptime = Script {
        id: new_id(),
        name: "uptime".into(),
        version: "0.0.1".into(),
        output_regex: ".*".into(),
        labels: "sample,sample2".into(),
        timeout: "5s".into(),
        script_content: r#"uptime -p"#.into(),
    };
    let os_version = Script {
        id: new_id(),
        name: "os_version".into(),
        version: "0.0.1".into(),
        output_regex: ".*".into(),
        labels: "sample,sample2".into(),
        timeout: "5s".into(),
        script_content: r#"cat /etc/os-release"#.into(),
    };
    let uptime_res = uptime.insert_into_db(pool.acquire().await.unwrap()).await;
    let os_ver_res = os_version
        .insert_into_db(pool.acquire().await.unwrap())
        .await;
    if uptime_res.rows_affected() > 0 && os_ver_res.rows_affected() > 0 {
        info!("DB init: sample scripts loaded");
    } else {
        warn!("DB init: Could not initialize sample scripts, starting with empty script table");
    }
}

async fn init_scheduling_table(pool: &Pool<Sqlite>) -> Option<()> {
    let uptime_id =
        get_script_id_by_name_from_db("uptime".into(), pool.acquire().await.unwrap()).await?;
    let os_version_id =
        get_script_id_by_name_from_db("os_version".into(), pool.acquire().await.unwrap()).await?;

    let uptime = Schedule {
        id: new_id(),
        script_id: uptime_id,
        attributes: "attr1,attr2".into(),
        cron: "* * * * *".into(),
        active: true,
    };
    let os_ver = Schedule {
        id: new_id(),
        script_id: os_version_id,
        attributes: "attr1,attr2".into(),
        cron: "* * * * *".into(),
        active: true,
    };
    let uptime_res = uptime.insert_into_db(pool.acquire().await.unwrap()).await;
    let os_ver_res = os_ver.insert_into_db(pool.acquire().await.unwrap()).await;
    if uptime_res.rows_affected() == 0 || os_ver_res.rows_affected() == 0 {
        None
    } else {
        Some(())
    }
}
