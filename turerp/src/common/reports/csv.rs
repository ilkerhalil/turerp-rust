//! CSV report generation using the csv crate

use super::{ReportError, ReportRequest};

pub fn generate_csv(request: &ReportRequest) -> Result<Vec<u8>, ReportError> {
    let params = &request.parameters;
    let mut wtr = csv::Writer::from_writer(vec![]);

    // Write BOM for Excel compatibility
    let mut data: Vec<u8> = vec![0xEF, 0xBB, 0xBF];

    // Headers
    let headers = params
        .get("headers")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let header_strings: Vec<String> = headers
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    if !header_strings.is_empty() {
        wtr.write_record(&header_strings)
            .map_err(|e| ReportError::Io(e.to_string()))?;
    }

    // Data rows
    let rows = params
        .get("rows")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for row in &rows {
        if let Some(cells) = row.as_array() {
            let record: Vec<String> = cells
                .iter()
                .map(|c| {
                    if let Some(s) = c.as_str() {
                        s.to_string()
                    } else if let Some(n) = c.as_f64() {
                        format!("{:.2}", n)
                    } else if let Some(n) = c.as_i64() {
                        n.to_string()
                    } else if let Some(b) = c.as_bool() {
                        b.to_string()
                    } else {
                        c.to_string()
                    }
                })
                .collect();
            wtr.write_record(&record)
                .map_err(|e| ReportError::Io(e.to_string()))?;
        }
    }

    data.extend(wtr.into_inner().map_err(|e| ReportError::Io(e.to_string()))?);
    Ok(data)
}

/// Streaming CSV writer for large datasets.
/// Writes record by record to avoid loading everything into memory.
pub struct StreamingCsvWriter {
    writer: csv::Writer<Vec<u8>>,
}

impl StreamingCsvWriter {
    pub fn new() -> Self {
        let mut writer = csv::Writer::from_writer(vec![]);
        // Write BOM
        let _ = writer.write("\u{FEFF}".as_bytes());
        Self { writer }
    }

    pub fn write_headers(&mut self, headers: &[&str]) -> Result<(), ReportError> {
        self.writer
            .write_record(headers)
            .map_err(|e| ReportError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn write_row(&mut self,
        cells: &[&str],
    ) -> Result<(), ReportError> {
        self.writer
            .write_record(cells)
            .map_err(|e| ReportError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn finish(mut self) -> Result<Vec<u8>, ReportError> {
        self.writer
            .into_inner()
            .map_err(|e| ReportError::Io(e.to_string()))
    }
}
