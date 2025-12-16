use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::{DirEntry, WalkBuilder};

pub mod comments;

use crate::comments::strip_comments_for_ext;

/// Configuration passed from the CLI layer (main.rs) into the core logic.
#[derive(Debug)]
pub struct Config {
    pub exts: HashSet<String>,
    pub paths: Vec<PathBuf>,
    pub follow_symlinks: bool,
    pub no_gitignore: bool,
    pub json: bool,
    pub excludes: Vec<String>,
    pub max_bytes: Option<u64>,
    pub strip_comments: bool,
    pub end_marker: bool,
}

#[derive(serde::Serialize)]
struct FileEntry {
    path: String,
    file_name: String,
    content: String,
}

pub fn run_with_config(cfg: Config) -> Result<()> {
    let exclude_globset = build_exclude_globset(&cfg.excludes)?;

    let mut had_error = false;
    let mut first_file = true;

    if cfg.json {
        println!("[");
    }

    for raw_root in &cfg.paths {
        // Canonicalise roots so running from arbitrary working dirs is reliable.
        let canon_root = match raw_root.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Skipping root {:?}: {}", raw_root, e);
                had_error = true;
                continue;
            }
        };

        let mut builder = WalkBuilder::new(&canon_root);
        builder.follow_links(cfg.follow_symlinks);

        // Helps avoid edge cases where process CWD is invalid and global ignores need a base.
        builder.current_dir(canon_root.clone());

        if cfg.no_gitignore {
            builder
                .git_ignore(false)
                .git_exclude(false)
                .git_global(false)
                .ignore(false);
        } else {
            builder
                .git_ignore(true)
                .git_exclude(true)
                .git_global(true)
                .ignore(true)
                .require_git(false);
        }

        // Values moved into the 'static filter closure must be owned separately.
        let root_for_filter = canon_root.clone();
        let exclude_globset = exclude_globset.clone();

        builder.filter_entry(move |entry: &DirEntry| {
            // Always keep the root.
            if entry.depth() == 0 {
                return true;
            }

            // Always keep the root.
            if entry.depth() == 0 {
                return true;
            }

            // Apply user exclude globs, relative to the current root.
            if let Some(ref gs) = exclude_globset {
                let path = entry.path();
                let rel = path.strip_prefix(&root_for_filter).unwrap_or(path);
                let rel_norm = normalize_for_matching(rel);

                if gs.is_match(&rel_norm) {
                    return false;
                }

                // If this is a directory, also try a trailing slash to make patterns
                // like `tests/**` able to prune the whole subtree early.
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
                    && !rel_norm.ends_with('/')
                {
                    let rel_dir = format!("{rel_norm}/");
                    if gs.is_match(&rel_dir) {
                        return false;
                    }
                }
            }

            true
        });

        let walker = builder.build();

        for result in walker {
            let entry = match result {
                Ok(e) => e,
                Err(err) => {
                    eprintln!("Walk error: {err}");
                    had_error = true;
                    continue;
                }
            };

            if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                continue;
            }

            let path = entry.path();
            if !matches_ext(path, &cfg.exts) {
                continue;
            }

            let display_path = make_display_path(&canon_root, path);

            if let Some(limit) = cfg.max_bytes
                && let Ok(meta) = fs::metadata(path)
                && meta.len() > limit
            {
                eprintln!(
                    "Skipping {} (size {} bytes > max {} bytes)",
                    display_path,
                    meta.len(),
                    limit
                );
                continue;
            }

            if cfg.json {
                if !first_file {
                    println!(",");
                }
                if let Err(err) = print_file_json(path, &display_path, cfg.strip_comments) {
                    eprintln!("Error printing {}: {:#}", display_path, err);
                    had_error = true;
                }
                first_file = false;
            } else if let Err(err) =
                print_file(path, &display_path, cfg.end_marker, cfg.strip_comments)
            {
                eprintln!("Error printing {}: {:#}", display_path, err);
                had_error = true;
            }
        }
    }

    if cfg.json {
        println!("\n]");
    }

    if had_error {
        anyhow::bail!("One or more files could not be read. See stderr for details.");
    }

    Ok(())
}

/// Build a GlobSet from the user–provided `--exclude` patterns.
/// Returns `Ok(None)` if there are no patterns.
fn build_exclude_globset(patterns: &[String]) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }

    let mut builder = GlobSetBuilder::new();

    for pat in patterns {
        let pat = pat.trim();
        if pat.is_empty() {
            continue;
        }

        let glob =
            Glob::new(pat).with_context(|| format!("Invalid --exclude glob pattern: {pat}"))?;
        builder.add(glob);
    }

    let set = builder
        .build()
        .context("Failed to build exclude glob set")?;

    Ok(Some(set))
}

/// Case-insensitive extension match, using the provided extension set.
pub fn matches_ext(path: &Path, exts: &HashSet<String>) -> bool {
    match path.extension().and_then(|s| s.to_str()) {
        Some(ext) => exts.contains(&ext.to_ascii_lowercase()),
        None => false,
    }
}

/// Produce a display path relative to `root` (stable regardless of current working directory).
pub fn make_display_path(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);

    // If root is a file and path == root, rel is empty.
    if rel.as_os_str().is_empty() {
        return path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
    }

    normalize_for_matching(rel)
}

/// Print a single file with header (and optional end marker), optionally stripping comments.
pub fn print_file(
    path: &Path,
    display_path: &str,
    end_marker: bool,
    strip_comments: bool,
) -> Result<()> {
    let bytes = fs::read(path).with_context(|| format!("Failed to read {}", display_path))?;
    let contents_lossy = String::from_utf8_lossy(&bytes);
    let mut text = contents_lossy.into_owned();

    if strip_comments {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        text = strip_comments_for_ext(&text, ext);
    }

    println!("========== FILE: {} ==========", display_path);
    print!("{text}");

    // Ensure there is a trailing newline before the separator between files.
    if !text.ends_with('\n') {
        println!();
    }

    if end_marker {
        println!("========== END FILE: {} ==========\n", display_path);
    } else {
        println!();
    }

    Ok(())
}

fn print_file_json(path: &Path, display_path: &str, strip_comments: bool) -> Result<()> {
    let bytes = fs::read(path).with_context(|| format!("Failed to read {}", display_path))?;
    let contents_lossy = String::from_utf8_lossy(&bytes);
    let mut text = contents_lossy.into_owned();

    if strip_comments {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        text = strip_comments_for_ext(&text, ext);
    }

    let entry = FileEntry {
        path: display_path.to_string(),
        file_name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        content: text,
    };

    let json = serde_json::to_string(&entry)?;
    print!("{}", json);

    Ok(())
}

/// Convert paths to a stable, slash-separated form for matching/printing.
fn normalize_for_matching(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::Path;

    #[test]
    fn matches_ext_is_case_insensitive_and_requires_extension() {
        let mut exts = HashSet::new();
        exts.insert("py".to_string());

        assert!(matches_ext(Path::new("foo.PY"), &exts));
        assert!(matches_ext(Path::new("dir/bar.py"), &exts));
        assert!(!matches_ext(Path::new("README"), &exts));
        assert!(!matches_ext(Path::new("script.sh"), &exts));
    }
}
