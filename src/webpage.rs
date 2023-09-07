use axum::{
    http::HeaderValue,
    response::{IntoResponse, Response},
};
use hyper::{header::CONTENT_TYPE, HeaderMap, StatusCode, Uri};

pub async fn web_page(uri: Uri) -> Response {
    let mut is_text_return = true;
    let mut header = HeaderMap::new();
    let path = uri.path().strip_prefix('/').unwrap();
    let path = if path.is_empty() {
        "index.html".to_string()
    } else if path.ends_with('/') {
        format!("{path}/index.html")
    } else {
        path.to_string()
    };
    tracing::trace!("got req: {path}");
    // fix content type
    if path.ends_with(".html") {
        header.insert(CONTENT_TYPE, HeaderValue::from_static("text/html"));
    } else if path.ends_with(".css") {
        header.insert(CONTENT_TYPE, HeaderValue::from_static("text/css"));
    } else if path.ends_with(".js") {
        header.insert(CONTENT_TYPE, HeaderValue::from_static("text/javascript"));
    } else if path.ends_with(".svg") {
        header.insert(CONTENT_TYPE, HeaderValue::from_static("image/svg+xml"));
    // } else {
    // header.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    } else {
        header.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
        is_text_return = false;
    }
    let maybe_file = crate::WEBPAGE.get_file(&path);
    match maybe_file {
        Some(file) => {
            if is_text_return {
                (StatusCode::OK, header, file.contents_utf8().unwrap()).into_response()
            } else {
                (StatusCode::OK, header, file.contents()).into_response()
            }
        }
        None => {
            // try as path
            let path = format!("{path}/index.html");
            header.insert(CONTENT_TYPE, HeaderValue::from_static("text/html"));
            let maybe_file = crate::WEBPAGE.get_file(path);
            match maybe_file {
                Some(file) => {
                    return (StatusCode::OK, header, file.contents_utf8().unwrap()).into_response();
                }
                None => {
                    return (
                        StatusCode::OK,
                        header,
                        crate::WEBPAGE
                            .get_file("404.html")
                            .unwrap()
                            .contents_utf8()
                            .unwrap(),
                    )
                        .into_response()
                }
            }
        }
    }
}
