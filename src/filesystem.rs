use crate::applications::execute_application_query;
use crate::models::{Condition, FileInfo, ProcessInfo, QueryResult, QueryType, SqlQuery};
use crate::network::execute_network_query;
use crate::parser::parse_compound_conditions;
use crate::processes::execute_process_query;
use crate::utils::{compare_strings, evaluate_single_condition, like_match, sort_results};
use crate::web::{execute_web_query, is_url};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Mutex;

pub fn execute_query(query: &SqlQuery) -> Result<QueryResult, String> {
    // Handle DELETE queries
    if query.query_type == QueryType::Delete {
        return execute_delete_query(query);
    }

    // Execute subqueries first and store their results
    let mut subquery_results = HashMap::new();
    let mut subquery_idx = 0;

    // Execute WHERE subqueries
    for subquery in &query.where_subqueries {
        let result = execute_query(&subquery.query)?;
        subquery_results.insert(format!("__SUBQUERY_{}__", subquery_idx), result.clone());
        subquery_results.insert(
            format!("__EXISTS_SUBQUERY_{}__", subquery_idx),
            result.clone(),
        );
        subquery_results.insert(format!("__SCALAR_SUBQUERY_{}__", subquery_idx), result);
        subquery_idx += 1;
    }

    // Execute SELECT subqueries
    for subquery in &query.select_subqueries {
        let result = execute_query(&subquery.query)?;
        subquery_results.insert(format!("__SELECT_SUBQUERY_{}__", subquery_idx), result);
        subquery_idx += 1;
    }

    // Check if this is a web query
    if is_url(&query.from_path) {
        return execute_web_query(query);
    }

    // Check if this is a process query
    if query.from_path == "ps" {
        let results = execute_process_query_with_subqueries(query, &subquery_results)?;
        return Ok(QueryResult::Processes(results));
    }

    // Check if this is a network query
    if query.from_path == "net" {
        let results = execute_network_query(query)?;
        return Ok(QueryResult::Network(results));
    }

    // Check if this is an application query
    if query.from_path == "applications" {
        let results = execute_application_query(query)?;
        return Ok(QueryResult::Applications(results));
    }

    let root_path = std::path::PathBuf::from(&query.from_path);
    if !root_path.exists() {
        return Err(format!("Path does not exist: {}", query.from_path));
    }

    // Parse WHERE conditions for early filtering, processing subquery placeholders
    let conditions = if let Some(where_clause) = &query.where_clause {
        let processed_where = process_where_subquery_placeholders(where_clause, &subquery_results);
        parse_compound_conditions(&processed_where)?
    } else {
        Vec::new()
    };

    let mut results = collect_files_recursive(&root_path, &root_path, &conditions)?;

    // Apply ORDER BY (only remaining filtering needed)
    if let Some(order_by) = &query.order_by {
        sort_results(&mut results, order_by, &query.order_direction)?;
    }

    // Apply LIMIT
    if let Some(limit) = query.limit {
        results.truncate(limit);
    }

    Ok(QueryResult::Files(results))
}

fn execute_delete_query(query: &SqlQuery) -> Result<QueryResult, String> {
    // Handle process deletion
    if query.from_path == "ps" {
        return execute_delete_process_query(query);
    }

    // Handle filesystem deletion
    let root_path = std::path::PathBuf::from(&query.from_path);
    if !root_path.exists() {
        return Err(format!("Path does not exist: {}", query.from_path));
    }

    // Parse WHERE conditions
    let conditions = if let Some(where_clause) = &query.where_clause {
        parse_compound_conditions(where_clause)?
    } else {
        Vec::new()
    };

    // Collect files to delete
    let files_to_delete = collect_files_recursive(&root_path, &root_path, &conditions)?;

    if files_to_delete.is_empty() {
        return Ok(QueryResult::Files(Vec::new()));
    }

    // For multiple files, prompt for confirmation
    if files_to_delete.len() > 1 {
        println!(
            "You are about to delete {} files. Are you sure? (y/N)",
            files_to_delete.len()
        );
        for file in &files_to_delete {
            println!("  {}", file.path);
        }

        print!("> ");
        io::stdout()
            .flush()
            .map_err(|e| format!("Failed to flush stdout: {}", e))?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| format!("Failed to read input: {}", e))?;

        if !input.trim().to_lowercase().starts_with('y') {
            println!("Deletion cancelled.");
            return Ok(QueryResult::Files(Vec::new()));
        }
    }

    // Delete the files/directories
    let mut deleted_files = Vec::new();
    for file_info in &files_to_delete {
        let full_path = root_path.join(&file_info.path);
        if full_path.is_dir() {
            if let Err(e) = fs::remove_dir_all(&full_path) {
                eprintln!("Failed to delete directory {}: {}", full_path.display(), e);
            } else {
                deleted_files.push(file_info.clone());
            }
        } else if let Err(e) = fs::remove_file(&full_path) {
            eprintln!("Failed to delete file {}: {}", full_path.display(), e);
        } else {
            deleted_files.push(file_info.clone());
        }
    }

    println!("Deleted {} items.", deleted_files.len());
    Ok(QueryResult::Files(deleted_files))
}

