//! Headless export demo:
//!
//! ```sh
//! cargo run --example export_demo --no-default-features
//! ```

use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = root.join("assets/sample.md");
    let markdown = fs::read_to_string(&source)?;
    let title = mdlite::markdown::document_title(&markdown, Some(&source));

    let output_dir = root.join("output");
    fs::create_dir_all(&output_dir)?;

    let pdf_path = output_dir.join("sample.pdf");
    let docx_path = output_dir.join("sample.docx");

    mdlite::export::pdf::export_pdf(&markdown, &pdf_path, &title, source.parent())?;
    mdlite::export::docx::export_docx(&markdown, &docx_path, source.parent())?;

    println!("PDF : {}", pdf_path.display());
    println!("DOCX: {}", docx_path.display());
    Ok(())
}
