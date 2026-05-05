//! PDF report generation using printpdf

use super::{ReportError, ReportRequest};
use printpdf::{BuiltinFont, Mm, PdfConformance, PdfDocument};

pub fn generate_invoice_pdf(request: &ReportRequest) -> Result<Vec<u8>, ReportError> {
    let params = &request.parameters;

    let invoice_no = params
        .get("invoice_no")
        .and_then(|v| v.as_str())
        .unwrap_or("N/A");
    let customer = params
        .get("customer")
        .and_then(|v| v.as_str())
        .unwrap_or("N/A");
    let date = params.get("date").and_then(|v| v.as_str()).unwrap_or("");
    let total = params.get("total").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let subtotal = params
        .get("subtotal")
        .and_then(|v| v.as_f64())
        .unwrap_or(total);
    let tax = params.get("tax").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let items = params
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let (mut doc, page1, layer1) =
        PdfDocument::new(&request.title, Mm(210.0), Mm(297.0), "Layer 1");
    doc = doc.with_conformance(PdfConformance::Custom(printpdf::CustomPdfConformance {
        requires_icc_profile: false,
        requires_xmp_metadata: false,
        ..Default::default()
    }));

    let current_layer = doc.get_page(page1).get_layer(layer1);
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| ReportError::GenerationFailed(format!("Font error: {}", e)))?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| ReportError::GenerationFailed(format!("Font error: {}", e)))?;

    // Title
    current_layer.use_text(&request.title, 18.0, Mm(10.0), Mm(270.0), &font_bold);

    // Invoice info
    let mut y = 255.0;
    current_layer.use_text(
        format!("Invoice No: {}", invoice_no),
        12.0,
        Mm(10.0),
        Mm(y),
        &font,
    );
    y -= 8.0;
    current_layer.use_text(format!("Date: {}", date), 12.0, Mm(10.0), Mm(y), &font);
    y -= 8.0;
    current_layer.use_text(
        format!("Customer: {}", customer),
        12.0,
        Mm(10.0),
        Mm(y),
        &font,
    );
    y -= 12.0;

    // Items table header
    if !items.is_empty() {
        current_layer.use_text("Description", 11.0, Mm(10.0), Mm(y), &font_bold);
        current_layer.use_text("Qty", 11.0, Mm(90.0), Mm(y), &font_bold);
        current_layer.use_text("Price", 11.0, Mm(120.0), Mm(y), &font_bold);
        current_layer.use_text("Total", 11.0, Mm(150.0), Mm(y), &font_bold);
        y -= 8.0;

        for item in &items {
            let desc = item
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let qty = item.get("quantity").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let price = item.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let line_total = item
                .get("total")
                .and_then(|v| v.as_f64())
                .unwrap_or(qty * price);

            current_layer.use_text(desc, 10.0, Mm(10.0), Mm(y), &font);
            current_layer.use_text(format!("{:.2}", qty), 10.0, Mm(90.0), Mm(y), &font);
            current_layer.use_text(format!("{:.2}", price), 10.0, Mm(120.0), Mm(y), &font);
            current_layer.use_text(format!("{:.2}", line_total), 10.0, Mm(150.0), Mm(y), &font);
            y -= 7.0;
        }
        y -= 8.0;
    }

    // Totals
    current_layer.use_text(
        format!("Subtotal: {:.2}", subtotal),
        12.0,
        Mm(10.0),
        Mm(y),
        &font,
    );
    y -= 8.0;
    current_layer.use_text(format!("Tax: {:.2}", tax), 12.0, Mm(10.0), Mm(y), &font);
    y -= 8.0;
    current_layer.use_text(
        format!("Total: {:.2}", total),
        14.0,
        Mm(10.0),
        Mm(y),
        &font_bold,
    );

    let mut buf = Vec::new();
    doc.save(&mut std::io::BufWriter::new(std::io::Cursor::new(&mut buf)))
        .map_err(|e| ReportError::GenerationFailed(format!("PDF save failed: {}", e)))?;
    Ok(buf)
}