fn execute_delete_process_query(query: &SqlQuery) -> Result<QueryResult, String> {
    use sysinfo::{ProcessRefreshKind, Signal, System};

    let conditions = if let Some(where_clause) = &query.where_clause {
        parse_compound_conditions(where_clause)?
    } else {
        Vec::new()
    };

    let mut system = System::new_all();
    system.refresh_processes_specifics(
        ProcessRefreshKind::everything()
            .without_disk_usage()
            .without_environ(),
    );

    let mut processes_to_kill = Vec::new();

    for (pid, process) in system.processes() {
        let process_info = crate::models::ProcessInfo::new(
            pid.as_u32(),
            process.name(),
            process.cpu_usage(),
            process.memory(),
            match process.status() {
                sysinfo::ProcessStatus::Run => "running",
                sysinfo::ProcessStatus::Sleep => "sleeping",
                sysinfo::ProcessStatus::Idle => "idle",
                sysinfo::ProcessStatus::Zombie => "zombie",
                sysinfo::ProcessStatus::Stop => "stopped",
                _ => "unknown",
            },
        );

        if evaluate_process_conditions(&process_info, &conditions) {
            processes_to_kill.push((pid, process_info));
        }
    }

    let mut killed_processes = Vec::new();
    for (pid, process_info) in processes_to_kill {
        if system
            .process(*pid)
            .unwrap()
            .kill_with(Signal::Term)
            .unwrap_or(false)
        {
            killed_processes.push(process_info);
        } else {
            eprintln!(
                "Failed to kill process {} ({})",
                process_info.name, process_info.pid
            );
        }
    }

    println!("Killed {} processes.", killed_processes.len());
    Ok(QueryResult::Processes(killed_processes))
}

fn evaluate_process_conditions(
    process: &crate::models::ProcessInfo,
    conditions: &[Condition],
) -> bool {
    for condition in conditions {
        let result = evaluate_single_process_condition(process, condition);
        let final_result = if condition.negated { !result } else { result };

        if !final_result {
            return false;
        }
    }
    true
}

