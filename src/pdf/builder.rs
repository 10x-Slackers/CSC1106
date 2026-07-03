//! Document-agnostic building blocks shared by every generated PDF.

use genpdf::fonts::{FontData, FontFamily};
use genpdf::style::Style;
use genpdf::{Alignment, Document, Element, Margins, SimplePageDecorator, elements};
use rust_decimal::Decimal;

use super::PdfError;

/// Fonts embedded into the binary at compile time, so PDF generation never
/// depends on the working directory or on font files being present at runtime.
const REGULAR_FONT: &[u8] = include_bytes!("../../assets/fonts/Inter_24pt-Regular.ttf");
const BOLD_FONT: &[u8] = include_bytes!("../../assets/fonts/Inter_24pt-Bold.ttf");

/// Format a decimal amount as currency (`S$0.00`), collapsing values that
/// round to zero so tiny negatives never render as `S$-0.00`.
pub fn money(amount: Decimal) -> String {
    let rounded = amount.round_dp(2);
    let value = if rounded.is_zero() {
        Decimal::ZERO
    } else {
        rounded
    };
    format!("S${value:.2}")
}

/// Bold text style.
fn bold() -> Style {
    Style::new().bold()
}

/// Build the Inter font family from the embedded regular and bold faces.
fn load_fonts() -> Result<FontFamily<FontData>, PdfError> {
    let load = |bytes: &[u8]| -> Result<FontData, PdfError> {
        FontData::new(bytes.to_vec(), None).map_err(|e| PdfError::Font(e.to_string()))
    };
    Ok(FontFamily {
        regular: load(REGULAR_FONT)?,
        bold: load(BOLD_FONT)?,
        // No italic faces embedded; reuse regular/bold so italics still render.
        italic: load(REGULAR_FONT)?,
        bold_italic: load(BOLD_FONT)?,
    })
}

/// Create a titled document with fonts, base font size, and page margins set.
pub fn new_document(title: &str) -> Result<Document, PdfError> {
    let mut doc = Document::new(load_fonts()?);
    doc.set_title(title);
    doc.set_font_size(10);
    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(18);
    doc.set_page_decorator(decorator);
    Ok(doc)
}

/// Push a document heading.
pub fn heading(doc: &mut Document, text: &str) {
    doc.push(elements::Paragraph::new(text).styled(Style::new().bold().with_font_size(18)));
    doc.push(elements::Break::new(0.5));
}

/// Push a section subheading.
pub fn subheading(doc: &mut Document, text: &str) {
    doc.push(elements::Paragraph::new(text).styled(Style::new().bold().with_font_size(12)));
    doc.push(elements::Break::new(0.5));
}

/// Push a single blank line.
pub fn spacer(doc: &mut Document) {
    doc.push(elements::Break::new(1));
}

/// The standard gap between a table and the total line(s) directly beneath it.
pub fn table_total_gap(doc: &mut Document) {
    doc.push(elements::Break::new(0.5));
}

/// A "label: value" line with the label in bold.
pub fn meta_line(label: &str, value: &str) -> elements::Paragraph {
    let mut p = elements::Paragraph::default();
    p.push_styled(format!("{label}: "), bold());
    p.push(value);
    p
}

/// A plain, unlabelled text line.
pub fn text_line(text: &str) -> elements::Paragraph {
    elements::Paragraph::new(text)
}

/// A right-aligned "label: value" summary line, optionally emphasised.
pub fn total_line(doc: &mut Document, label: &str, value: &str, strong: bool) {
    let style = if strong { bold() } else { Style::new() };
    let mut p = elements::Paragraph::default();
    p.push_styled(format!("{label}: "), style);
    p.push_styled(value.to_string(), style);
    p.set_alignment(Alignment::Right);
    doc.push(p);
}

/// Push a bordered table: first column left, the rest right-aligned.
/// `weights` are *relative* column widths (e.g. `[6, 3]` = 2:1).
pub fn push_table(doc: &mut Document, weights: &[usize], header: &[&str], rows: Vec<Vec<String>>) {
    push_table_aligned(doc, weights, header, rows, 1);
}

/// Push a bordered table, right-aligning columns from index `right_from`
/// onwards; earlier columns stay left-aligned. `weights` are relative widths.
pub fn push_table_aligned(
    doc: &mut Document,
    weights: &[usize],
    header: &[&str],
    rows: Vec<Vec<String>>,
    right_from: usize,
) {
    let mut table = elements::TableLayout::new(weights.to_vec());
    table.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));

    push_cells(
        &mut table,
        header.iter().map(|s| s.to_string()),
        true,
        right_from,
    );
    for row in rows {
        push_cells(&mut table, row.into_iter(), false, right_from);
    }
    doc.push(table);
}

/// Push one table row (header cells in bold), right-aligning cells from index
/// `right_from` onwards.
fn push_cells(
    table: &mut elements::TableLayout,
    cells: impl Iterator<Item = String>,
    header: bool,
    right_from: usize,
) {
    let mut row = table.row();
    for (i, text) in cells.enumerate() {
        let mut para = elements::Paragraph::new(text);
        if i >= right_from {
            para.set_alignment(Alignment::Right);
        }
        let style = if header { bold() } else { Style::new() };
        // Pad each cell generously so rows aren't cramped against the borders.
        row.push_element(para.styled(style).padded(Margins::trbl(2, 3, 2, 3)));
    }
    row.push().expect("row cell count matches column count");
}

/// Render the finished document into PDF bytes.
pub fn render(doc: Document) -> Result<Vec<u8>, PdfError> {
    let mut buf = Vec::new();
    doc.render(&mut buf)
        .map_err(|e| PdfError::Render(e.to_string()))?;
    Ok(buf)
}
