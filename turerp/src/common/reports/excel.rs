//! Excel report generation using rust_xlsxwriter

use super::{ReportError, ReportRequest};

pub fn generate_excel(request: &ReportRequest) -> Result<Vec<u8>, ReportError> {
    let params = &request.parameters;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Title row
    let title_format = workbook
        .add_format()
        .set_bold()
        .set_font_size(14)
        .set_font_color(0x1F4E79);
    worksheet.write_with_format(0, 0, &request.title, &title_format)?;

    // Headers
    let headers = params
        .get("headers")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let header_format = workbook
        .add_format()
        .set_bold()
        .set_background_color(0xD9E1F2)
        .set_border_bottom(rust_xlsxwriter::FormatBorder::Thin);

    let mut max_col = 0u16;
    for (col, header) in headers.iter().enumerate() {
        if let Some(h) = header.as_str() {
            worksheet.write_with_format(2, col as u16, h, &header_format)?;
            max_col = max_col.max(col as u16);
        }
    }

    // Data rows
    let rows = params
        .get("rows")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for (row_idx, row) in rows.iter().enumerate() {
        let row_num = (row_idx + 3) as u32;
        if let Some(cells) = row.as_array() {
            for (col_idx, cell) in cells.iter().enumerate() {
                let col = col_idx as u16;
                if let Some(val) = cell.as_str() {
                    worksheet.write(row_num, col, val)?;
                } else if let Some(num) = cell.as_f64() {
                    worksheet.write(row_num, col, num)?;
                } else if let Some(num) = cell.as_i64() {
                    worksheet.write(row_num, col, num as i32)?;
                } else if let Some(b) = cell.as_bool() {
                    worksheet.write(row_num, col, b)?;
                }
            }
        }
    }

    // Auto-filter and freeze
    if max_col > 0 && !rows.is_empty() {
        let last_row = (rows.len() + 2) as u32;
        worksheet.autofilter(2, 0, last_row, max_col)?;
    }
    worksheet.freeze_panes(3, 0)?;

    workbook
        .save_to_buffer()
        .map_err(|e| ReportError::GenerationFailed(format!("Excel generation failed: {}", e)))
}
