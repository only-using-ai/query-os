use crate::models::{Condition, NetInfo};
use crate::parser::parse_compound_conditions;
use crate::utils::{compare_strings, like_match};
use std::process::Command;

pub fn execute_network_query(query: &crate::models::SqlQuery) -> Result<Vec<NetInfo>, String> {
    let conditions = if let Some(where_clause) = &query.where_clause {
        parse_compound_conditions(where_clause)?
    } else {
        Vec::new()
    };

    let network_info = collect_network_info()?;
    let mut results: Vec<NetInfo> = Vec::new();

    // Apply WHERE conditions
    for net_info in network_info {
        if evaluate_network_conditions(&net_info, &conditions) {
            results.push(net_info);
        }
    }

    // Apply DISTINCT
    if query.distinct {
        let mut seen = std::collections::HashSet::new();
        let mut unique_results = Vec::new();

        for net_info in results {
            // Create a key from all selected fields for DISTINCT comparison
            let key = if query.select_fields.contains(&"*".to_string()) {
                format!("{}|{}|{}", net_info.name, net_info.port, net_info.pid)
            } else {
                let mut key_parts = Vec::new();
                for field in &query.select_fields {
                    match field.as_str() {
                        "name" => key_parts.push(net_info.name.clone()),
                        "port" => key_parts.push(net_info.port.clone()),
                        "pid" => key_parts.push(net_info.pid.clone()),
                        _ => {}
                    }
                }
                key_parts.join("|")
            };

            if seen.insert(key) {
                unique_results.push(net_info);
            }
        }

        results = unique_results;
    }

    // Apply ORDER BY
    if let Some(order_by) = &query.order_by {
        sort_network_results(&mut results, order_by, &query.order_direction)?;
    }

    // Apply LIMIT
    if let Some(limit) = query.limit {
        results.truncate(limit);
    }

    Ok(results)
}

fn collect_network_info() -> Result<Vec<NetInfo>, String> {
    let mut network_info = Vec::new();

    // Try multiple commands in order of preference
    let output = if let Ok(output) = Command::new("ss").args(&["-tlnp"]).output() {
        if output.status.success() {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            try_netstat_or_lsof()?
        }
    } else {
        try_netstat_or_lsof()?
    };

    // Parse the output to extract port and PID information
    let lines: Vec<&str> = output.lines().collect();
    for line in lines.iter().skip(1) {
        // Skip header
        if let Some(net_info) = parse_network_line(line) {
            network_info.push(net_info);
        }
    }

    Ok(network_info)
}

fn try_netstat_or_lsof() -> Result<String, String> {
    // Try lsof first (works on macOS and Linux)
    if let Ok(output) = Command::new("lsof").args(&["-i", "-P", "-n"]).output() {
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }

    // Fallback to netstat
    let output = Command::new("netstat")
        .args(&["-tlnp"])
        .output()
        .map_err(|_| "Failed to run network commands".to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err("All network commands failed".to_string())
    }
}

fn parse_network_line(line: &str) -> Option<NetInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 7 {
        return None;
    }

    // Check if this is lsof output format first (COMMAND, PID, USER, FD, TYPE, DEVICE, SIZE/OFF, NODE, NAME)
    if parts.len() >= 9 && parts[0].chars().all(|c| c.is_alphabetic() || c == '-') {
        // lsof format: COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME
        let command = parts.get(0)?;
        let pid_str = parts.get(1)?;
        let name_field = parts.get(8)?; // NAME field contains IP:Port or hostname:port

        if let Ok(pid) = pid_str.parse::<u32>() {
            // Extract port from NAME field (format: IP:Port or hostname:Port or :Port)
            if let Some(port_str) = extract_port_from_lsof_name(name_field) {
                if let Ok(port) = port_str.parse::<u16>() {
                    return Some(NetInfo::new(command, port, pid));
                }
            }
        }
    }
    // Check if this looks like ss output (contains ':')
    else if line.contains(':') && parts.len() >= 5 {
        // ss command format
        let local_addr = parts.get(3)?; // Local Address:Port
        let process_info = parts.get(4..)?.join(" "); // Process info

        if let Some((_, port_str)) = local_addr.rsplit_once(':') {
            if let Ok(port) = port_str.parse::<u16>() {
                if let Some(pid) = extract_pid_from_process_info(&process_info) {
                    // Get process name from PID
                    if let Some(process_name) = get_process_name(pid) {
                        return Some(NetInfo::new(&process_name, port, pid));
                    }
                }
            }
        }
    } else if parts.len() >= 7 {
        // netstat command format
        let local_addr = parts.get(3)?; // Local Address
        let program_info = parts.get(6)?; // PID/Program name

        // Local address might be IP:Port or just Port
        let port_str = if local_addr.contains(':') {
            local_addr.split(':').last()?
        } else {
            local_addr
        };

        if let Ok(port) = port_str.parse::<u16>() {
            if let Some((pid, _)) = program_info.split_once('/') {
                if let Ok(pid_num) = pid.parse::<u32>() {
                    if let Some(process_name) = get_process_name(pid_num) {
                        return Some(NetInfo::new(&process_name, port, pid_num));
                    }
                }
            }
        }
    }

    None
}

