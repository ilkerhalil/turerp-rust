//! PDF report generation using genpdf

use super::{ReportError, ReportRequest};

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
    let subtotal = params.get("subtotal").and_then(|v| v.as_f64()).unwrap_or(total);
    let tax = params.get("tax").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let items = params
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let font = genpdf::fonts::BuiltinFont::Courier;
    let font_bold = genpdf::fonts::BuiltinFont::CourierBold;

    let font_family =
        genpdf::fonts::FontFamily::new(font.clone(), font.clone(), font_bold.clone(), font_bold.clone());

    let mut doc = genpdf::Document::new(font_family);
    doc.set_title(&request.title);
    doc.set_minimal_conformance();

    use genpdf::elements::{Break, Paragraph, PaddedElement, Table};
    use genpdf::style::Style;

    // Title
    doc.push(
        Paragraph::new(&request.title).styled(Style::new().bold().with_font_size(18)),
    );
    doc.push(Break::new(1));

    // Invoice info
    doc.push(Paragraph::new(format!("Invoice No: {}", invoice_no)));
    doc.push(Paragraph::new(format!("Date: {}", date)));
    doc.push(Paragraph::new(format!("Customer: {}", customer)));
    doc.push(Break::new(1));

    // Items table
    if !items.is_empty() {
        let mut table = Table::new();
        table.set_header(vec![
            Paragraph::new("Description"),
            Paragraph::new("Qty"),
            Paragraph::new("Price"),
            Paragraph::new("Total"),
        ]);

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

            table.push_row(vec![
                Paragraph::new(desc),
                Paragraph::new(format!("{:.2}", qty)),
                Paragraph::new(format!("{:.2}", price)),
                Paragraph::new(format!("{:.2}", line_total)),
            ]);
        }
        doc.push(table);
        doc.push(Break::new(1));
    }

    // Totals
    doc.push(Paragraph::new(format!("Subtotal: {:.2}", subtotal)));
    doc.push(Paragraph::new(format!("Tax: {:.2}", tax)));
    doc.push(
        Paragraph::new(format!("Total: {:.2}", total)).styled(Style::new().bold()),
    );

    doc.render_to_vec()
        .map_err(|e| ReportError::GenerationFailed(format!("PDF generation failed: {}", e)))
}
