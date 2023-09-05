use axum::response::IntoResponse;
use hyper::{HeaderMap, StatusCode, Uri};

pub async fn web_page(uri: Uri) -> impl IntoResponse {
    let header = HeaderMap::new();
    let path = uri.path();
    println!("got req: {path}");
    let maybe_file = crate::WEBPAGE.get_file(path);
    match maybe_file {
        Some(file) => {
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
