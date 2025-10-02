use crate::models::{Condition, FileInfo, ProcessInfo};
use crate::processes::parse_memory;
use prettytable::{Cell, Row, Table};
use regex::Regex;

pub fn evaluate_conditions(file: &FileInfo, conditions: &[Condition]) -> bool {
    // All conditions must be true (AND logic)
    for condition in conditions {
        let result = evaluate_single_condition(file, condition);
        let final_result = if condition.negated { !result } else { result };

        if !final_result {
            return false;
        }
    }
    true
}

pub fn evaluate_single_condition(file: &FileInfo, condition: &Condition) -> bool {
    match condition.field.as_str() {
        "name" => {
            if condition.operator == "LIKE" {
                like_match(&file.name, &condition.value)
            } else {
                compare_strings(&file.name, &condition.operator, &condition.value)
            }
        }
        "type" => compare_strings(&file.file_type, &condition.operator, &condition.value),
        "permissions" => compare_strings(&file.permissions, &condition.operator, &condition.value),
        "path" => {
            if condition.operator == "LIKE" {
                like_match(&file.path, &condition.value)
            } else {
                compare_strings(&file.path, &condition.operator, &condition.value)
            }
        }
        "size" => {
            // For size comparison, extract numeric value
            if let Ok(file_size) = parse_size(&file.size) {
                if let Ok(compare_size) = parse_size(&condition.value) {
                    match condition.operator.as_str() {
                        "=" => file_size == compare_size,
                        "!=" => file_size != compare_size,
                        ">" => file_size > compare_size,
                        "<" => file_size < compare_size,
                        ">=" => file_size >= compare_size,
                        "<=" => file_size <= compare_size,
                        _ => false,
                    }
                } else {
                    false
                }
            } else {
                false
            }
        }
        "depth" => {
            // For depth comparison, compare numeric values directly
            if let Ok(compare_depth) = condition.value.parse::<usize>() {
                match condition.operator.as_str() {
                    "=" => file.depth == compare_depth,
                    "!=" => file.depth != compare_depth,
                    ">" => file.depth > compare_depth,
                    "<" => file.depth < compare_depth,
                    ">=" => file.depth >= compare_depth,
                    "<=" => file.depth <= compare_depth,
                    _ => false,
                }
            } else {
                false
            }
        }
        "extension" => {
            // Handle extension comparison, treating None as "NULL"
            let file_ext = file.extension.as_deref().unwrap_or("NULL");
            if condition.operator == "LIKE" {
                like_match(file_ext, &condition.value)
            } else {
                compare_strings(file_ext, &condition.operator, &condition.value)
            }
        }
        _ => false,
    }
}

pub fn parse_size(size_str: &str) -> Result<f64, String> {
    let re = Regex::new(r"([\d.]+)\s*(B|KB|MB|GB|TB)?").unwrap();
    if let Some(caps) = re.captures(size_str) {
        let num: f64 = caps[1]
            .parse()
            .map_err(|_| "Invalid number format".to_string())?;
        let unit = caps.get(2).map_or("B", |m| m.as_str());

        let multiplier = match unit {
            "B" => 1.0,
            "KB" => 1024.0,
            "MB" => 1024.0 * 1024.0,
            "GB" => 1024.0 * 1024.0 * 1024.0,
            "TB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
            _ => return Err(format!("Invalid size unit: {}", unit)),
        };

        Ok(num * multiplier)
    } else {
        Err("Invalid size format".to_string())
    }
}

pub fn like_match(text: &str, pattern: &str) -> bool {
    // Convert SQL LIKE pattern to regex
    // % matches zero or more characters
    // _ matches exactly one character

    let mut regex_pattern = String::new();
    let mut chars = pattern.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '%' => regex_pattern.push_str(".*"),
            '_' => regex_pattern.push('.'),
            _ => {
                // Escape regex special characters, but allow . and * that we add
                // Note: backslashes are NOT escaped since they're literal in SQL LIKE
                match ch {
                    '.' | '*' | '+' | '?' | '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' => {
                        regex_pattern.push('\\');
                        regex_pattern.push(ch);
                    }
                    _ => regex_pattern.push(ch),
                }
            }
        }
    }

    if let Ok(regex) = Regex::new(&format!("^{}$", regex_pattern)) {
        regex.is_match(text)
    } else {
        false
    }
}

