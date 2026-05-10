//! PDF report generation using printpdf

use super::{ReportError, ReportRequest};
use printpdf::{
    BuiltinFont, Mm, Op, PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt, TextItem,
};

fn text_op(text: &str, size: f32, x: Mm, y: Mm, font: BuiltinFont) -> Vec<Op> {
    vec![
        Op::StartTextSection,
        Op::SetFont {
            font: PdfFontHandle::Builtin(font),
            size: Pt(size),
        },
        Op::SetTextCursor {
            pos: Point::new(x, y),
        },
        Op::ShowText {
            items: vec![TextItem::Text(text.to_string())],
        },
        Op::EndTextSection,
    ]
}

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

    let mut ops = Vec::new();

    // Title
    ops.extend(text_op(
        &request.title,
        18.0,
        Mm(10.0),
        Mm(270.0),
        BuiltinFont::HelveticaBold,
    ));

    // Invoice info
    let mut y = 255.0;
    ops.extend(text_op(
        &format!("Invoice No: {}", invoice_no),
        12.0,
        Mm(10.0),
        Mm(y),
        BuiltinFont::Helvetica,
    ));
    y -= 8.0;
    ops.extend(text_op(
        &format!("Date: {}", date),
        12.0,
        Mm(10.0),
        Mm(y),
        BuiltinFont::Helvetica,
    ));
    y -= 8.0;
    ops.extend(text_op(
        &format!("Customer: {}", customer),
        12.0,
        Mm(10.0),
        Mm(y),
        BuiltinFont::Helvetica,
    ));
    y -= 12.0;

    // Items table header
    if !items.is_empty() {
        ops.extend(text_op(
            "Description",
            11.0,
            Mm(10.0),
            Mm(y),
            BuiltinFont::HelveticaBold,
        ));
        ops.extend(text_op(
            "Qty",
            11.0,
            Mm(90.0),
            Mm(y),
            BuiltinFont::HelveticaBold,
        ));
        ops.extend(text_op(
            "Price",
            11.0,
            Mm(120.0),
            Mm(y),
            BuiltinFont::HelveticaBold,
        ));
        ops.extend(text_op(
            "Total",
            11.0,
            Mm(150.0),
            Mm(y),
            BuiltinFont::HelveticaBold,
        ));
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

            ops.extend(text_op(desc, 10.0, Mm(10.0), Mm(y), BuiltinFont::Helvetica));
            ops.extend(text_op(
                &format!("{:.2}", qty),
                10.0,
                Mm(90.0),
                Mm(y),
                BuiltinFont::Helvetica,
            ));
            ops.extend(text_op(
                &format!("{:.2}", price),
                10.0,
                Mm(120.0),
                Mm(y),
                BuiltinFont::Helvetica,
            ));
            ops.extend(text_op(
                &format!("{:.2}", line_total),
                10.0,
                Mm(150.0),
                Mm(y),
                BuiltinFont::Helvetica,
            ));
            y -= 7.0;
        }
        y -= 8.0;
    }

    // Totals
    ops.extend(text_op(
        &format!("Subtotal: {:.2}", subtotal),
        12.0,
        Mm(10.0),
        Mm(y),
        BuiltinFont::Helvetica,
    ));
    y -= 8.0;
    ops.extend(text_op(
        &format!("Tax: {:.2}", tax),
        12.0,
        Mm(10.0),
        Mm(y),
        BuiltinFont::Helvetica,
    ));
    y -= 8.0;
    ops.extend(text_op(
        &format!("Total: {:.2}", total),
        14.0,
        Mm(10.0),
        Mm(y),
        BuiltinFont::HelveticaBold,
    ));

    let page = PdfPage::new(Mm(210.0), Mm(297.0), ops);

    let mut doc = PdfDocument::new(&request.title);
    doc.pages.push(page);

    let mut warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);

    if warnings
        .iter()
        .any(|w| w.msg.contains("error") || w.msg.contains("Error") || w.msg.contains("ERROR"))
    {
        return Err(ReportError::GenerationFailed(
            "PDF generation produced errors".to_string(),
        ));
    }

    Ok(bytes)
}
