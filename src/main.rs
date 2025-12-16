use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{ArgAction, Parser};
use pc::{Config, run_with_config};

/// pc - print code.
///
/// Recursively print source files with file-path headers, ready to paste into
/// other tools (like ChatGPT). By default it:
///
///   - respects .gitignore / .ignore / git exclude files
///   - skips common junk directories (target, node_modules, venv, etc.)
///   - allows adding extra exclude globs
///   - can strip full-line comments and blank lines
#[derive(Parser, Debug)]
#[command(
    name = "pc",
    author,
    version,
    about = "Print code files with path separators, respecting .gitignore",
    long_about = r#"Recursively print source files with file-path headers, ready to
paste into other tools (like ChatGPT).

By default it:
  • respects .gitignore / .ignore / git exclude files
  • skips common junk directories (target, node_modules, venv, etc.)
  • allows adding extra exclude globs
  • can strip full-line comments and blank lines

Typical usage:
  pc -t py
  pc -t py,rs src tests
"#
)]
struct Args {
    /// File extensions / types to include (e.g. py, rs).
    ///
    /// Can be repeated or comma-separated:
    ///   pc -t py
    ///   pc -t py,rs
    ///   pc -t py -t rs
    #[arg(
        short = 't',
        long = "type",
        alias = "ext",
        value_name = "EXT",
        action = ArgAction::Append,
        value_delimiter = ',',
        required = true
    )]
    exts: Vec<String>,

    /// Paths to scan (files or directories). Defaults to current directory.
    ///
    /// You can pass multiple:
    ///   pc -t py src tests tools
    #[arg(value_name = "PATH", default_value = ".")]
    paths: Vec<PathBuf>,

    /// Follow symbolic links during traversal.
    #[arg(long = "follow-symlinks")]
    follow_symlinks: bool,

    /// Disable reading .gitignore / .ignore / git exclude files.
    ///
    /// By default, pc honours:
    ///   - .gitignore files in the tree
    ///   - .ignore files
    ///   - global Git exclude config
    #[arg(long = "no-gitignore")]
    no_gitignore: bool,

    /// Additional glob patterns to exclude (files or directories).
    ///
    /// Patterns are evaluated relative to each PATH root and use glob-style
    /// matching (via globset), e.g.:
    ///
    ///   pc -t py --exclude 'migrations/**'
    ///   pc -t py --exclude 'tests/**,*.gen.py'
    ///
    /// Multiple flags and comma-separated values are both allowed.
    #[arg(
        long = "exclude",
        short = 'E',
        value_name = "GLOB",
        action = ArgAction::Append,
        value_delimiter = ','
    )]
    excludes: Vec<String>,

    /// Maximum file size to print, in bytes (skip larger files).
    ///
    /// Useful when you want to avoid dumping big generated artifacts.
    #[arg(long = "max-bytes", value_name = "N")]
    max_bytes: Option<u64>,

    /// Strip full-line comments and blank lines when printing.
    ///
    /// For known extensions (py, sh, rs, c, cpp, js, ts, java, go, sql, etc.)
    /// this drops lines whose first non-whitespace chars are a comment marker.
    /// It deliberately does NOT try to strip inline/trailing comments or block
    /// comments to avoid breaking code.
    #[arg(long = "strip-comments")]
    strip_comments: bool,

    /// Output as a JSON array of objects { "path": "...", "content": "..." }.
    #[arg(long = "json")]
    json: bool,

    /// Print an explicit END marker after each file.
    ///
    /// This is handy if you want a clear end-of-file delimiter for tooling.
    #[arg(long = "end-marker")]
    end_marker: bool,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {:#}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    // Normalise extensions to lowercase, no leading dot.
    let mut ext_set = HashSet::new();
    for e in &args.exts {
        let norm = e.trim().trim_start_matches('.').to_ascii_lowercase();
        if !norm.is_empty() {
            ext_set.insert(norm);
        }
    }

    if ext_set.is_empty() {
        bail!("No valid extensions provided (after normalisation).");
    }

    let cfg = Config {
        exts: ext_set,
        paths: args.paths,
        follow_symlinks: args.follow_symlinks,

        no_gitignore: args.no_gitignore,
        json: args.json,
        excludes: args.excludes,
        max_bytes: args.max_bytes,
        strip_comments: args.strip_comments,
        end_marker: args.end_marker,
    };

    run_with_config(cfg)
}
