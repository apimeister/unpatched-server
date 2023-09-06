use axum::{
    http::{header, StatusCode},
    response::{Html, IntoResponse},
};
use headers::{HeaderMap, HeaderValue};

use crate::{jwt::Claims, API_YAML};

/// load swagger gui
pub async fn api_ui(_claims: Claims) -> impl IntoResponse {
    let html = r#"<!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="utf-8" />
      <meta name="viewport" content="width=device-width, initial-scale=1" />
      <meta name="description" content="Unpatched Server" />
      <title>Unpatched Server - SwaggerUI</title>
      <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5.3.1/swagger-ui.css" />
      <link rel="icon" href="/bandaid.svg">
    </head>
    <body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5.3.1/swagger-ui-bundle.js" crossorigin></script>
    <script>
      window.onload = () => {
        window.ui = SwaggerUIBundle({
          url: '/api/api.yaml',
          dom_id: '#swagger-ui',
        });
      };
    </script>
    </body>
    </html>"#;
    (StatusCode::OK, Html(html))
}

/// load api.yaml
pub async fn api_def(_claims: Claims) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET"),
    );
    (StatusCode::OK, headers, API_YAML)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_api_def() {
        let claims = Claims::default();
        let api_def = api_def(claims).await.into_response();
        assert_eq!(api_def.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_api_ui() {
        let claims = Claims::default();
        let api_ui = api_ui(claims).await.into_response();
        assert_eq!(api_ui.status(), axum::http::StatusCode::OK);
    }
}
