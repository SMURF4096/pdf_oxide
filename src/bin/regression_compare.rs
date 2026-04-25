//! Regression comparison binary.
//! For each PDF: extracts all pages sequentially, outputs per-page stats.
//! Usage: regression_compare <pdf_path>
//! Output: JSON lines per page + summary line

use pdf_oxide::PdfDocument;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::env;
use std::time::Instant;

fn text_hash(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

fn json_str(s: &str) -> String { format!("\"{}\"", escape(s)) }
fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

fn main() {
    let path = match env::args().nth(1) {
        Some(p) => p,
        None => { eprintln!("usage: regression_compare <pdf>"); std::process::exit(1); }
    };

    let t0 = Instant::now();
    let doc = match PdfDocument::open(&path) {
        Ok(d) => d,
        Err(e) => {
            println!("{{\"type\":\"error\",\"path\":{},\"stage\":\"open\",\"msg\":{}}}",
                json_str(&path), json_str(&e.to_string()));
            return;
        }
    };

    let page_count = match doc.page_count() {
        Ok(n) => n,
        Err(e) => {
            println!("{{\"type\":\"error\",\"path\":{},\"stage\":\"page_count\",\"msg\":{}}}",
                json_str(&path), json_str(&e.to_string()));
            return;
        }
    };

    let mut total_nonws: i64 = 0;
    let mut errors = 0usize;

    for i in 0..page_count {
        let pt = Instant::now();
        match doc.extract_text(i) {
            Ok(text) => {
                let nonws = text.chars().filter(|c| !c.is_whitespace()).count() as i64;
                let hash = text_hash(&text);
                let ms = pt.elapsed().as_millis();
                total_nonws += nonws;
                println!("{{\"type\":\"page\",\"path\":{},\"page\":{},\"nonws\":{},\"hash\":{},\"ms\":{}}}",
                    json_str(&path), i, nonws, hash, ms);
            }
            Err(e) => {
                errors += 1;
                println!("{{\"type\":\"page_err\",\"path\":{},\"page\":{},\"msg\":{}}}",
                    json_str(&path), i, json_str(&e.to_string()));
            }
        }
    }

    let total_ms = t0.elapsed().as_millis();
    println!("{{\"type\":\"summary\",\"path\":{},\"pages\":{},\"total_nonws\":{},\"errors\":{},\"total_ms\":{}}}",
        json_str(&path), page_count, total_nonws, errors, total_ms);
}
