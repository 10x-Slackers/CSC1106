//! Document-agnostic building blocks shared by every generated PDF.

use std::fs;
use std::sync::LazyLock;

use genpdf::fonts::{FontData, FontFamily};
use genpdf::style::Style;
use genpdf::{Alignment, Document, Element, Margins, SimplePageDecorator, elements};
use rust_decimal::Decimal;

use super::PdfError;

/// Directory holding the bundled TTF fonts, relative to the working directory
/// (consistent with how Tera templates and static assets are loaded).
const FONT_DIR: &str = "assets/fonts";

/// Bundled font bytes, read from disk once for the process lifetime.
static FONT_BYTES: LazyLock<std::io::Result<Vec<u8>>> =
    LazyLock::new(|| fs::read(format!("{FONT_DIR}/Inter-VariableFont.ttf")));

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

/// Load the bundled Inter font family.
fn load_fonts() -> Result<FontFamily<FontData>, PdfError> {
    let bytes = FONT_BYTES
        .as_ref()
        .map_err(|e| PdfError::FontLoad(std::io::Error::new(e.kind(), e.to_string())))?;
    let make = || FontData::new(bytes.clone(), None).map_err(|e| PdfError::Font(e.to_string()));
    Ok(FontFamily {
        regular: make()?,
        bold: make()?,
        italic: make()?,
        bold_italic: make()?,
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
    doc.push(elements::Break::new(0.2));
}

/// Push a single blank line.
pub fn spacer(doc: &mut Document) {
    doc.push(elements::Break::new(1));
}

/// A "label: value" line with the label in bold.
pub fn meta_line(label: &str, value: &str) -> elements::Paragraph {
    let mut p = elements::Paragraph::default();
    p.push_styled(format!("{label}: "), bold());
    p.push(value);
    p
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

/// Push a bordered table. `weights` sets relative column widths; every column
/// after the first is right-aligned (labels left, figures right).
pub fn push_table(doc: &mut Document, weights: &[usize], header: &[&str], rows: Vec<Vec<String>>) {
    let mut table = elements::TableLayout::new(weights.to_vec());
    table.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));

    push_cells(&mut table, header.iter().map(|s| s.to_string()), true);
    for row in rows {
        push_cells(&mut table, row.into_iter(), false);
    }
    doc.push(table);
}

/// Push one table row, right-aligning every cell after the first.
fn push_cells(
    table: &mut elements::TableLayout,
    cells: impl Iterator<Item = String>,
    header: bool,
) {
    let mut row = table.row();
    for (i, text) in cells.enumerate() {
        let mut para = elements::Paragraph::new(text);
        if i > 0 {
            para.set_alignment(Alignment::Right);
        }
        let style = if header { bold() } else { Style::new() };
        // Pad each cell so text never touches the frame borders.
        row.push_element(para.styled(style).padded(Margins::trbl(1, 2, 1, 2)));
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
