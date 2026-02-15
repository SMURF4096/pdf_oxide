//! Comprehensive PDF corpus verification tool with timing and password support.
//!
//! Usage: verify_corpus [OPTIONS] <directory>...
//!
//! Options:
//!   --timeout <secs>    Per-PDF timeout in seconds (default: 180)
//!   --csv <path>        Write detailed CSV report
//!   --passwords <path>  Load password list from file (one per line)
//!   --slow <ms>         Threshold for "slow" classification (default: 5000)
//!   --jobs <n>          Parallel jobs (default: 1, sequential for stability)

use pdf_oxide::document::PdfDocument;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Known passwords commonly used in PDF test suites (pdf.js, veraPDF, SafeDocs)
const KNOWN_PASSWORDS: &[&str] = &[
    "",             // empty password (most common for owner-only encryption)
    "owner",        // pdf.js common
    "user",         // pdf.js common
    "asdfasdf",     // pdf.js common
    "password",     // generic
    "test",         // generic
    "123456",       // generic
    "ownerpass",    // pdf.js
    "userpass",     // pdf.js
    "Password",     // capitalized
];

#[derive(Debug, Clone)]
struct PdfResult {
    path: String,
    filename: String,
    corpus: String,
    status: Status,
    error_msg: String,
    open_ms: f64,
    page_count: usize,
    extract_ms: f64,
    total_ms: f64,
    file_size_bytes: u64,
    pages_per_sec: f64,
    mb_per_sec: f64,
    password_used: String,
}

#[derive(Debug, Clone, PartialEq)]
enum Status {
    Pass,
    Fail,
    Panic,
    Timeout,
    PasswordProtected,
    Slow,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Pass => write!(f, "PASS"),
            Status::Fail => write!(f, "FAIL"),
            Status::Panic => write!(f, "PANIC"),
            Status::Timeout => write!(f, "TIMEOUT"),
            Status::PasswordProtected => write!(f, "PASSWORD"),
            Status::Slow => write!(f, "SLOW"),
        }
    }
}

fn detect_corpus(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    if path_str.contains("veraPDF") {
        "veraPDF".to_string()
    } else if path_str.contains("pdfium") {
        "pdfium".to_string()
    } else if path_str.contains("pdfs_pdfjs") {
        "pdfjs".to_string()
    } else if path_str.contains("pdfs_safedocs") {
        "safedocs".to_string()
    } else if path_str.contains("pdfs_issue_regression") {
        "issue_regression".to_string()
    } else if path_str.contains("fixtures_regression") {
        "fixtures_regression".to_string()
    } else if path_str.contains("fixtures_policy") {
        "fixtures_policy".to_string()
    } else if path_str.contains("pdf_large") {
        "pdf_large".to_string()
    } else if path_str.contains("pdf_oxide_tests") {
        "pdf_oxide_tests".to_string()
    } else {
        "unknown".to_string()
    }
}

fn try_open_with_passwords(
    path: &Path,
    passwords: &[String],
) -> (
    Result<PdfDocument, String>,
    String,
    Duration,
) {
    let start = Instant::now();

    // First try opening normally (empty password auto-attempted by library)
    match PdfDocument::open(path) {
        Ok(mut doc) => {
            // Check if we can actually access pages (encryption might block this)
            match doc.page_count() {
                Ok(_) => return (Ok(doc), String::new(), start.elapsed()),
                Err(e) => {
                    let err_str = e.to_string();
                    // If it's a password error, try passwords
                    if err_str.contains("password")
                        || err_str.contains("encrypt")
                        || err_str.contains("Incorrect password")
                        || err_str.contains("Authentication")
                    {
                        // Try each password
                        for pw in passwords {
                            if let Ok(mut doc2) = PdfDocument::open(path) {
                                if let Ok(true) = doc2.authenticate(pw.as_bytes()) {
                                    if doc2.page_count().is_ok() {
                                        return (
                                            Ok(doc2),
                                            pw.clone(),
                                            start.elapsed(),
                                        );
                                    }
                                }
                            }
                        }
                        return (Err(format!("Password protected: {}", err_str)), String::new(), start.elapsed());
                    }
                    return (Err(err_str), String::new(), start.elapsed());
                }
            }
        }
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("password")
                || err_str.contains("encrypt")
                || err_str.contains("Incorrect password")
            {
                // Try passwords on open failure
                for pw in passwords {
                    if let Ok(mut doc2) = PdfDocument::open(path) {
                        if let Ok(true) = doc2.authenticate(pw.as_bytes()) {
                            if doc2.page_count().is_ok() {
                                return (Ok(doc2), pw.clone(), start.elapsed());
                            }
                        }
                    }
                }
                return (Err(format!("Password protected: {}", err_str)), String::new(), start.elapsed());
            }
            return (Err(err_str), String::new(), start.elapsed());
        }
    }
}

