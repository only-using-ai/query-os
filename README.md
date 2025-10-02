The Only Using AI team is completely committed to not writing a single line of code by hand. We are to experiment to see how far we can go ONLY using AI. Usage (if using Cursor):

```bash
@product-development-team.md Implement the following feature <define the feature>
```

# Q - Filesystem & Process SQL Query Tool

A command-line application written in Rust that allows you to query both filesystem contents and system processes using SQL-like syntax.

## Features

- **Triple Query Support**: Query filesystem (`FROM /path`), processes (`FROM ps`), and applications (`FROM applications`)
- **Recursive directory traversal**: Automatically explores subdirectories
- **SQL-like syntax**: Supports SELECT, FROM, WHERE, ORDER BY, and LIMIT clauses
- **Subquery Support**: IN, EXISTS, and scalar subqueries in WHERE clauses and SELECT statements
- **Filesystem Fields**: name, type, modified_date, permissions, size, path, extension
- **Process Fields**: pid, name, cpu_usage, memory_usage, status
- **Application Fields**: name, version, path, size, category
- **Flexible filtering**: WHERE clauses with comparison operators (=, !=, >, <, >=, <=), LIKE patterns, compound conditions (AND), and negation (NOT)
- **Sorting**: ORDER BY support for all fields
- **Result limiting**: LIMIT clause to restrict output
- **Table output**: Clean tabular display in the terminal
- **Performance timing**: Shows query execution time in green at the bottom

## Usage

```bash
cargo run -- --query "SELECT * FROM /path/to/directory"
```

The tool displays query execution time in green text at the bottom of the results.

## Syntax

```
SELECT [fields|*] FROM path [WHERE condition] [ORDER BY field] [LIMIT number]
```

### Fields

#### Filesystem Queries
- `name`: File/directory name
- `type`: "file" or "directory"
- `modified_date`: Last modification date (YYYY-MM-DD HH:MM:SS)
- `permissions`: Unix permissions in octal format (e.g., 100644)
- `size`: File size with units (B, KB, MB, GB, TB)
- `path`: Relative path from the query root
- `extension`: File extension (lowercase, NULL for directories/files without extensions)

#### Process Queries
- `pid`: Process ID (numeric)
- `name`: Process name/command
- `cpu_usage`: CPU usage percentage (e.g., "5.2%")
- `memory_usage`: Memory usage with units (B, KB, MB, GB, TB)
- `status`: Process status (running, sleeping, idle, zombie, stopped)

### Examples

#### Filesystem Queries
```bash
# List all files and directories in current directory
q --query "SELECT * FROM ."

# Show only files in /tmp
q --query "SELECT name, type, size FROM /tmp WHERE type = 'file'"

# Find large files (>1MB) and sort by size
q --query "SELECT name, size FROM /home/user/Documents WHERE size > '1 MB' ORDER BY size"

# List directories only, limit to 10 results
q --query "SELECT name FROM /var WHERE type = 'directory' LIMIT 10"

# Find Rust source files
q --query "SELECT name FROM . WHERE name LIKE '%.rs'"

# Find files in the src directory
q --query "SELECT name, path FROM . WHERE path LIKE 'src/%'"

# Find Rust files but exclude those in target directory
q --query "SELECT name, path FROM . WHERE name LIKE '%.rs' AND path NOT LIKE '%target/%'"

# Find files modified recently, sorted by date
q --query "SELECT name, modified_date FROM . ORDER BY modified_date DESC LIMIT 5"

# Find all Rust source files
q --query "SELECT name, extension FROM . WHERE extension = 'rs'"

# Group files by extension and sort by extension
q --query "SELECT name, extension FROM . ORDER BY extension"

# Find files with specific extensions using LIKE
q --query "SELECT name, extension FROM . WHERE extension LIKE 'j%'"

# Find files without extensions
q --query "SELECT name FROM . WHERE extension = 'NULL'"

# Find files larger than the average file size
q --query "SELECT name, size FROM . WHERE size > (SELECT AVG(size) FROM . WHERE type = 'file')"

# Find files that exist in both directories using EXISTS
q --query "SELECT name FROM /dir1 WHERE EXISTS (SELECT 1 FROM /dir2 WHERE name = (SELECT name FROM /dir1))"
```

