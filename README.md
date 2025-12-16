# pc - print code

`pc` (“print code”) is a small CLI utility that recursively prints source files - ideal for feeding code into LLMs or other tools that expect a single text or json blob.

It is ideal for:
- Feeding code into LLMs.
- Preparing or labeling code and text datasets.
- Creating single-file backups, snapshots or validation checksums.

It is designed to be:

- **Predictable**
  - Respects `.gitignore`, `.ignore`, and git global excludes by default
- **LLM-friendly**
  - Each file is wrapped in a clear header (and optional end marker)
  - Optionally strip full-line comments and blank lines
- **Convenient**
  - Filter by extension
  - Add custom exclude globs
  - Guard against large files with a size limit
  - **JSON output** for easy parsing and dataset creation

---

## Install

From a clone of this repository:

```bash
# Build release binary
cargo build --release

# Optionally install into ~/.cargo/bin
cargo install --path .
```

Ensure `~/.cargo/bin` is on your `PATH` if you want to run `pc` directly:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

---

## Basic usage

Synopsis:

```text
pc [OPTIONS] --type <EXT>... [PATH]...
```

- `--type` (`-t` / `--ext`) is **required**
- `PATH` defaults to `.` (current directory) if omitted
- You can pass multiple paths

Examples:

```bash
# From the project root: print all Rust files
pc -t rs .

# Print Python files under the current directory
pc -t py

# Print Python AND Rust files
pc -t py,rs
# or
pc -t py -t rs

# Restrict to specific roots
pc -t py src tests

```

Output format (for each matched file):

```text
========== FILE: src/main.rs ==========
fn main() {
    println!("hello");
}
```

---

## Respecting `.gitignore` and skip rules

By default, `pc`:

- Honours:
  - `.gitignore` files in the tree
  - `.ignore` files
  - git global excludes (`core.excludesFile`)

This applies whether or not the directory is an actual git repository.

You can disable these behaviours if needed:

```bash
# Ignore .gitignore / .ignore / git global excludes
pc -t rs --no-gitignore .
```

---

## Excluding additional paths

You can add your own exclude rules as glob patterns (via `globset`):

```bash
# Exclude tests and migrations
pc -t py --exclude 'tests/**,migrations/**' .

# Multiple flags also work
pc -t py --exclude 'tests/**' --exclude 'migrations/**' .
```

Notes:

- Patterns are matched **relative to each root PATH** you pass.
- Examples:
  - `tests/**` – matches anything under `tests/` at any depth
  - `migrations/**` – matches anything under `migrations/`
  - `*.gen.py` – matches any file ending in `.gen.py`

---

## Stripping comments and blank lines

To reduce noise before sending code to a model, you can strip full-line
comments and blank lines:

```bash
# Python: strip full-line comments and blanks
pc -t py --strip-comments .

# Rust and Python together with stripping
pc -t rs,py --strip-comments .
```

Behaviour:

- For **known languages**, it drops lines whose first non-whitespace characters are a comment leader:
  - `#` for `py`, `sh`, `bash`, `zsh`, `rb`, `yaml`, `yml`, `toml`
  - `//` for `rs`, `c`, `h`, `cpp`, `hpp`, `cc`, `js`, `ts`, `java`, `go`, `cs`, `swift`, `kt`
  - `--` for `sql`
- For **all files**, it drops completely blank lines.
- It **does not** attempt to remove:
  - inline/trailing comments (`code  # comment`, `code // comment`)
  - block comments (e.g. `/* ... */`, triple-quoted blocks, etc.)

The goal is conservative behaviour that does not risk breaking code.

---

## Limiting file size

To avoid accidentally dumping huge generated files:

```bash
# Skip files larger than 200 KB
pc -t py --max-bytes 200000 .
```

Files exceeding the limit are skipped with a message to `stderr`:

```text
Skipping src/big_generated.py (size 450123 bytes > max 200000 bytes)
```

---

## End-of-file markers

If you want explicit end markers, especially for tooling:


```bash
pc -t py --end-marker .
```

Then each file looks like:

```text
========== FILE: src/main.py ==========
print("hello")

========== END FILE: src/main.py ==========

```

---

## JSON Output

For programmatic usage or dataset creation, you can output a JSON array of file objects:

```bash
pc -t py --json .
```

Output:

```json
[{"path":"src/main.py","file_name":"main.py","content":"print(\"hello\")\n"}]
```

---

## Combined examples

Some practical combinations:

```bash
# 1. Dump all Python code in current project for LLM analysis
pc -t py --strip-comments --max-bytes 200000 > /tmp/project-py-for-llm.txt

# 2. Dump Rust + Python from two repos, excluding tests and large files
pc -t rs,py --exclude 'tests/**' --max-bytes 300000 /home/user/Projects/repo-a /home/user/Projects/repo-b > /tmp/multi-repo-snippet.txt

# 3. From anywhere: scan only src/ and tools/ in a repo
pc -t rs /home/user/Projects/my-repo/src /home/user/Projects/my-repo/tools
```