fn process_pdf(path: &Path, passwords: &[String], slow_threshold_ms: u64) -> PdfResult {
    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let corpus = detect_corpus(path);
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let total_start = Instant::now();

    let (doc_result, password_used, open_duration) = try_open_with_passwords(path, passwords);
    let open_ms = open_duration.as_secs_f64() * 1000.0;

    let mut doc = match doc_result {
        Ok(d) => d,
        Err(e) => {
            let total_ms = total_start.elapsed().as_secs_f64() * 1000.0;
            let status = if e.contains("Password protected") {
                Status::PasswordProtected
            } else {
                Status::Fail
            };
            return PdfResult {
                path: path.to_string_lossy().to_string(),
                filename,
                corpus,
                status,
                error_msg: e,
                open_ms,
                page_count: 0,
                extract_ms: 0.0,
                total_ms,
                file_size_bytes: file_size,
                pages_per_sec: 0.0,
                mb_per_sec: 0.0,
                password_used,
            };
        }
    };

    // Get page count
    let page_count = match doc.page_count() {
        Ok(c) => c,
        Err(e) => {
            let total_ms = total_start.elapsed().as_secs_f64() * 1000.0;
            return PdfResult {
                path: path.to_string_lossy().to_string(),
                filename,
                corpus,
                status: Status::Fail,
                error_msg: format!("page_count failed: {}", e),
                open_ms,
                page_count: 0,
                extract_ms: 0.0,
                total_ms,
                file_size_bytes: file_size,
                pages_per_sec: 0.0,
                mb_per_sec: 0.0,
                password_used,
            };
        }
    };

    // Extract text from all pages
    let extract_start = Instant::now();
    let mut error_msg = String::new();
    let mut any_error = false;

    for page_idx in 0..page_count {
        match doc.extract_text(page_idx) {
            Ok(_) => {}
            Err(e) => {
                if error_msg.is_empty() {
                    error_msg = format!("page {}: {}", page_idx, e);
                }
                any_error = true;
            }
        }
    }

    let extract_ms = extract_start.elapsed().as_secs_f64() * 1000.0;
    let total_ms = total_start.elapsed().as_secs_f64() * 1000.0;

    let pages_per_sec = if total_ms > 0.0 {
        (page_count as f64) / (total_ms / 1000.0)
    } else {
        0.0
    };
    let mb_per_sec = if total_ms > 0.0 {
        (file_size as f64 / 1_048_576.0) / (total_ms / 1000.0)
    } else {
        0.0
    };

    let status = if any_error {
        Status::Fail
    } else if total_ms > slow_threshold_ms as f64 {
        Status::Slow
    } else {
        Status::Pass
    };

    PdfResult {
        path: path.to_string_lossy().to_string(),
        filename,
        corpus,
        status,
        error_msg,
        open_ms,
        page_count,
        extract_ms,
        total_ms,
        file_size_bytes: file_size,
        pages_per_sec,
        mb_per_sec,
        password_used,
    }
}

