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
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Script {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub output_regex: String,
    pub labels: Vec<String>,
    pub timeout: String,
    pub script_content: String,
}

impl Script {
    /// Insert into or Replace `Script` in scripts table in SQLite database
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
    pub async fn insert_into_db(self, mut connection: PoolConnection<Sqlite>) -> SqliteQueryResult {
        let q = r#"REPLACE INTO scripts( id, name, version, output_regex, labels, timeout, script_content ) VALUES ( ?, ?, ?, ?, ?, ?, ? )"#;
        query(q)
            .bind(self.id.to_string())
            .bind(self.name)
            .bind(self.version)
            .bind(self.output_regex)
            .bind(serde_json::to_string(&self.labels).unwrap())
            .bind(self.timeout)
            .bind(self.script_content)
            .execute(&mut *connection)
            .await
            .unwrap()
    }
    /// return labels as comma-seperated `String`
    pub fn labels(&self) -> String {
        self.labels.join(",")
    }
}

impl From<SqliteRow> for Script {
    fn from(s: SqliteRow) -> Self {
        Script {
            id: s.get::<String, _>("id").parse().unwrap(),
            name: s.get::<String, _>("name"),
            version: s.get::<String, _>("version"),
            output_regex: s.get::<String, _>("output_regex"),
            labels: serde_json::from_str(&s.get::<String, _>("labels")).unwrap(),
            timeout: s.get::<String, _>("timeout"),
            script_content: s.get::<String, _>("script_content"),
        }
    }
}

/// API to get all scripts
pub async fn get_scripts_api(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let script_vec = get_scripts_from_db(None, pool.acquire().await.unwrap()).await;
    Json(script_vec)
}

/// API to get one script
pub async fn get_one_script_api(
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    let script_vec = get_scripts_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    Json(script_vec.first().cloned())
}

/// API to delete all scripts
pub async fn delete_scripts_api(State(pool): State<SqlitePool>) -> impl IntoResponse {
    delete_scripts_from_db(None, pool.acquire().await.unwrap()).await
}

/// API to delete one script
pub async fn delete_one_script_api(
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> impl IntoResponse {
    let filter = format!("id='{id}'",);
    delete_scripts_from_db(Some(&filter), pool.acquire().await.unwrap()).await
}

/// API to create a new script
pub async fn post_scripts_api(
    State(pool): State<SqlitePool>,
    Json(payload): Json<Script>,
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

pub async fn get_scripts_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> Vec<Script> {
    let stmt = if let Some(f) = filter {
        format!("SELECT * FROM scripts WHERE {f}")
    } else {
        "SELECT * FROM scripts".into()
    };
    let scripts = match query(&stmt).fetch_all(&mut *connection).await {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    scripts.into_iter().map(|s| s.into()).collect()
}

pub async fn delete_scripts_from_db(
    filter: Option<&str>,
    mut connection: PoolConnection<Sqlite>,
) -> StatusCode {
    let stmt = if let Some(f) = filter {
        format!("DELETE FROM scripts WHERE {f}")
    } else {
        "DELETE FROM scripts".into()
    };
    let res = query(&stmt).execute(&mut *connection).await;
    if res.is_err() {
        error!("{res:?}");
        StatusCode::FORBIDDEN
    } else {
        StatusCode::OK
    }
}

#[allow(dead_code)]
// FIXME: make undead
pub async fn update_text_field(
    id: Uuid,
    column: &str,
    data: String,
    connection: PoolConnection<Sqlite>,
) -> SqliteQueryResult {
    crate::db::update_text_field(id, column, data, "scripts", connection).await
}

pub async fn count_rows(connection: PoolConnection<Sqlite>) -> Result<i64, sqlx::Error> {
    crate::db::count_rows("scripts", connection).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_database, init_database};
    use tracing_subscriber::{
        fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
    };

    #[tokio::test]
    async fn test_scripts() {
        registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()))
            .with(fmt::layer())
            .try_init()
            .unwrap_or(());

        let pool = create_database("sqlite::memory:").await.unwrap();

        init_database(&pool).await.unwrap();
        let scripts = get_scripts_from_db(None, pool.acquire().await.unwrap()).await;
        assert_eq!(scripts.len(), 4);

        let mut script = Script::default();
        let i1 = script
            .clone()
            .insert_into_db(pool.acquire().await.unwrap())
            .await;
        assert_eq!(i1.rows_affected(), 1);
        script.id = Uuid::new_v4();
        let i2 = script
            .clone()
            .insert_into_db(pool.acquire().await.unwrap())
            .await;
        assert_eq!(i2.rows_affected(), 1);

        let scripts = count_rows(pool.acquire().await.unwrap()).await.unwrap();
        assert_eq!(scripts, 6);

        let err_scripts =
            get_scripts_from_db(Some("this-doesnt-work"), pool.acquire().await.unwrap()).await;
        assert_eq!(err_scripts.len(), 0);

        let _upd = update_text_field(
            script.id,
            "timeout",
            "100s".to_string(),
            pool.acquire().await.unwrap(),
        )
        .await;
        let scripts = get_scripts_from_db(
            Some(format!("id='{}'", script.id).as_str()),
            pool.acquire().await.unwrap(),
        )
        .await;
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].timeout, "100s");

        let single_del = delete_scripts_from_db(
            Some(format!("id='{}'", script.id).as_str()),
            pool.acquire().await.unwrap(),
        )
        .await;
        assert_eq!(single_del, axum::http::StatusCode::OK);
        let scripts = count_rows(pool.acquire().await.unwrap()).await.unwrap();
        assert_eq!(scripts, 5);

        let del_fail =
            delete_scripts_from_db(Some("this_doesnt_work"), pool.acquire().await.unwrap()).await;
        assert_eq!(del_fail, axum::http::StatusCode::FORBIDDEN);

        let del = delete_scripts_from_db(None, pool.acquire().await.unwrap()).await;
        assert_eq!(del, axum::http::StatusCode::OK);
        let scripts = count_rows(pool.acquire().await.unwrap()).await.unwrap();
        assert_eq!(scripts, 0);
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
        let new_script = Script::default();
        let api_post =
            post_scripts_api(axum::extract::State(pool.clone()), Json(new_script.clone()))
                .await
                .into_response();
        assert_eq!(api_post.status(), axum::http::StatusCode::CREATED);

        let api_get_all = get_scripts_api(axum::extract::State(pool.clone()))
            .await
            .into_response();
        assert_eq!(api_get_all.status(), axum::http::StatusCode::OK);

        let api_get_one = get_one_script_api(
            axum::extract::Path(new_script.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_get_one.status(), axum::http::StatusCode::OK);

        let api_del_one = delete_one_script_api(
            axum::extract::Path(new_script.id),
            axum::extract::State(pool.clone()),
        )
        .await
        .into_response();
        assert_eq!(api_del_one.status(), axum::http::StatusCode::OK);

        let api_del_all = delete_scripts_api(axum::extract::State(pool.clone()))
            .await
            .into_response();
        assert_eq!(api_del_all.status(), axum::http::StatusCode::OK);
    }
}
