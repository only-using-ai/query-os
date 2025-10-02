use crate::models::{Condition, ProcessInfo};
use crate::parser::parse_compound_conditions;
use crate::utils::{compare_strings, like_match, sort_process_results};
use sysinfo::{ProcessRefreshKind, System};

pub fn execute_process_query(query: &crate::models::SqlQuery) -> Result<Vec<ProcessInfo>, String> {
    let conditions = if let Some(where_clause) = &query.where_clause {
        parse_compound_conditions(where_clause)?
    } else {
        Vec::new()
    };

    let processes = collect_processes()?;
    let mut results: Vec<ProcessInfo> = Vec::new();

    // Apply WHERE conditions
    for process in processes {
        if evaluate_process_conditions(&process, &conditions) {
            results.push(process);
        }
    }

    // Apply ORDER BY
    if let Some(order_by) = &query.order_by {
        sort_process_results(&mut results, order_by, &query.order_direction)?;
    }

    // Apply LIMIT
    if let Some(limit) = query.limit {
        results.truncate(limit);
    }

    Ok(results)
}

fn collect_processes() -> Result<Vec<ProcessInfo>, String> {
    let mut system = System::new_all();
    system.refresh_processes_specifics(
        ProcessRefreshKind::everything()
            .without_disk_usage()
            .without_environ(),
    );

    let mut processes = Vec::new();

    for (pid, process) in system.processes() {
        let status = match process.status() {
            sysinfo::ProcessStatus::Run => "running",
            sysinfo::ProcessStatus::Sleep => "sleeping",
            sysinfo::ProcessStatus::Idle => "idle",
            sysinfo::ProcessStatus::Zombie => "zombie",
            sysinfo::ProcessStatus::Stop => "stopped",
            _ => "unknown",
        };

        let process_info = ProcessInfo::new(
            pid.as_u32(),
            process.name(),
            process.cpu_usage(),
            process.memory(),
            status,
        );

        processes.push(process_info);
    }

    Ok(processes)
}

fn evaluate_process_conditions(process: &ProcessInfo, conditions: &[Condition]) -> bool {
    for condition in conditions {
        let result = evaluate_single_process_condition(process, condition);
        let final_result = if condition.negated { !result } else { result };

        if !final_result {
            return false;
        }
    }
    true
}

fn evaluate_single_process_condition(process: &ProcessInfo, condition: &Condition) -> bool {
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
            if let Ok(process_memory) = parse_memory(&process.memory_usage) {
                if let Ok(compare_memory) = parse_memory(&condition.value) {
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

pub fn parse_memory(memory_str: &str) -> Result<f64, String> {
    let re = regex::Regex::new(r"([\d.]+)\s*(B|KB|MB|GB|TB)?").unwrap();
    if let Some(caps) = re.captures(memory_str) {
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
            _ => return Err(format!("Invalid memory unit: {}", unit)),
        };

        Ok(num * multiplier)
    } else {
        Err("Invalid memory format".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Condition;

    #[test]
    fn test_parse_memory() {
        assert_eq!(parse_memory("512 B").unwrap(), 512.0);
        assert_eq!(parse_memory("1.00 KB").unwrap(), 1024.0);
        assert_eq!(parse_memory("1 MB").unwrap(), 1024.0 * 1024.0);
    }

    #[test]
    fn test_evaluate_process_conditions() {
        let process = ProcessInfo::new(1234, "node", 5.5, 1024 * 1024, "running");

        let conditions = vec![
            Condition {
                field: "name".to_string(),
                operator: "LIKE".to_string(),
                value: "node".to_string(),
                negated: false,
            },
            Condition {
                field: "status".to_string(),
                operator: "=".to_string(),
                value: "running".to_string(),
                negated: false,
            },
        ];

        assert!(evaluate_process_conditions(&process, &conditions));

        // Test with a condition that should NOT match
        let bad_conditions = vec![Condition {
            field: "name".to_string(),
            operator: "=".to_string(),
            value: "python".to_string(),
            negated: false,
        }];

        assert!(!evaluate_process_conditions(&process, &bad_conditions));
    }

    #[test]
    fn test_execute_process_query_basic() {
        use crate::models::SqlQuery;

        let query = SqlQuery {
            query_type: crate::models::QueryType::Select,
            select_fields: vec!["pid".to_string(), "name".to_string()],
            select_field_aliases: vec![None, None],
            select_subqueries: Vec::new(),
            from_path: "ps".to_string(),
            where_clause: None,
            where_subqueries: Vec::new(),
            order_by: None,
            order_direction: crate::models::SortDirection::Ascending,
            limit: Some(2),
            distinct: false,
        };

        let result = execute_process_query(&query);
        assert!(result.is_ok());

        let processes = result.unwrap();
        assert!(processes.len() <= 2); // Should be limited to 2

        // Check that the returned processes have the expected fields
        for process in processes {
            assert!(!process.pid.is_empty());
            assert!(!process.name.is_empty());
        }
    }

    #[test]
    fn test_execute_process_query_with_where() {
        use crate::models::SqlQuery;

        let query = SqlQuery {
            query_type: crate::models::QueryType::Select,
            select_fields: vec!["pid".to_string(), "name".to_string(), "status".to_string()],
            select_field_aliases: vec![None, None, None],
            select_subqueries: Vec::new(),
            from_path: "ps".to_string(),
            where_clause: Some("status = 'running'".to_string()),
            where_subqueries: Vec::new(),
            order_by: None,
            order_direction: crate::models::SortDirection::Ascending,
            limit: Some(3),
            distinct: false,
        };

        let result = execute_process_query(&query);
        assert!(result.is_ok());

        let processes = result.unwrap();
        // Should only include running processes (or fewer if limit is reached)
        for process in processes {
            assert_eq!(process.status, "running");
        }
    }
}
