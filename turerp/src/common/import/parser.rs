//! CSV and JSON parsing helpers for bulk import

use csv::StringRecord;
use serde::de::DeserializeOwned;

use crate::common::import::model::{ImportError, ImportFormat};
use crate::error::ApiError;

/// Parse raw bytes into rows based on format
pub fn parse_rows<T: DeserializeOwned>(
    data: &[u8],
    format: ImportFormat,
) -> Result<Vec<(usize, T)>, ApiError> {
    match format {
        ImportFormat::Csv => parse_csv(data),
        ImportFormat::Json => parse_json(data),
    }
}

/// Parse CSV data into typed rows
fn parse_csv<T: DeserializeOwned>(data: &[u8]) -> Result<Vec<(usize, T)>, ApiError> {
    let mut reader = csv::Reader::from_reader(data);
    let headers = reader
        .headers()
        .map_err(|e| ApiError::Validation(format!("CSV header error: {}", e)))?
        .clone();

    let mut results = Vec::new();
    let mut row_num = 1; // Start at 1 (header is row 0)

    for record in reader.records() {
        row_num += 1;
        let record = record.map_err(|e| {
            ApiError::Validation(format!("CSV parse error at row {}: {}", row_num, e))
        })?;
        let value = deserialize_record::<T>(&headers, &record, row_num)?;
        results.push((row_num, value));
    }

    Ok(results)
}

/// Deserialize a single CSV record using serde
fn deserialize_record<T: DeserializeOwned>(
    headers: &StringRecord,
    record: &StringRecord,
    row_num: usize,
) -> Result<T, ApiError> {
    if headers.len() != record.len() {
        return Err(ApiError::Validation(format!(
            "Row {}: column count mismatch (expected {}, got {})",
            row_num,
            headers.len(),
            record.len()
        )));
    }

    let mut map = serde_json::Map::new();
    for (i, header) in headers.iter().enumerate() {
        let value = record
            .get(i)
            .map(|v| serde_json::Value::String(v.to_string()))
            .unwrap_or(serde_json::Value::Null);
        map.insert(header.to_string(), value);
    }

    let json_value = serde_json::Value::Object(map);
    serde_json::from_value(json_value).map_err(|e| {
        ApiError::Validation(format!(
            "Row {}: failed to deserialize record: {}",
            row_num, e
        ))
    })
}

/// Parse JSON array data into typed rows
fn parse_json<T: DeserializeOwned>(data: &[u8]) -> Result<Vec<(usize, T)>, ApiError> {
    let json_value: serde_json::Value = serde_json::from_slice(data)
        .map_err(|e| ApiError::Validation(format!("JSON parse error: {}", e)))?;

    let array = json_value
        .as_array()
        .ok_or_else(|| ApiError::Validation("JSON data must be an array".to_string()))?;

    let mut results = Vec::new();
    for (i, item) in array.iter().enumerate() {
        let row_num = i + 1;
        let value = serde_json::from_value(item.clone()).map_err(|e| {
            ApiError::Validation(format!(
                "Row {}: failed to deserialize JSON item: {}",
                row_num, e
            ))
        })?;
        results.push((row_num, value));
    }

    Ok(results)
}

/// Build a CSV template with the given headers
pub fn build_csv_template(headers: &[&str]) -> Result<Vec<u8>, ApiError> {
    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(headers)
        .map_err(|e| ApiError::Internal(format!("CSV writer error: {}", e)))?;
    wtr.into_inner()
        .map_err(|e| ApiError::Internal(format!("CSV writer error: {}", e)))
}

/// Build a JSON template (empty array with example object)
pub fn build_json_template(example: serde_json::Value) -> Result<Vec<u8>, ApiError> {
    let arr = serde_json::json!([example]);
    serde_json::to_vec_pretty(&arr)
        .map_err(|e| ApiError::Internal(format!("JSON serialization error: {}", e)))
}

/// Collect all parse errors into a single result
pub fn collect_parse_errors<T>(
    results: Vec<Result<(usize, T), ApiError>>,
) -> (Vec<(usize, T)>, Vec<ImportError>) {
    let mut rows = Vec::new();
    let mut errors = Vec::new();

    for result in results {
        match result {
            Ok((row, value)) => rows.push((row, value)),
            Err(e) => errors.push(ImportError {
                row: 0,
                field: None,
                message: e.to_string(),
            }),
        }
    }

    (rows, errors)
}

/// Write records to CSV bytes
pub fn write_csv_records(headers: &[&str], records: Vec<Vec<String>>) -> Result<Vec<u8>, ApiError> {
    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(headers)
        .map_err(|e| ApiError::Internal(format!("CSV writer error: {}", e)))?;
    for record in records {
        wtr.write_record(&record)
            .map_err(|e| ApiError::Internal(format!("CSV writer error: {}", e)))?;
    }
    wtr.into_inner()
        .map_err(|e| ApiError::Internal(format!("CSV writer error: {}", e)))
}

/// Write records to JSON bytes
pub fn write_json_records<T: Serialize>(records: &[T]) -> Result<Vec<u8>, ApiError> {
    serde_json::to_vec_pretty(records)
        .map_err(|e| ApiError::Internal(format!("JSON serialization error: {}", e)))
}

use serde::Serialize;

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    struct TestRow {
        code: String,
        name: String,
    }

    #[test]
    fn test_parse_csv() {
        let data = b"code,name\nP001,Product 1\nP002,Product 2";
        let rows = parse_csv::<TestRow>(data).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].1.code, "P001");
        assert_eq!(rows[0].0, 2); // row number
    }

    #[test]
    fn test_parse_json() {
        let data = br#"[{"code":"P001","name":"Product 1"},{"code":"P002","name":"Product 2"}]"#;
        let rows = parse_json::<TestRow>(data).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].1.code, "P001");
    }

    #[test]
    fn test_csv_template() {
        let template = build_csv_template(&["code", "name", "price"]).unwrap();
        let s = String::from_utf8(template).unwrap();
        assert!(s.contains("code"));
        assert!(s.contains("name"));
        assert!(s.contains("price"));
    }

    #[test]
    fn test_csv_mismatch_columns() {
        let data = b"code,name\nP001";
        let result = parse_csv::<TestRow>(data);
        assert!(result.is_err());
    }
}