fn extract_port_from_lsof_name(name: &str) -> Option<&str> {
    // NAME field in lsof can be:
    // - IP:Port (e.g., "127.0.0.1:3000")
    // - hostname:Port (e.g., "localhost:3000")
    // - :Port (e.g., ":3000" for listening on all interfaces)
    // - [host]:Port for IPv6

    if let Some(port_part) = name.rsplit_once(':') {
        let port_str = port_part.1;
        // Check if it's a valid port number
        if port_str.chars().all(|c| c.is_numeric()) {
            Some(port_str)
        } else {
            None
        }
    } else {
        None
    }
}

fn extract_pid_from_process_info(process_info: &str) -> Option<u32> {
    // Look for patterns like "pid=1234" or "users:(("pid",1234,"
    if let Some(pid_start) = process_info.find("pid=") {
        let pid_part = &process_info[pid_start + 4..];
        if let Some(pid_end) = pid_part.find(',') {
            pid_part[..pid_end].parse::<u32>().ok()
        } else if let Some(pid_end) = pid_part.find(')') {
            pid_part[..pid_end].parse::<u32>().ok()
        } else {
            pid_part.split_whitespace().next()?.parse::<u32>().ok()
        }
    } else if let Some(users_start) = process_info.find("users:((") {
        // Handle ss users format: users:(("process",pid,...
        let users_part = &process_info[users_start + 9..];
        if let Some(pid_start) = users_part.find(',') {
            let pid_str = &users_part[pid_start + 1..];
            if let Some(pid_end) = pid_str.find(',') {
                pid_str[..pid_end].trim_matches('"').parse::<u32>().ok()
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

fn get_process_name(pid: u32) -> Option<String> {
    // Read /proc/<pid>/comm to get process name
    let comm_path = format!("/proc/{}/comm", pid);
    std::fs::read_to_string(&comm_path)
        .ok()
        .map(|s| s.trim().to_string())
}

fn evaluate_network_conditions(net_info: &NetInfo, conditions: &[Condition]) -> bool {
    for condition in conditions {
        let result = evaluate_single_network_condition(net_info, condition);
        let final_result = if condition.negated { !result } else { result };

        if !final_result {
            return false;
        }
    }
    true
}

fn evaluate_single_network_condition(net_info: &NetInfo, condition: &Condition) -> bool {
    // Handle NULL checks first
    if condition.operator == "IS" && condition.value == "NULL" {
        let is_null = match condition.field.as_str() {
            "name" => net_info.name.is_empty(),
            "port" => net_info.port.is_empty(),
            "pid" => net_info.pid.is_empty(),
            _ => false,
        };
        // Return the base NULL check result; negation is handled by evaluate_network_conditions
        return is_null;
    }

    match condition.field.as_str() {
        "name" => {
            if condition.operator == "LIKE" {
                like_match(&net_info.name, &condition.value)
            } else {
                compare_strings(&net_info.name, &condition.operator, &condition.value)
            }
        }
        "port" => {
            if condition.operator == "LIKE" {
                like_match(&net_info.port, &condition.value)
            } else {
                // Parse ports as numbers for numeric comparison
                match (net_info.port.parse::<u16>(), condition.value.parse::<u16>()) {
                    (Ok(net_port), Ok(cond_port)) => match condition.operator.as_str() {
                        "=" => net_port == cond_port,
                        "!=" => net_port != cond_port,
                        ">" => net_port > cond_port,
                        "<" => net_port < cond_port,
                        ">=" => net_port >= cond_port,
                        "<=" => net_port <= cond_port,
                        _ => false,
                    },
                    _ => false, // If parsing fails, condition is false
                }
            }
        }
        "pid" => {
            if condition.operator == "LIKE" {
                like_match(&net_info.pid, &condition.value)
            } else {
                // Parse PIDs as numbers for numeric comparison
                match (net_info.pid.parse::<u32>(), condition.value.parse::<u32>()) {
                    (Ok(net_pid), Ok(cond_pid)) => match condition.operator.as_str() {
                        "=" => net_pid == cond_pid,
                        "!=" => net_pid != cond_pid,
                        ">" => net_pid > cond_pid,
                        "<" => net_pid < cond_pid,
                        ">=" => net_pid >= cond_pid,
                        "<=" => net_pid <= cond_pid,
                        _ => false,
                    },
                    _ => false, // If parsing fails, condition is false
                }
            }
        }
        _ => false,
    }
}

fn sort_network_results(results: &mut Vec<NetInfo>, order_by: &str, direction: &crate::models::SortDirection) -> Result<(), String> {
    // Validate order_by field first
    match order_by {
        "name" | "port" | "pid" => {},
        _ => return Err(format!("Invalid ORDER BY field: {}", order_by)),
    }

    results.sort_by(|a, b| {
        let ordering = match order_by {
            "name" => a.name.cmp(&b.name),
            "port" => {
                let a_port = a.port.parse::<u16>().unwrap_or(0);
                let b_port = b.port.parse::<u16>().unwrap_or(0);
                a_port.cmp(&b_port)
            },
            "pid" => {
                let a_pid = a.pid.parse::<u32>().unwrap_or(0);
                let b_pid = b.pid.parse::<u32>().unwrap_or(0);
                a_pid.cmp(&b_pid)
            },
            _ => std::cmp::Ordering::Equal, // Should not happen due to validation above
        };

        // Reverse ordering for descending sort
        match direction {
            crate::models::SortDirection::Descending => ordering.reverse(),
            crate::models::SortDirection::Ascending => ordering,
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Condition;

    #[test]
    fn test_net_info_new() {
        let net_info = NetInfo::new("node", 3000, 1234);
        assert_eq!(net_info.name, "node");
        assert_eq!(net_info.port, "3000");
        assert_eq!(net_info.pid, "1234");
    }

    #[test]
    fn test_evaluate_network_conditions() {
        let net_info = NetInfo::new("node", 3000, 1234);

        let conditions = vec![Condition {
            field: "port".to_string(),
            operator: "=".to_string(),
            value: "3000".to_string(),
            negated: false,
        }];

        assert!(evaluate_network_conditions(&net_info, &conditions));

        // Test non-matching condition
        let bad_conditions = vec![Condition {
            field: "port".to_string(),
            operator: "=".to_string(),
            value: "8080".to_string(),
            negated: false,
        }];

        assert!(!evaluate_network_conditions(&net_info, &bad_conditions));
    }

    #[test]
    fn test_sort_network_results() {
        let mut results = vec![
            NetInfo::new("node", 8080, 2000),
            NetInfo::new("apache", 80, 1000),
            NetInfo::new("nginx", 443, 1500),
        ];

        // Sort by port
        sort_network_results(&mut results, "port", &crate::models::SortDirection::Ascending).unwrap();
        assert_eq!(results[0].port, "80");
        assert_eq!(results[1].port, "443");
        assert_eq!(results[2].port, "8080");

        // Sort by name
        sort_network_results(&mut results, "name", &crate::models::SortDirection::Ascending).unwrap();
        assert_eq!(results[0].name, "apache");
        assert_eq!(results[1].name, "nginx");
        assert_eq!(results[2].name, "node");
    }
}
