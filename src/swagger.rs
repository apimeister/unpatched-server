use axum::{
    http::{header, StatusCode},
    response::{Html, IntoResponse},
};
use headers::{HeaderMap, HeaderValue};

pub async fn api_ui() -> impl IntoResponse {
    let html = r#"<!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="utf-8" />
      <meta name="viewport" content="width=device-width, initial-scale=1" />
      <meta name="description" content="Unpatched Server" />
      <title>Unpatched Server - SwaggerUI</title>
      <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5.3.1/swagger-ui.css" />
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

pub async fn api_def() -> impl IntoResponse {
    let spec = std::fs::read_to_string("api.yaml").unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET"),
    );
    (StatusCode::OK, headers, spec)
}