#### Process Queries
```bash
# List all processes
q --query "SELECT * FROM ps"

# Show running processes only
q --query "SELECT pid, name, status FROM ps WHERE status = 'running' LIMIT 10"

# Find processes by name pattern
q --query "SELECT pid, name, cpu_usage FROM ps WHERE name LIKE '%chrome%'"

# Find high CPU usage processes
q --query "SELECT pid, name, cpu_usage FROM ps WHERE cpu_usage > '1.0' ORDER BY cpu_usage DESC"

# Find memory-intensive processes
q --query "SELECT pid, name, memory_usage FROM ps WHERE memory_usage > '100 MB' ORDER BY memory_usage DESC"

# Find processes using more memory than the average
q --query "SELECT pid, name, memory_usage FROM ps WHERE memory_usage > (SELECT AVG(memory_usage) FROM ps)"

# Show process count by status
q --query "SELECT status, COUNT(*) as count FROM ps GROUP BY status"
```

#### Application Queries
```bash
# List all installed applications
q --query "SELECT * FROM applications"

# Find applications by name
q --query "SELECT name, version FROM applications WHERE name LIKE '%Chrome%'"

# Find developer tools
q --query "SELECT name, category FROM applications WHERE category LIKE '%developer%'"

# Find applications by category
q --query "SELECT name FROM applications WHERE category = 'public.app-category.productivity' LIMIT 10"

# Find applications with version information
q --query "SELECT name, version FROM applications WHERE version IS NOT NULL ORDER BY version DESC"

# Get application sizes (note: sizes are approximate for performance)
q --query "SELECT name, size FROM applications WHERE size > '100 MB' ORDER BY size DESC"
```

### WHERE Conditions

- String comparisons: `name = 'Cargo.toml'`, `type != 'directory'`
- Size comparisons: `size > '100 KB'`, `size < '1 GB'`
- Date comparisons: `modified_date > '2024-01-01'`
- Pattern matching: `name LIKE '%.rs'`, `path LIKE 'src/%'`
  - `%` matches zero or more characters
  - `_` matches exactly one character
- Compound conditions: `condition1 AND condition2`
- Negation: `NOT condition`, `field NOT LIKE 'pattern'`

### Subqueries

FQ supports subqueries to enable more complex queries:

#### WHERE Clause Subqueries
- **IN subqueries**: `WHERE name IN (SELECT name FROM /tmp WHERE type = 'file')`
- **EXISTS subqueries**: `WHERE EXISTS (SELECT 1 FROM /tmp WHERE name = 'target.txt')`
- **Scalar subqueries**: `WHERE size > (SELECT AVG(size) FROM /tmp)`

#### SELECT Clause Subqueries
- **Scalar subqueries**: `SELECT name, (SELECT COUNT(*) FROM /tmp WHERE type = 'file') as file_count FROM /tmp`

### Size Units

Size values can use units: B, KB, MB, GB, TB
- `size > '500 KB'`
- `size < '2 GB'`

## Installation

### Quick Install (Recommended)
```bash
# Clone the repository
git clone <repository-url>
cd query-os

# Run the install script
./install.sh
```

This will build and install the `q` binary to your Cargo bin directory.

### Manual Installation
```bash
# Ensure you have Rust installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone <repository-url>
cd query-os
cargo build --release

# The binary will be in target/release/q
# Optionally install globally
cargo install --path .
```

### Release Build (with tests)
For development or full release process:
```bash
./release.sh
```

This runs tests, builds in release mode, and installs the binary.

## Permission Handling

If the application encounters permission errors when accessing directories or files, it treats them as if they don't exist (as requested). This ensures the query continues without interruption.

## Error Handling

The application provides clear error messages for:
- Invalid SQL syntax
- Non-existent paths
- Unsupported operations

## Testing

Run the test suite with:
```bash
cargo test
```

The test suite covers:
- SQL parsing
- File metadata extraction
- Size formatting
- Filtering and sorting logic
