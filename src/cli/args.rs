use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "pdf-oxide",
    version,
    about = "Fast, local PDF processing",
    long_about = "pdf-oxide — the fastest PDF toolkit.\nRun with no arguments for interactive REPL mode."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Output file path (defaults to stdout for text outputs)
    #[arg(short, long, global = true)]
    pub output: Option<PathBuf>,

    /// Page range, e.g. "1-5", "1,3,7", "1-3,7,10-12"
    #[arg(short, long, global = true)]
    pub pages: Option<String>,

    /// Show verbose output with timing
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress all non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Output as JSON
    #[arg(short, long, global = true)]
    pub json: bool,

    /// Password for encrypted PDFs
    #[arg(long, global = true)]
    pub password: Option<String>,

    /// Skip the banner in REPL mode
    #[arg(long, global = true)]
    pub no_banner: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Extract plain text from a PDF
    Text {
        /// Input PDF file
        file: PathBuf,
    },

    /// Convert PDF to Markdown
    Markdown {
        /// Input PDF file
        file: PathBuf,
    },

    /// Convert PDF to HTML
    Html {
        /// Input PDF file
        file: PathBuf,
    },

    /// Show PDF metadata and page count
    Info {
        /// Input PDF file
        file: PathBuf,
    },

    /// Merge multiple PDFs into one
    Merge {
        /// Input PDF files (first file is the base)
        #[arg(required = true, num_args = 2..)]
        files: Vec<PathBuf>,
    },

    /// Split a PDF into individual pages
    Split {
        /// Input PDF file
        file: PathBuf,
    },

    /// Create a PDF from Markdown, HTML, or plain text
    Create {
        /// Input source file
        file: PathBuf,

        /// Input format
        #[arg(long, value_parser = ["markdown", "html", "text"])]
        from: String,
    },

    /// Compress and optimize a PDF
    Compress {
        /// Input PDF file
        file: PathBuf,
    },

    /// Encrypt a PDF with a password (placeholder — coming in v0.4.0)
    Encrypt {
        /// Input PDF file
        file: PathBuf,
    },

    /// Decrypt a password-protected PDF
    Decrypt {
        /// Input PDF file
        file: PathBuf,

        /// Password to decrypt
        #[arg(long)]
        password: String,
    },

    /// Search for text in a PDF
    Search {
        /// Input PDF file
        file: PathBuf,

        /// Search pattern (regex supported)
        pattern: String,

        /// Case-insensitive search
        #[arg(short, long)]
        ignore_case: bool,
    },

    /// Extract images from a PDF
    Images {
        /// Input PDF file
        file: PathBuf,
    },

    /// Rotate pages by 90, 180, or 270 degrees
    Rotate {
        /// Input PDF file
        file: PathBuf,

        /// Rotation angle in degrees (90, 180, 270, or -90)
        #[arg(long)]
        degrees: i32,
    },

    /// Remove specific pages from a PDF
    Delete {
        /// Input PDF file
        file: PathBuf,
    },

    /// Reorder pages in a PDF
    Reorder {
        /// Input PDF file
        file: PathBuf,

        /// New page order as comma-separated 1-indexed numbers (e.g. "3,1,2,5,4")
        #[arg(long)]
        order: String,
    },

    /// Read, edit, or strip PDF metadata
    Metadata {
        /// Input PDF file
        file: PathBuf,

        /// Set document title
        #[arg(long)]
        title: Option<String>,

        /// Set document author
        #[arg(long)]
        author: Option<String>,

        /// Set document subject
        #[arg(long)]
        subject: Option<String>,

        /// Set document keywords
        #[arg(long)]
        keywords: Option<String>,

        /// Strip all metadata fields
        #[arg(long)]
        strip: bool,
    },

    /// Add a text watermark to pages
    Watermark {
        /// Input PDF file
        file: PathBuf,

        /// Watermark text (presets: CONFIDENTIAL, DRAFT, SAMPLE, "DO NOT COPY")
        text: String,

        /// Opacity (0.0-1.0)
        #[arg(long, default_value = "0.3")]
        opacity: f32,

        /// Rotation angle in degrees
        #[arg(long, default_value = "45")]
        rotation: f32,

        /// Font size in points
        #[arg(long, default_value = "48")]
        font_size: f32,

        /// Text color as R,G,B (0.0-1.0 each, e.g. "0.8,0,0")
        #[arg(long)]
        color: Option<String>,
    },

    /// List document bookmarks/outline
    Bookmarks {
        /// Input PDF file
        file: PathBuf,
    },

    /// Flatten annotations and/or form fields
    Flatten {
        /// Input PDF file
        file: PathBuf,

        /// Flatten form fields
        #[arg(long)]
        forms: bool,

        /// Flatten annotations
        #[arg(long)]
        annotations: bool,
    },

    /// Crop page margins
    Crop {
        /// Input PDF file
        file: PathBuf,

        /// Margins as left,right,top,bottom in points (e.g. "50,50,50,50")
        #[arg(long)]
        margins: String,
    },

    /// List, fill, or export form fields
    Forms {
        /// Input PDF file
        file: PathBuf,

        /// Fill fields as key=value pairs (e.g. "name=John,age=30")
        #[arg(long)]
        fill: Option<String>,

        /// Export form data (fdf or xfdf)
        #[arg(long, value_parser = ["fdf", "xfdf"])]
        export: Option<String>,
    },
}
