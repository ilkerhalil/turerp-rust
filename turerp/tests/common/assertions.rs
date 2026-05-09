use actix_web::body::to_bytes;
use actix_web::http::StatusCode;
use actix_web::test::TestRequest;

/// Assert HTTP response status code
pub fn assert_status(resp: &actix_web::dev::ServiceResponse, expected: StatusCode) {
    assert_eq!(resp.status(), expected, "Expected status {:?}, got {:?}", expected, resp.status());
}

/// Assert a JSON field value at a dot-separated path
pub async fn assert_json_field(
    resp: actix_web::dev::ServiceResponse,
    path: &str,
    expected: serde_json::Value,
) {
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let mut current = &json;
    for segment in path.split('.') {
        current = current.get(segment).unwrap_or_else(|| panic!("Path '{}' not found in JSON", segment));
    }
    assert_eq!(*current, expected, "JSON field '{}' mismatch", path);
}

/// Assert response contains an integer id and return it
pub async fn assert_id(resp: actix_web::dev::ServiceResponse) -> i64 {
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    json["id"].as_i64().expect("Response JSON missing 'id' field")
}
