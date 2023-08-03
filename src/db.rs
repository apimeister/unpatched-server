use std::str::FromStr;

use crate::{
    new_id,
    schedule::Schedule,
    script::{get_script_id_by_name_from_db, Script},
};
use sqlx::{
    pool::PoolConnection,
    query,
    sqlite::{SqliteConnectOptions, SqliteQueryResult},
    Pool, Row, Sqlite, SqlitePool,
};
use tracing::{debug, error, info, warn};

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
/// * scheduling table
/// * sample scripts
/// * sample schedules
///
/// More info: [DB.md](DB.md)
pub async fn init_database(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    create_hosts_table(pool.acquire().await?).await?;
    create_scripts_table(pool.acquire().await?).await?;
    create_executions_table(pool.acquire().await?).await?;
    create_scheduling_table(pool.acquire().await?).await?;
    let tables = query("PRAGMA table_list;")
        .fetch_all(&mut *pool.acquire().await.unwrap())
        .await?;
    info!("DB Init: created {} tables", tables.len());
    let script_count = query("SELECT count(id) as id_count FROM scripts")
        .fetch_one(&mut *pool.acquire().await?)
        .await?;
    if script_count.get::<i32, _>("id_count") == 0 {
        init_scripts_table(pool).await
    } else {
        debug!("DB init: script table has scripts, samples not loaded");
    }

    let schedule_count = query("SELECT count(id) as id_count FROM scheduling")
        .fetch_one(&mut *pool.acquire().await?)
        .await?;
    if schedule_count.get::<i32, _>("id_count") == 0 {
        match init_scheduling_table(pool).await {
            Some(_) => info!("DB init: sample schedules loaded"),
            None => warn!("DB init: Could not initialize sample schedules, starting with empty schedule table")
        }
    } else {
        debug!("DB init: scheduling table has schedules, samples not loaded");
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
/// | last_pong | TEXT | last checkin from agent
async fn create_hosts_table(mut connection: PoolConnection<Sqlite>) -> Result<(), sqlx::Error> {
    let _res = query(
        r#"CREATE TABLE IF NOT EXISTS 
        hosts(
            id TEXT PRIMARY KEY NOT NULL,
            alias TEXT,
            attributes TEXT,
            ip TEXT,
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
/// | request | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS.SSS")
/// | response | TEXT | as ISO8601 string ("YYYY-MM-DD HH:MM:SS.SSS")
/// | host_id | TEXT | uuid
/// | script_id | TEXT | uuid
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
            script_id TEXT,
            output TEXT
        )"#,
    )
    .execute(&mut *connection)
    .await?;
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
    let _res = query(
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

pub async fn update_text_field(
    id: String,
    column: &str,
    data: String,
    table: &str,
    mut connection: PoolConnection<Sqlite>,
) -> SqliteQueryResult {
    let stmt = format!("UPDATE {} SET {} = ? WHERE id = ?", table, column);
    match query(&stmt)
        .bind(data)
        .bind(id.clone())
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

pub async fn update_timestamp(
    id: String,
    column: &str,
    table: &str,
    mut connection: PoolConnection<Sqlite>,
) -> SqliteQueryResult {
    let stmt = format!("UPDATE {} SET {} = datetime() WHERE id = ?", table, column);
    match query(&stmt)
        .bind(id.clone())
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
            .init();
        let pool = create_database("sqlite::memory:").await.unwrap();
        init_database(&pool).await.unwrap();
        // let x = query("SELECT count(id) as id FROM scripts")
        let tables = query("PRAGMA table_list;")
            .fetch_all(&mut *pool.acquire().await.unwrap())
            .await
            .unwrap();
        assert_eq!(tables.len(), 6);
        // run again to check already-present branch
        init_database(&pool).await.unwrap();
    }
}