fn evaluate_single_process_condition(
    process: &crate::models::ProcessInfo,
    condition: &Condition,
) -> bool {
    match condition.field.as_str() {
        "pid" => {
            if condition.operator == "LIKE" {
                like_match(&process.pid, &condition.value)
            } else {
                compare_strings(&process.pid, &condition.operator, &condition.value)
            }
        }
        "name" => {
            if condition.operator == "LIKE" {
                like_match(&process.name, &condition.value)
            } else {
                compare_strings(&process.name, &condition.operator, &condition.value)
            }
        }
        "cpu_usage" => {
            // Extract numeric value from "X.X%" format
            if let Some(cpu_str) = process.cpu_usage.strip_suffix('%') {
                if let Ok(cpu_val) = cpu_str.parse::<f32>() {
                    if let Ok(compare_val) = condition.value.parse::<f32>() {
                        match condition.operator.as_str() {
                            "=" => (cpu_val - compare_val).abs() < 0.1, // Allow small floating point differences
                            "!=" => (cpu_val - compare_val).abs() >= 0.1,
                            ">" => cpu_val > compare_val,
                            "<" => cpu_val < compare_val,
                            ">=" => cpu_val >= compare_val,
                            "<=" => cpu_val <= compare_val,
                            _ => false,
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        }
        "memory_usage" => {
            // For memory comparison, extract numeric value
            if let Ok(process_memory) = crate::processes::parse_memory(&process.memory_usage) {
                if let Ok(compare_memory) = crate::processes::parse_memory(&condition.value) {
                    match condition.operator.as_str() {
                        "=" => process_memory == compare_memory,
                        "!=" => process_memory != compare_memory,
                        ">" => process_memory > compare_memory,
                        "<" => process_memory < compare_memory,
                        ">=" => process_memory >= compare_memory,
                        "<=" => process_memory <= compare_memory,
                        _ => false,
                    }
                } else {
                    false
                }
            } else {
                false
            }
        }
        "status" => compare_strings(&process.status, &condition.operator, &condition.value),
        _ => false,
    }
}

fn collect_files_recursive(
    root_path: &Path,
    current_path: &Path,
    conditions: &[Condition],
) -> Result<Vec<FileInfo>, String> {
    let results = Mutex::new(Vec::new());

    // Create a lightweight FileInfo for early filtering
    let temp_file_info = FileInfo::new_lightweight(current_path, root_path);

    // Early filtering: check conditions that can be evaluated with lightweight info
    // Skip depth filtering in early phase since it's not performance-critical
    let should_process = if let Some(ref file_info) = temp_file_info {
        let mut matches = true;
        for condition in conditions {
            // Only check path conditions in early filtering (depth is handled later)
            if condition.field == "path" {
                let result = evaluate_single_condition(file_info, condition);
                let final_result = if condition.negated { !result } else { result };
                if !final_result {
                    matches = false;
                    break;
                }
            }
        }
        matches
    } else {
        true // If we can't get file info, process anyway (permission errors)
    };

    if !should_process {
        return Ok(Vec::new()); // Skip this path entirely
    }

    // For directories, check if we should recurse based on path filters
    let should_recurse = if current_path.is_dir() {
        // If we have path conditions that exclude certain directories, check them
        let mut recurse = true;
        for condition in conditions {
            if condition.field == "path" && condition.operator == "LIKE" && !condition.negated {
                // For LIKE conditions, if the path pattern would exclude subdirectories, we might skip
                // This is a simplified check - in practice we'd need more sophisticated analysis
                if condition.value.contains("%target/%") {
                    // Skip recursing into target directories
                    recurse = false;
                    break;
                }
            }
        }
        recurse
    } else {
        false
    };

    // Add current file/directory if it passes all filtering conditions
    if let Some(mut file_info) = temp_file_info {
        let mut matches = true;
        for condition in conditions {
            let result = evaluate_single_condition(&file_info, condition);
            let final_result = if condition.negated { !result } else { result };
            if !final_result {
                matches = false;
                break;
            }
        }

        if matches {
            // Upgrade to full metadata only for files that match our criteria
            file_info.upgrade_to_full(current_path);
            results.lock().unwrap().push(file_info);
        }
    }

    // If it's a directory and we should recurse, process children in parallel
    if should_recurse {
        if let Ok(entries) = fs::read_dir(current_path) {
            let child_paths: Vec<std::path::PathBuf> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .collect();

            // Process children in parallel
            child_paths.into_par_iter().for_each(|path| {
                if let Ok(mut sub_results) = collect_files_recursive(root_path, &path, conditions) {
                    results.lock().unwrap().append(&mut sub_results);
                }
            });
        }
    }

    Ok(results.into_inner().unwrap())
}

/// Execute process query with subquery support
fn execute_process_query_with_subqueries(
    query: &SqlQuery,
    _subquery_results: &HashMap<String, QueryResult>,
) -> Result<Vec<ProcessInfo>, String> {
    // For now, delegate to the existing process query execution
    // This would need to be enhanced to handle subqueries similar to filesystem queries
    execute_process_query(query)
}

/// Process WHERE clause subquery placeholders and replace them with actual values
fn process_where_subquery_placeholders(
    where_clause: &str,
    subquery_results: &HashMap<String, QueryResult>,
) -> String {
    let mut processed = where_clause.to_string();

    // Handle IN subqueries
    for (placeholder, result) in subquery_results {
        if placeholder.starts_with("__SUBQUERY_") {
            if let QueryResult::Files(files) = result {
                let values: Vec<String> = files
                    .iter()
                    .map(|f| format!("'{}'", f.name.replace("'", "''")))
                    .collect();
                let replacement = if values.is_empty() {
                    "NULL".to_string()
                } else {
                    format!("({})", values.join(", "))
                };
                processed = processed.replace(placeholder, &replacement);
            } else if let QueryResult::Processes(processes) = result {
                let values: Vec<String> = processes.iter().map(|p| p.pid.clone()).collect();
                let replacement = if values.is_empty() {
                    "NULL".to_string()
                } else {
                    format!("({})", values.join(", "))
                };
                processed = processed.replace(placeholder, &replacement);
            } else if let QueryResult::Applications(apps) = result {
                let values: Vec<String> = apps
                    .iter()
                    .map(|a| format!("'{}'", a.name.replace("'", "''")))
                    .collect();
                let replacement = if values.is_empty() {
                    "NULL".to_string()
                } else {
                    format!("({})", values.join(", "))
                };
                processed = processed.replace(placeholder, &replacement);
            }
        }
    }

    // Handle EXISTS subqueries - replace with TRUE/FALSE
    for (placeholder, result) in subquery_results {
        if placeholder.starts_with("__EXISTS_SUBQUERY_") {
            let has_results = match result {
                QueryResult::Files(files) => !files.is_empty(),
                QueryResult::Processes(processes) => !processes.is_empty(),
                QueryResult::Network(network_info) => !network_info.is_empty(),
                QueryResult::Applications(apps) => !apps.is_empty(),
            };
            let replacement = if has_results { "TRUE" } else { "FALSE" };
            processed = processed.replace(placeholder, replacement);
        }
    }

    // Handle scalar subqueries - replace with single value
    for (placeholder, result) in subquery_results {
        if placeholder.starts_with("__SCALAR_SUBQUERY_") {
            let replacement = match result {
                QueryResult::Files(files) => {
                    if files.is_empty() {
                        "NULL".to_string()
                    } else {
                        // For scalar subqueries, we take the first result's name
                        // This could be enhanced to support specific field selection
                        format!("'{}'", files[0].name.replace("'", "''"))
                    }
                }
                QueryResult::Processes(processes) => {
                    if processes.is_empty() {
                        "NULL".to_string()
                    } else {
                        processes[0].pid.clone()
                    }
                }
                QueryResult::Network(network_info) => {
                    if network_info.is_empty() {
                        "NULL".to_string()
                    } else {
                        // For scalar subqueries, we return the port number
                        network_info[0].port.clone()
                    }
                }
                QueryResult::Applications(apps) => {
                    if apps.is_empty() {
                        "NULL".to_string()
                    } else {
                        // For scalar subqueries, we take the first result's name
                        format!("'{}'", apps[0].name.replace("'", "''"))
                    }
                }
            };
            processed = processed.replace(placeholder, &replacement);
        }
    }

    processed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Condition, QueryType, SqlQuery};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_early_filtering_path_exclusion() {
        use std::path::Path;

        let conditions = vec![Condition {
            field: "path".to_string(),
            operator: "LIKE".to_string(),
            value: "%target/%".to_string(),
            negated: true, // NOT LIKE '%target/%'
        }];

        // This should be filtered out early
        let target_path = Path::new("/tmp/target/debug/main.rs");
        let root_path = Path::new("/tmp");

        let result = collect_files_recursive(root_path, target_path, &conditions);
        assert!(result.is_ok());
        // Should return empty vec since path is filtered
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_execute_delete_query_file_not_found() {
        let query = SqlQuery {
            query_type: QueryType::Delete,
            select_fields: Vec::new(),
            select_field_aliases: Vec::new(),
            select_subqueries: Vec::new(),
            from_path: "/nonexistent/path".to_string(),
            where_clause: None,
            where_subqueries: Vec::new(),
            order_by: None,
            order_direction: crate::models::SortDirection::Ascending,
            limit: None,
            distinct: false,
        };

        let result = execute_delete_query(&query);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Path does not exist"));
    }

    #[test]
    fn test_execute_delete_query_empty_results() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_string_lossy().to_string();

        let query = SqlQuery {
            query_type: QueryType::Delete,
            select_fields: Vec::new(),
            select_field_aliases: Vec::new(),
            select_subqueries: Vec::new(),
            from_path: temp_path,
            where_clause: Some("name = 'nonexistent.txt'".to_string()),
            where_subqueries: Vec::new(),
            order_by: None,
            order_direction: crate::models::SortDirection::Ascending,
            limit: None,
            distinct: false,
        };

        let result = execute_delete_query(&query);
        assert!(result.is_ok());
        let query_result = result.unwrap();
        match query_result {
            QueryResult::Files(files) => assert!(files.is_empty()),
            _ => panic!("Expected Files result"),
        }
    }

    #[test]
    fn test_execute_delete_query_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a test file
        let test_file = temp_path.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let query = SqlQuery {
            query_type: QueryType::Delete,
            select_fields: Vec::new(),
            select_field_aliases: Vec::new(),
            select_subqueries: Vec::new(),
            from_path: temp_path.to_string_lossy().to_string(),
            where_clause: Some("name = 'test.txt'".to_string()),
            where_subqueries: Vec::new(),
            order_by: None,
            order_direction: crate::models::SortDirection::Ascending,
            limit: None,
            distinct: false,
        };

        let result = execute_delete_query(&query);
        assert!(result.is_ok());
        let query_result = result.unwrap();
        match query_result {
            QueryResult::Files(files) => {
                assert_eq!(files.len(), 1);
                assert_eq!(files[0].name, "test.txt");
                // File should be deleted
                assert!(!test_file.exists());
            }
            _ => panic!("Expected Files result"),
        }
    }

    #[test]
    fn test_execute_delete_query_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a test directory
        let test_dir = temp_path.join("testdir");
        fs::create_dir(&test_dir).unwrap();
        let nested_file = test_dir.join("nested.txt");
        fs::write(&nested_file, "nested content").unwrap();

        let query = SqlQuery {
            query_type: QueryType::Delete,
            select_fields: Vec::new(),
            select_field_aliases: Vec::new(),
            select_subqueries: Vec::new(),
            from_path: temp_path.to_string_lossy().to_string(),
            where_clause: Some("name = 'testdir'".to_string()),
            where_subqueries: Vec::new(),
            order_by: None,
            order_direction: crate::models::SortDirection::Ascending,
            limit: None,
            distinct: false,
        };

        let result = execute_delete_query(&query);
        assert!(result.is_ok());
        let query_result = result.unwrap();
        match query_result {
            QueryResult::Files(files) => {
                assert_eq!(files.len(), 1);
                assert_eq!(files[0].name, "testdir");
                // Directory should be deleted
                assert!(!test_dir.exists());
            }
            _ => panic!("Expected Files result"),
        }
    }

    #[test]
    fn test_evaluate_process_conditions() {
        let process = crate::models::ProcessInfo::new(1234, "node", 5.5, 1024 * 1024, "running");

        let conditions = vec![Condition {
            field: "name".to_string(),
            operator: "=".to_string(),
            value: "node".to_string(),
            negated: false,
        }];

        assert!(evaluate_process_conditions(&process, &conditions));

        // Test non-matching condition
        let bad_conditions = vec![Condition {
            field: "name".to_string(),
            operator: "=".to_string(),
            value: "python".to_string(),
            negated: false,
        }];

        assert!(!evaluate_process_conditions(&process, &bad_conditions));
    }

    #[test]
    fn test_like_match() {
        assert!(like_match("test.txt", "%.txt"));
        assert!(like_match("hello", "h%"));
        assert!(!like_match("test.txt", "%.rs"));
        assert!(like_match("main.rs", "main.%"));
    }

    #[test]
    fn test_compare_strings() {
        assert!(compare_strings("abc", "=", "abc"));
        assert!(compare_strings("abc", "!=", "def"));
        assert!(compare_strings("abc", ">", "abb"));
        assert!(compare_strings("abc", "<", "abd"));
        assert!(compare_strings("abc", ">=", "abc"));
        assert!(compare_strings("abc", "<=", "abc"));
    }

    #[test]
    fn test_process_where_subquery_placeholders() {
        let mut subquery_results = HashMap::new();

        // Test IN subquery with files - create proper FileInfo structs
        let file1 = FileInfo {
            name: "test1.txt".to_string(),
            file_type: "file".to_string(),
            modified_date: chrono::Utc::now(),
            permissions: "644".to_string(),
            size: "100 B".to_string(),
            path: "test1.txt".to_string(),
            depth: 1,
            extension: Some("txt".to_string()),
        };
        let file2 = FileInfo {
            name: "test2.txt".to_string(),
            file_type: "file".to_string(),
            modified_date: chrono::Utc::now(),
            permissions: "644".to_string(),
            size: "200 B".to_string(),
            path: "test2.txt".to_string(),
            depth: 1,
            extension: Some("txt".to_string()),
        };
        let files = vec![file1, file2];
        subquery_results.insert("__SUBQUERY_0__".to_string(), QueryResult::Files(files));

        // Test EXISTS subquery
        let empty_files = Vec::new();
        subquery_results.insert(
            "__EXISTS_SUBQUERY_1__".to_string(),
            QueryResult::Files(empty_files),
        );

        let processes = vec![ProcessInfo::new(1234, "node", 5.5, 1024, "running")];
        subquery_results.insert(
            "__EXISTS_SUBQUERY_2__".to_string(),
            QueryResult::Processes(processes),
        );

        // Test scalar subquery
        let scalar_file = FileInfo {
            name: "scalar.txt".to_string(),
            file_type: "file".to_string(),
            modified_date: chrono::Utc::now(),
            permissions: "644".to_string(),
            size: "50 B".to_string(),
            path: "scalar.txt".to_string(),
            depth: 1,
            extension: Some("txt".to_string()),
        };
        let scalar_files = vec![scalar_file];
        subquery_results.insert(
            "__SCALAR_SUBQUERY_3__".to_string(),
            QueryResult::Files(scalar_files),
        );

        let where_clause = "name IN __SUBQUERY_0__ AND __EXISTS_SUBQUERY_1__ AND __EXISTS_SUBQUERY_2__ AND size > __SCALAR_SUBQUERY_3__";
        let processed = process_where_subquery_placeholders(where_clause, &subquery_results);

        assert!(processed.contains("name IN ('test1.txt', 'test2.txt')"));
        assert!(processed.contains("FALSE"));
        assert!(processed.contains("TRUE"));
        assert!(processed.contains("size > 'scalar.txt'"));
    }

    #[test]
    fn test_subquery_parsing() {
        // For now, test that basic parsing still works
        // Subquery parsing will be enhanced in future iterations
        let query_str = "SELECT name FROM /tmp WHERE type = 'file'";
        let query = crate::parser::parse_query(query_str).unwrap();

        assert_eq!(query.where_subqueries.len(), 0);
        assert_eq!(query.select_subqueries.len(), 0);
    }

    #[test]
    fn test_depth_filtering() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory structure
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create structure: temp/dir1/dir2/file.txt
        let dir1_path = temp_path.join("dir1");
        fs::create_dir(&dir1_path).unwrap();
        let dir2_path = dir1_path.join("dir2");
        fs::create_dir(&dir2_path).unwrap();
        let file_path = dir2_path.join("file.txt");
        fs::write(&file_path, "test content").unwrap();

        // Test depth = 1 filter (should return dir1)
        let conditions = vec![Condition {
            field: "depth".to_string(),
            operator: "=".to_string(),
            value: "1".to_string(),
            negated: false,
        }];

        let results = collect_files_recursive(temp_path, temp_path, &conditions).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "dir1");
        assert_eq!(results[0].depth, 1);

        // Test depth = 3 filter (should return file.txt)
        let conditions = vec![Condition {
            field: "depth".to_string(),
            operator: "=".to_string(),
            value: "3".to_string(),
            negated: false,
        }];

        let results = collect_files_recursive(temp_path, temp_path, &conditions).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "file.txt");
        assert_eq!(results[0].depth, 3);

        // Test depth > 2 filter (should return file.txt)
        let conditions = vec![Condition {
            field: "depth".to_string(),
            operator: ">".to_string(),
            value: "2".to_string(),
            negated: false,
        }];

        let results = collect_files_recursive(temp_path, temp_path, &conditions).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "file.txt");
        assert_eq!(results[0].depth, 3);
    }

    #[test]
    fn test_select_subquery_parsing() {
        // For now, test that basic parsing still works
        // SELECT subquery parsing will be enhanced in future iterations
        let query_str = "SELECT name FROM /tmp";
        let query = crate::parser::parse_query(query_str).unwrap();

        assert_eq!(query.select_subqueries.len(), 0);
        assert_eq!(query.select_fields, vec!["name".to_string()]);
    }
}
