use pdf_oxide::PdfDocument;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = &args[1];
    let page: usize = args[2].parse().unwrap();
    let doc = PdfDocument::open(path).unwrap();
    let text = doc.extract_text(page).unwrap_or_default();
    print!("{}", text);
}