fn find_pdfs(dir: &Path) -> Vec<PathBuf> {
    let mut pdfs = Vec::new();
    fn walk(dir: &Path, pdfs: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, pdfs);
                } else if let Some(ext) = path.extension() {
                    if ext.eq_ignore_ascii_case("pdf") {
                        pdfs.push(path);
                    }
                }
            }
        }
    }
    walk(dir, &mut pdfs);
    pdfs.sort();
    pdfs
}

fn print_histogram(label: &str, values: &[f64], unit: &str) {
    if values.is_empty() {
        return;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let len = sorted.len();
    let sum: f64 = sorted.iter().sum();
    let mean = sum / len as f64;
    let p50 = sorted[len / 2];
    let p90 = sorted[(len as f64 * 0.9) as usize];
    let p95 = sorted[(len as f64 * 0.95) as usize];
    let p99 = sorted[((len as f64 * 0.99) as usize).min(len - 1)];
    let max = sorted[len - 1];
    let min = sorted[0];

    println!("  {label}:");
    println!("    count={len}  min={min:.1}{unit}  mean={mean:.1}{unit}  p50={p50:.1}{unit}  p90={p90:.1}{unit}  p95={p95:.1}{unit}  p99={p99:.1}{unit}  max={max:.1}{unit}");

    // Bucket histogram
    let buckets: &[(f64, &str)] = &[
        (10.0, "<10ms"),
        (50.0, "10-50ms"),
        (100.0, "50-100ms"),
        (500.0, "100-500ms"),
        (1000.0, "0.5-1s"),
        (5000.0, "1-5s"),
        (10000.0, "5-10s"),
        (30000.0, "10-30s"),
        (60000.0, "30-60s"),
        (f64::MAX, ">60s"),
    ];
    let mut prev = 0.0;
    for &(upper, label) in buckets {
        let count = sorted.iter().filter(|&&v| v >= prev && v < upper).count();
        if count > 0 {
            let bar_len = (count as f64 / len as f64 * 40.0) as usize;
            let bar: String = "#".repeat(bar_len.max(1));
            println!("    {label:>10}: {count:>5} ({:>5.1}%) {bar}", count as f64 / len as f64 * 100.0);
        }
        prev = upper;
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Parse arguments
    let mut directories: Vec<String> = Vec::new();
    let mut timeout_secs: u64 = 180;
    let mut csv_path: Option<String> = None;
    let mut password_file: Option<String> = None;
    let mut slow_threshold_ms: u64 = 5000;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--timeout" => {
                i += 1;
                timeout_secs = args[i].parse().expect("Invalid timeout");
            }
            "--csv" => {
                i += 1;
                csv_path = Some(args[i].clone());
            }
            "--passwords" => {
                i += 1;
                password_file = Some(args[i].clone());
            }
            "--slow" => {
                i += 1;
                slow_threshold_ms = args[i].parse().expect("Invalid slow threshold");
            }
            _ => {
                directories.push(args[i].clone());
            }
        }
        i += 1;
    }

    if directories.is_empty() {
        eprintln!("Usage: verify_corpus [OPTIONS] <directory>...");
        eprintln!("  --timeout <secs>    Per-PDF timeout (default: 180)");
        eprintln!("  --csv <path>        Write CSV report");
        eprintln!("  --passwords <path>  Password file (one per line)");
        eprintln!("  --slow <ms>         Slow threshold (default: 5000)");
        std::process::exit(1);
    }

    // Build password list
    let mut passwords: Vec<String> = KNOWN_PASSWORDS.iter().map(|s| s.to_string()).collect();
    if let Some(pw_file) = &password_file {
        if let Ok(content) = std::fs::read_to_string(pw_file) {
            for line in content.lines() {
                let pw = line.trim().to_string();
                if !pw.is_empty() && !passwords.contains(&pw) {
                    passwords.push(pw);
                }
            }
        }
    }

    // Collect all PDFs
    let mut all_pdfs: Vec<PathBuf> = Vec::new();
    for dir in &directories {
        let dir_path = Path::new(dir);
        if !dir_path.exists() {
            eprintln!("WARNING: Directory not found: {}", dir);
            continue;
        }
        let pdfs = find_pdfs(dir_path);
        eprintln!("  Found {} PDFs in {}", pdfs.len(), dir);
        all_pdfs.extend(pdfs);
    }

    let total = all_pdfs.len();
    eprintln!("Total: {} PDFs to verify (timeout={}s, slow={}ms)", total, timeout_secs, slow_threshold_ms);
    eprintln!();

    // Process each PDF
    let mut results: Vec<PdfResult> = Vec::with_capacity(total);
    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut password_blocked = 0usize;
    let mut slow = 0usize;
    let mut timeout = 0usize;
    let mut panic_count = 0usize;

    let global_start = Instant::now();

    for (idx, pdf_path) in all_pdfs.iter().enumerate() {
        let timeout_dur = Duration::from_secs(timeout_secs);

        // Use a separate thread + channel for real timeout enforcement
        let pdf_path_clone = pdf_path.clone();
        let passwords_clone = passwords.clone();
        let slow_ms = slow_threshold_ms;

        let (tx, rx) = std::sync::mpsc::channel();
        let _handle = std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                process_pdf(&pdf_path_clone, &passwords_clone, slow_ms)
            }));
            let _ = tx.send(result);
        });

        let result = match rx.recv_timeout(timeout_dur) {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };
                PdfResult {
                    path: pdf_path.to_string_lossy().to_string(),
                    filename: pdf_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                    corpus: detect_corpus(pdf_path),
                    status: Status::Panic,
                    error_msg: msg,
                    open_ms: 0.0,
                    page_count: 0,
                    extract_ms: 0.0,
                    total_ms: 0.0,
                    file_size_bytes: std::fs::metadata(pdf_path).map(|m| m.len()).unwrap_or(0),
                    pages_per_sec: 0.0,
                    mb_per_sec: 0.0,
                    password_used: String::new(),
                }
            }
            Err(_) => {
                // Timeout — thread is still running but we move on
                PdfResult {
                    path: pdf_path.to_string_lossy().to_string(),
                    filename: pdf_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                    corpus: detect_corpus(pdf_path),
                    status: Status::Timeout,
                    error_msg: format!("Exceeded {}s timeout", timeout_secs),
                    open_ms: 0.0,
                    page_count: 0,
                    extract_ms: 0.0,
                    total_ms: timeout_secs as f64 * 1000.0,
                    file_size_bytes: std::fs::metadata(pdf_path).map(|m| m.len()).unwrap_or(0),
                    pages_per_sec: 0.0,
                    mb_per_sec: 0.0,
                    password_used: String::new(),
                }
            }
        };

        match result.status {
            Status::Pass => pass += 1,
            Status::Fail => fail += 1,
            Status::PasswordProtected => password_blocked += 1,
            Status::Slow => slow += 1,
            Status::Timeout => timeout += 1,
            Status::Panic => panic_count += 1,
        }

        // Print non-pass results immediately
        match result.status {
            Status::Pass | Status::Slow => {}
            _ => {
                let short_err = if result.error_msg.len() > 80 {
                    format!("{}...", &result.error_msg[..80])
                } else {
                    result.error_msg.clone()
                };
                eprintln!(
                    "  {:>7} {:>8.0}ms  {} — {}",
                    result.status, result.total_ms, result.filename, short_err
                );
            }
        }

        // Print slow results
        if result.status == Status::Slow {
            eprintln!(
                "  {:>7} {:>8.0}ms  {} ({} pages, {:.1} MB)",
                result.status,
                result.total_ms,
                result.filename,
                result.page_count,
                result.file_size_bytes as f64 / 1_048_576.0
            );
        }

        results.push(result);

        // Progress every 200 files
        if (idx + 1) % 200 == 0 || idx + 1 == total {
            let elapsed = global_start.elapsed().as_secs();
            let rate = if elapsed > 0 { (idx + 1) as f64 / elapsed as f64 } else { 0.0 };
            eprintln!(
                "  [{}/{}] pass={} fail={} pw={} slow={} timeout={} panic={}  ({:.1} PDFs/s)",
                idx + 1, total, pass, fail, password_blocked, slow, timeout, panic_count, rate
            );
        }
    }

    let global_elapsed = global_start.elapsed();

    // ═══════════════════════════════════════════════════════════════
    // Summary
    // ═══════════════════════════════════════════════════════════════
    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("  VERIFICATION RESULTS  ({:.1}s elapsed)", global_elapsed.as_secs_f64());
    println!("═══════════════════════════════════════════════════════════════");
    println!("  Total:           {}", total);
    println!("  Pass:            {} ({:.1}%)", pass, pass as f64 / total as f64 * 100.0);
    println!("  Slow (>{}ms):    {} ({:.1}%)", slow_threshold_ms, slow, slow as f64 / total as f64 * 100.0);
    println!("  Fail:            {} ({:.1}%)", fail, fail as f64 / total as f64 * 100.0);
    println!("  Password:        {} ({:.1}%)", password_blocked, password_blocked as f64 / total as f64 * 100.0);
    println!("  Timeout (>{}s):  {} ({:.1}%)", timeout_secs, timeout, timeout as f64 / total as f64 * 100.0);
    println!("  Panic:           {}", panic_count);
    println!("═══════════════════════════════════════════════════════════════");
    let success_rate = (pass + slow) as f64 / total as f64 * 100.0;
    println!("  Success rate:    {:.1}% (pass + slow)", success_rate);
    println!();

    // Per-corpus breakdown
    println!("Per-corpus breakdown:");
    let mut corpus_stats: HashMap<String, (usize, usize, usize, usize, usize, usize)> = HashMap::new();
    for r in &results {
        let entry = corpus_stats.entry(r.corpus.clone()).or_insert((0, 0, 0, 0, 0, 0));
        entry.0 += 1; // total
        match r.status {
            Status::Pass => entry.1 += 1,
            Status::Fail => entry.2 += 1,
            Status::PasswordProtected => entry.3 += 1,
            Status::Slow => entry.4 += 1,
            Status::Timeout => entry.5 += 1,
            Status::Panic => entry.5 += 1,
        }
    }
    let mut corpus_names: Vec<_> = corpus_stats.keys().cloned().collect();
    corpus_names.sort();
    println!("  {:>20}  {:>6}  {:>6}  {:>6}  {:>4}  {:>4}  {:>6}", "Corpus", "Total", "Pass", "Fail", "PW", "Slow", "T/O+P");
    for name in &corpus_names {
        let (t, p, f, pw, s, tp) = corpus_stats[name];
        let rate = if t > 0 { (p + s) as f64 / t as f64 * 100.0 } else { 0.0 };
        println!("  {:>20}  {:>6}  {:>6}  {:>6}  {:>4}  {:>4}  {:>5}  ({:.1}%)", name, t, p, f, pw, s, tp, rate);
    }
    println!();

    // Timing distribution (pass + slow only)
    let pass_times: Vec<f64> = results
        .iter()
        .filter(|r| r.status == Status::Pass || r.status == Status::Slow)
        .map(|r| r.total_ms)
        .collect();
    let open_times: Vec<f64> = results
        .iter()
        .filter(|r| r.status == Status::Pass || r.status == Status::Slow)
        .map(|r| r.open_ms)
        .collect();
    let extract_times: Vec<f64> = results
        .iter()
        .filter(|r| (r.status == Status::Pass || r.status == Status::Slow) && r.page_count > 0)
        .map(|r| r.extract_ms)
        .collect();

    println!("Timing distribution (successful PDFs):");
    print_histogram("Total time", &pass_times, "ms");
    println!();
    print_histogram("Open time", &open_times, "ms");
    println!();
    print_histogram("Extract time", &extract_times, "ms");
    println!();

    // Top 20 slowest
    let mut by_time: Vec<&PdfResult> = results
        .iter()
        .filter(|r| r.status == Status::Pass || r.status == Status::Slow)
        .collect();
    by_time.sort_by(|a, b| b.total_ms.partial_cmp(&a.total_ms).unwrap());
    println!("Top 20 slowest PDFs:");
    println!("  {:>10}  {:>6}  {:>8}  {:>8}  {}", "total_ms", "pages", "MB", "pg/s", "filename");
    for r in by_time.iter().take(20) {
        println!(
            "  {:>10.0}  {:>6}  {:>8.1}  {:>8.1}  {}",
            r.total_ms,
            r.page_count,
            r.file_size_bytes as f64 / 1_048_576.0,
            r.pages_per_sec,
            r.filename
        );
    }
    println!();

    // List all failures
    let failures: Vec<&PdfResult> = results.iter().filter(|r| r.status == Status::Fail).collect();
    if !failures.is_empty() {
        println!("All failures ({}):", failures.len());
        // Group by error type
        let mut error_groups: HashMap<String, Vec<&PdfResult>> = HashMap::new();
        for r in &failures {
            // Extract error category (first line or first 60 chars)
            let category = r.error_msg.split('\n').next().unwrap_or(&r.error_msg);
            let category = if category.len() > 80 {
                &category[..80]
            } else {
                category
            };
            error_groups
                .entry(category.to_string())
                .or_default()
                .push(r);
        }
        let mut groups: Vec<_> = error_groups.into_iter().collect();
        groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        for (error, files) in &groups {
            println!("  [{} files] {}", files.len(), error);
            for r in files.iter().take(5) {
                println!("    - {} ({})", r.filename, r.corpus);
            }
            if files.len() > 5 {
                println!("    ... and {} more", files.len() - 5);
            }
        }
        println!();
    }

    // List panics
    let panics: Vec<&PdfResult> = results.iter().filter(|r| r.status == Status::Panic).collect();
    if !panics.is_empty() {
        println!("PANICS ({}) — THESE ARE BUGS:", panics.len());
        for r in &panics {
            println!("  - {} — {}", r.filename, r.error_msg);
        }
        println!();
    }

    // List password-protected
    let pw_protected: Vec<&PdfResult> = results
        .iter()
        .filter(|r| r.status == Status::PasswordProtected)
        .collect();
    if !pw_protected.is_empty() {
        println!("Password-protected ({}):", pw_protected.len());
        for r in &pw_protected {
            println!("  - {} ({})", r.filename, r.corpus);
        }
        println!();
    }

    // Write CSV
    if let Some(csv_file) = &csv_path {
        let mut f = std::fs::File::create(csv_file).expect("Cannot create CSV file");
        writeln!(
            f,
            "path,filename,corpus,status,error,open_ms,pages,extract_ms,total_ms,file_bytes,pages_per_sec,mb_per_sec,password"
        )
        .unwrap();
        for r in &results {
            writeln!(
                f,
                "\"{}\",\"{}\",{},{},\"{}\",{:.1},{},{:.1},{:.1},{},{:.1},{:.2},\"{}\"",
                r.path.replace('"', "\"\""),
                r.filename.replace('"', "\"\""),
                r.corpus,
                r.status,
                r.error_msg.replace('"', "\"\"").replace('\n', " "),
                r.open_ms,
                r.page_count,
                r.extract_ms,
                r.total_ms,
                r.file_size_bytes,
                r.pages_per_sec,
                r.mb_per_sec,
                r.password_used,
            )
            .unwrap();
        }
        eprintln!("CSV report written to: {}", csv_file);
    }

    // Exit code
    if panic_count > 0 {
        std::process::exit(2);
    } else if timeout > 0 {
        std::process::exit(1);
    }
}
