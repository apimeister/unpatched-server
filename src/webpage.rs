use axum::{http::HeaderValue, response::IntoResponse};
use hyper::{HeaderMap, StatusCode, Uri};

pub async fn web_page(uri: Uri) -> impl IntoResponse {
    let mut header = HeaderMap::new();
    let path = uri.path().strip_prefix('/').unwrap();
    let path = if path.is_empty() {
        "index.html".to_string()
    } else if path.ends_with('/') {
        format!("{path}/index.html")
    } else {
        path.to_string()
    };
    println!("got req: {path}");
    // fix content type
    if path.ends_with(".html") {
        header.insert("Content-Type", HeaderValue::from_static("text/html"));
    } else {
        header.insert("Content-Type", HeaderValue::from_static("text/plain"));
    }
    println!("contains {}",crate::WEBPAGE.contains(&path));
    println!("path {:?}",crate::WEBPAGE.get_entry(&path));
    let maybe_file = crate::WEBPAGE.get_file(path);
    match maybe_file {
        Some(file) => {
            println!("found");
            return (StatusCode::OK, header, file.contents_utf8().unwrap());
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
        }
    }
}