pub fn compare_strings(left: &str, operator: &str, right: &str) -> bool {
    match operator {
        "=" => left == right,
        "!=" => left != right,
        ">" => left > right,
        "<" => left < right,
        ">=" => left >= right,
        "<=" => left <= right,
        _ => false,
    }
}

pub fn sort_results(results: &mut [FileInfo], order_by: &str, direction: &crate::models::SortDirection) -> Result<(), String> {
    let field = order_by.trim().to_lowercase();

    results.sort_by(|a, b| {
        let ordering = match field.as_str() {
            "name" => a.name.cmp(&b.name),
            "type" => a.file_type.cmp(&b.file_type),
            "modified_date" => a.modified_date.cmp(&b.modified_date),
            "permissions" => a.permissions.cmp(&b.permissions),
            "size" => {
                let a_size = parse_size(&a.size).unwrap_or(0.0);
                let b_size = parse_size(&b.size).unwrap_or(0.0);
                a_size
                    .partial_cmp(&b_size)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
            "path" => a.path.cmp(&b.path),
            "extension" => a.extension.cmp(&b.extension),
            _ => std::cmp::Ordering::Equal,
        };

        // Reverse ordering for descending sort
        match direction {
            crate::models::SortDirection::Descending => ordering.reverse(),
            crate::models::SortDirection::Ascending => ordering,
        }
    });

    Ok(())
}

pub fn sort_process_results(results: &mut [ProcessInfo], order_by: &str, direction: &crate::models::SortDirection) -> Result<(), String> {
    let field = order_by.trim().to_lowercase();

    results.sort_by(|a, b| {
        let ordering = match field.as_str() {
            "pid" => {
                let a_pid: u32 = a.pid.parse().unwrap_or(0);
                let b_pid: u32 = b.pid.parse().unwrap_or(0);
                a_pid.cmp(&b_pid)
            }
            "name" => a.name.cmp(&b.name),
            "cpu_usage" => {
                let a_cpu: f32 = a
                    .cpu_usage
                    .strip_suffix('%')
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0.0);
                let b_cpu: f32 = b
                    .cpu_usage
                    .strip_suffix('%')
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0.0);
                a_cpu
                    .partial_cmp(&b_cpu)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
            "memory_usage" => {
                let a_memory = parse_memory(&a.memory_usage).unwrap_or(0.0);
                let b_memory = parse_memory(&b.memory_usage).unwrap_or(0.0);
                a_memory
                    .partial_cmp(&b_memory)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
            "status" => a.status.cmp(&b.status),
            _ => std::cmp::Ordering::Equal,
        };

        // Reverse ordering for descending sort
        match direction {
            crate::models::SortDirection::Descending => ordering.reverse(),
            crate::models::SortDirection::Ascending => ordering,
        }
    });

    Ok(())
}

pub fn display_results(results: &[FileInfo], select_fields: &[String]) {
    let mut table = Table::new();

    // Check if this is web content (has web_content file_type)
    let is_web_content = results.iter().any(|f| f.file_type == "web_content");

    if is_web_content {
        // For web content, show selector as header and extracted content as rows
        let mut header_row = Row::empty();
        for field in select_fields {
            header_row.add_cell(Cell::new(field));
        }
        table.add_row(header_row);

        // Add data rows - each result is one extracted element
        for file in results {
            let mut row = Row::empty();
            // For web content, all columns show the same extracted content
            for _ in select_fields {
                row.add_cell(Cell::new(&file.path));
            }
            table.add_row(row);
        }
    } else {
        // Regular file results
        // Add header row
        let mut header_row = Row::empty();
        for field in select_fields {
            header_row.add_cell(Cell::new(field));
        }
        table.add_row(header_row);

        // Add data rows
        for file in results {
            let mut row = Row::empty();
            for field in select_fields {
                let value = match field.as_str() {
                    "name" => &file.name,
                    "type" => &file.file_type,
                    "modified_date" => &file.modified_date.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "permissions" => &file.permissions,
                    "size" => &file.size,
                    "path" => &file.path,
                    "depth" => &file.depth.to_string(),
                    "extension" => file.extension.as_deref().unwrap_or("NULL"),
                    _ => "",
                };
                row.add_cell(Cell::new(value));
            }
            table.add_row(row);
        }
    }

    table.printstd();
}

pub fn display_process_results(results: &[ProcessInfo], select_fields: &[String]) {
    let mut table = Table::new();

    // Add header row
    let mut header_row = Row::empty();
    for field in select_fields {
        header_row.add_cell(Cell::new(field));
    }
    table.add_row(header_row);

    // Add data rows
    for process in results {
        let mut row = Row::empty();
        for field in select_fields {
            let value = match field.as_str() {
                "pid" => &process.pid,
                "name" => &process.name,
                "cpu_usage" => &process.cpu_usage,
                "memory_usage" => &process.memory_usage,
                "status" => &process.status,
                _ => "",
            };
            row.add_cell(Cell::new(value));
        }
        table.add_row(row);
    }

    table.printstd();
}

pub fn display_network_results(results: &[crate::models::NetInfo], select_fields: &[String]) {
    let mut table = Table::new();

    // Add header row
    let mut header_row = Row::empty();
    for field in select_fields {
        header_row.add_cell(Cell::new(field));
    }
    table.add_row(header_row);

    // Add data rows
    for net_info in results {
        let mut row = Row::empty();
        for field in select_fields {
            let value = match field.as_str() {
                "name" => &net_info.name,
                "port" => &net_info.port,
                "pid" => &net_info.pid,
                _ => "",
            };
            // Display empty values (NULL) in gray
            if value.is_empty() {
                row.add_cell(Cell::new(&format!("\x1b[90mNULL\x1b[0m")));
            } else {
                row.add_cell(Cell::new(value));
            }
        }
        table.add_row(row);
    }

    table.printstd();
}

pub fn display_application_results(results: &[crate::models::ApplicationInfo], select_fields: &[String]) {
    let mut table = Table::new();

    // Add header row
    let mut header_row = Row::empty();
    for field in select_fields {
        header_row.add_cell(Cell::new(field));
    }
    table.add_row(header_row);

    // Add data rows
    for app in results {
        let mut row = Row::empty();
        for field in select_fields {
            let value = match field.as_str() {
                "name" => &app.name,
                "version" => app.version.as_deref().unwrap_or("NULL"),
                "path" => &app.path,
                "size" => app.size.as_deref().unwrap_or("NULL"),
                "category" => app.category.as_deref().unwrap_or("NULL"),
                _ => "",
            };
            // Display NULL values in gray
            if value == "NULL" || value.is_empty() {
                row.add_cell(Cell::new(&format!("\x1b[90mNULL\x1b[0m")));
            } else {
                row.add_cell(Cell::new(value));
            }
        }
        table.add_row(row);
    }

    table.printstd();
}

pub fn expand_path(path: &str) -> String {
    if path.starts_with('~') {
        if let Some(home_dir) = dirs::home_dir() {
            if path == "~" {
                return home_dir.to_string_lossy().to_string();
            } else if let Some(rest) = path.strip_prefix("~/") {
                return home_dir.join(rest).to_string_lossy().to_string();
            }
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Condition, FileInfo};
    use chrono::DateTime;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("512 B").unwrap(), 512.0);
        assert_eq!(parse_size("1.00 KB").unwrap(), 1024.0);
        assert_eq!(parse_size("1 MB").unwrap(), 1024.0 * 1024.0);
    }

    #[test]
    fn test_like_match() {
        // Test ending with pattern
        assert!(like_match("main.rs", "%.rs"));
        assert!(like_match("test.rs", "%.rs"));
        assert!(!like_match("main.txt", "%.rs"));

        // Test starting with pattern
        assert!(like_match("Cargo.toml", "Cargo%"));
        assert!(like_match("Cargo.lock", "Cargo%"));
        assert!(!like_match("main.rs", "Cargo%"));

        // Test containing pattern (the main bug we fixed)
        assert!(like_match("file.md", "%.md%"));
        assert!(like_match("readme.md", "%.md%"));
        assert!(like_match("test.md.txt", "%.md%"));
        assert!(like_match("markdown.md", "%.md%"));
        assert!(like_match("file.mdx", "%.md%")); // .mdx contains .md
        assert!(!like_match("file.txt", "%.md%"));
        assert!(!like_match("mdfile", "%.md%"));
        assert!(!like_match("file.mad", "%.md%")); // .mad does not contain .md

        // Test path patterns
        assert!(like_match("src/main.rs", "src/%"));
        assert!(like_match("src/test/main.rs", "src/%"));
        assert!(!like_match("main.rs", "src/%"));

        // Test exact match
        assert!(like_match("test", "test"));
        assert!(!like_match("testing", "test"));

        // Test single character wildcard
        assert!(like_match("test.txt", "test._xt"));
        assert!(like_match("test.txt", "test.t_t"));
        assert!(!like_match("test.txt", "test._x"));

        // Test complex patterns
        assert!(like_match("src/main.rs", "src/%main%"));
        assert!(like_match("target/debug/main", "target/%/main"));
        assert!(!like_match("src/test.rs", "src/%main%"));

        // Test patterns with regex special characters that should be escaped
        assert!(like_match("file[1].txt", "file[1]%"));
        assert!(like_match("file(1).txt", "file(1)%"));
        assert!(like_match("file+1.txt", "file+1%"));
        assert!(like_match("file^1.txt", "file^1%"));
        assert!(like_match("file$1.txt", "file$1%"));
        assert!(like_match("file?1.txt", "file?1%"));
        assert!(like_match("file*1.txt", "file*1%"));
        assert!(like_match("file.1.txt", "file.1%"));
        assert!(like_match("file|1.txt", "file|1%"));
        assert!(like_match("file\\1.txt", "file\\\\1%"));
    }

    #[test]
    fn test_evaluate_conditions() {
        let file = FileInfo {
            name: "main.rs".to_string(),
            file_type: "file".to_string(),
            modified_date: DateTime::from(std::time::SystemTime::UNIX_EPOCH),
            permissions: "644".to_string(),
            size: "1024 B".to_string(),
            path: "src/main.rs".to_string(),
            depth: 2,
            extension: Some("rs".to_string()),
        };

        let conditions = vec![
            Condition {
                field: "name".to_string(),
                operator: "LIKE".to_string(),
                value: "%.rs".to_string(),
                negated: false,
            },
            Condition {
                field: "path".to_string(),
                operator: "LIKE".to_string(),
                value: "%target/%".to_string(),
                negated: true,
            },
        ];

        assert!(evaluate_conditions(&file, &conditions));

        // Test with a file that should NOT match
        let bad_file = FileInfo {
            name: "main.rs".to_string(),
            file_type: "file".to_string(),
            modified_date: DateTime::from(std::time::SystemTime::UNIX_EPOCH),
            permissions: "644".to_string(),
            size: "1024 B".to_string(),
            path: "target/debug/main.rs".to_string(), // This should fail the NOT LIKE condition
            depth: 3,
            extension: Some("rs".to_string()),
        };

        assert!(!evaluate_conditions(&bad_file, &conditions));
    }

    #[test]
    fn test_extension_filtering() {
        // Test file with extension
        let rs_file = FileInfo {
            name: "main.rs".to_string(),
            file_type: "file".to_string(),
            modified_date: DateTime::from(std::time::SystemTime::UNIX_EPOCH),
            permissions: "644".to_string(),
            size: "1024 B".to_string(),
            path: "src/main.rs".to_string(),
            depth: 2,
            extension: Some("rs".to_string()),
        };

        // Test file without extension
        let no_ext_file = FileInfo {
            name: "README".to_string(),
            file_type: "file".to_string(),
            modified_date: DateTime::from(std::time::SystemTime::UNIX_EPOCH),
            permissions: "644".to_string(),
            size: "512 B".to_string(),
            path: "README".to_string(),
            depth: 1,
            extension: None,
        };

        // Test directory
        let dir = FileInfo {
            name: "src".to_string(),
            file_type: "directory".to_string(),
            modified_date: DateTime::from(std::time::SystemTime::UNIX_EPOCH),
            permissions: "755".to_string(),
            size: "0 B".to_string(),
            path: "src".to_string(),
            depth: 1,
            extension: None,
        };

        // Test filtering by extension
        let rs_condition = Condition {
            field: "extension".to_string(),
            operator: "=".to_string(),
            value: "rs".to_string(),
            negated: false,
        };

        assert!(evaluate_conditions(&rs_file, &[rs_condition.clone()]));
        assert!(!evaluate_conditions(&no_ext_file, &[rs_condition.clone()]));
        assert!(!evaluate_conditions(&dir, &[rs_condition.clone()]));

        // Test filtering by NULL extension
        let null_condition = Condition {
            field: "extension".to_string(),
            operator: "=".to_string(),
            value: "NULL".to_string(),
            negated: false,
        };

        assert!(!evaluate_conditions(&rs_file, &[null_condition.clone()]));
        assert!(evaluate_conditions(&no_ext_file, &[null_condition.clone()]));
        assert!(evaluate_conditions(&dir, &[null_condition.clone()]));

        // Test LIKE pattern matching for extensions
        let like_condition = Condition {
            field: "extension".to_string(),
            operator: "LIKE".to_string(),
            value: "r%".to_string(),
            negated: false,
        };

        assert!(evaluate_conditions(&rs_file, &[like_condition.clone()]));
        assert!(!evaluate_conditions(
            &no_ext_file,
            &[like_condition.clone()]
        ));
    }

    #[test]
    fn test_expand_path() {
        // Test regular path (should remain unchanged)
        assert_eq!(expand_path("/tmp/test"), "/tmp/test");
        assert_eq!(expand_path("relative/path"), "relative/path");

        // Test home directory expansion
        if let Some(home_dir) = dirs::home_dir() {
            let home_str = home_dir.to_string_lossy();
            assert_eq!(expand_path("~"), home_str);
            assert_eq!(
                expand_path("~/Documents"),
                format!("{}/Documents", home_str)
            );
            assert_eq!(
                expand_path("~/test/file"),
                format!("{}/test/file", home_str)
            );
        }
    }

    #[test]
    fn test_sort_results_descending() {
        let file1 = FileInfo {
            name: "a.txt".to_string(),
            file_type: "file".to_string(),
            modified_date: DateTime::from(std::time::SystemTime::UNIX_EPOCH),
            permissions: "644".to_string(),
            size: "100 B".to_string(),
            path: "a.txt".to_string(),
            depth: 1,
            extension: Some("txt".to_string()),
        };

        let file2 = FileInfo {
            name: "b.txt".to_string(),
            file_type: "file".to_string(),
            modified_date: DateTime::from(std::time::SystemTime::UNIX_EPOCH),
            permissions: "644".to_string(),
            size: "200 B".to_string(),
            path: "b.txt".to_string(),
            depth: 1,
            extension: Some("txt".to_string()),
        };

        let file3 = FileInfo {
            name: "c.txt".to_string(),
            file_type: "file".to_string(),
            modified_date: DateTime::from(std::time::SystemTime::UNIX_EPOCH),
            permissions: "644".to_string(),
            size: "50 B".to_string(),
            path: "c.txt".to_string(),
            depth: 1,
            extension: Some("txt".to_string()),
        };

        let mut results = vec![file1.clone(), file2.clone(), file3.clone()];

        // Test descending sort by name
        sort_results(&mut results, "name", &crate::models::SortDirection::Descending).unwrap();
        assert_eq!(results[0].name, "c.txt");
        assert_eq!(results[1].name, "b.txt");
        assert_eq!(results[2].name, "a.txt");

        // Test ascending sort by name
        sort_results(&mut results, "name", &crate::models::SortDirection::Ascending).unwrap();
        assert_eq!(results[0].name, "a.txt");
        assert_eq!(results[1].name, "b.txt");
        assert_eq!(results[2].name, "c.txt");
    }
}
