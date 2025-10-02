use crate::models::{ApplicationInfo, Condition, SqlQuery};
use crate::parser::parse_compound_conditions;
use crate::utils::evaluate_single_condition;
use std::io::Cursor;
use std::path::Path;

pub fn execute_application_query(query: &SqlQuery) -> Result<Vec<ApplicationInfo>, String> {
    // Parse WHERE conditions early for optimization
    let conditions = if let Some(where_clause) = &query.where_clause {
        parse_compound_conditions(where_clause)?
    } else {
        Vec::new()
    };

    // Check if we need expensive metadata (size)
    let needs_size = query.select_fields.contains(&"size".to_string())
        || query.select_fields.contains(&"*".to_string())
        || conditions.iter().any(|c| c.field == "size");

    // Get all installed applications with optimized metadata loading
    let all_apps = get_installed_applications_optimized(needs_size)?;

    // Apply WHERE filtering
    let mut filtered_apps: Vec<ApplicationInfo> = all_apps
        .into_iter()
        .filter(|app| evaluate_application_conditions(app, &conditions))
        .collect();

    // Apply ORDER BY
    if let Some(order_by) = &query.order_by {
        sort_application_results(&mut filtered_apps, order_by, &query.order_direction)?;
    }

    // Apply LIMIT
    if let Some(limit) = query.limit {
        filtered_apps.truncate(limit);
    }

    Ok(filtered_apps)
}

fn get_installed_applications_optimized(needs_size: bool) -> Result<Vec<ApplicationInfo>, String> {
    #[cfg(target_os = "macos")]
    {
        get_macos_applications(needs_size)
    }

    #[cfg(target_os = "linux")]
    {
        get_linux_applications(needs_size)
    }

    #[cfg(target_os = "windows")]
    {
        get_windows_applications(needs_size)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err("Application querying is not supported on this platform".to_string())
    }
}

#[cfg(target_os = "macos")]
fn get_macos_applications(needs_size: bool) -> Result<Vec<ApplicationInfo>, String> {
    use std::fs;
    use std::path::Path;

    let mut applications = Vec::new();

    // Common macOS application directories
    let home_apps = format!("{}/Applications", std::env::var("HOME").unwrap_or_default());
    let app_dirs = vec!["/Applications", "/System/Applications", &home_apps];

    // Use parallel processing for better performance
    use rayon::prelude::*;

    let results: Vec<Result<Vec<ApplicationInfo>, String>> = app_dirs
        .into_par_iter()
        .map(|dir| {
            let mut dir_apps = Vec::new();
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("app") {
                        if let Some(app_info) = parse_macos_app_bundle(&path, needs_size) {
                            dir_apps.push(app_info);
                        }
                    }
                }
            }
            Ok(dir_apps)
        })
        .collect();

    // Collect all results
    for result in results {
        match result {
            Ok(mut dir_apps) => applications.append(&mut dir_apps),
            Err(e) => return Err(e),
        }
    }

    Ok(applications)
}

#[cfg(target_os = "macos")]
fn parse_macos_app_bundle(path: &Path, needs_size: bool) -> Option<ApplicationInfo> {
    use plist::Value;
    use std::fs;

    let info_plist_path = path.join("Contents/Info.plist");

    if !info_plist_path.exists() {
        return None;
    }

    // Try to read and parse the Info.plist
    if let Ok(plist_data) = fs::read(&info_plist_path) {
        if let Ok(plist) = Value::from_reader(Cursor::new(&plist_data)) {
            if let Value::Dictionary(dict) = plist {
                let name = dict
                    .get("CFBundleDisplayName")
                    .or_else(|| dict.get("CFBundleName"))
                    .and_then(|v| v.as_string())
                    .unwrap_or_else(|| {
                        path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Unknown")
                    });

                let version = dict
                    .get("CFBundleVersion")
                    .or_else(|| dict.get("CFBundleShortVersionString"))
                    .and_then(|v| v.as_string())
                    .map(|s| s.to_string());

                let category = dict
                    .get("LSApplicationCategoryType")
                    .and_then(|v| v.as_string())
                    .map(|s| s.to_string());

                // Get bundle size only if needed (expensive operation)
                let size = if needs_size {
                    Some(get_directory_size_fast(path))
                } else {
                    None
                };

                return Some(ApplicationInfo::new(
                    &name,
                    version,
                    &path.to_string_lossy(),
                    size,
                    category,
                ));
            }
        }
    }

    // Fallback: just use the bundle name without plist info
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown");

    let size = if needs_size {
        Some(get_directory_size_fast(path))
    } else {
        None
    };

    Some(ApplicationInfo::new(
        &name,
        None,
        &path.to_string_lossy(),
        size,
        None,
    ))
}

#[cfg(target_os = "linux")]
fn get_linux_applications(needs_size: bool) -> Result<Vec<ApplicationInfo>, String> {
    use std::fs;
    use std::path::Path;

    let mut applications = Vec::new();

    // Common Linux application directories
    let home_apps = format!(
        "{}/.local/share/applications",
        std::env::var("HOME").unwrap_or_default()
    );
    let app_dirs = vec![
        "/usr/share/applications",
        "/usr/local/share/applications",
        &home_apps,
    ];

    for dir in app_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Some(app_info) = parse_linux_desktop_file(&path, needs_size) {
                        applications.push(app_info);
                    }
                }
            }
        }
    }

    Ok(applications)
}

#[cfg(target_os = "linux")]
fn parse_linux_desktop_file(path: &Path, needs_size: bool) -> Option<ApplicationInfo> {
    use std::fs;

    if let Ok(content) = fs::read_to_string(path) {
        let mut name = None;
        let mut exec = None;
        let mut categories = None;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("Name=") {
                name = Some(line[5..].to_string());
            } else if line.starts_with("Exec=") {
                exec = Some(line[5..].to_string());
            } else if line.starts_with("Categories=") {
                categories = Some(line[11..].to_string());
            }
        }

        if let Some(app_name) = name {
            // Try to find the actual executable path
            let exec_path = if let Some(exec_cmd) = &exec {
                // Extract the executable name from the Exec line
                exec_cmd.split_whitespace().next().unwrap_or(exec_cmd)
            } else {
                ""
            };

            // For now, we'll use the desktop file path as the application path
            // In a more complete implementation, we'd resolve the Exec path
            let resolved_path = if !exec_path.is_empty() && Path::new(exec_path).exists() {
                exec_path.to_string()
            } else {
                path.to_string_lossy().to_string()
            };

            // Get file size only if needed
            let size = if needs_size {
                if Path::new(&resolved_path).exists() {
                    Some(get_file_size(Path::new(&resolved_path)))
                } else {
                    None
                }
            } else {
                None
            };

            // Parse categories (take the first one)
            let category = categories
                .as_ref()
                .and_then(|cats| cats.split(';').next())
                .map(|s| s.to_string());

            return Some(ApplicationInfo::new(
                &app_name,
                None, // Version not typically available in desktop files
                &resolved_path,
                size,
                category,
            ));
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn get_windows_applications(_needs_size: bool) -> Result<Vec<ApplicationInfo>, String> {
    use std::path::Path;
    use winreg::enums::*;
    use winreg::RegKey;

    let mut applications = Vec::new();

    // Query Windows registry for installed applications
    if let Ok(hkcu) = RegKey::predef(HKEY_CURRENT_USER) {
        if let Ok(uninstall_key) =
            hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall")
        {
            for subkey_name in uninstall_key.enum_keys().flatten() {
                if let Ok(subkey) = uninstall_key.open_subkey(&subkey_name) {
                    if let Some(app_info) = parse_windows_registry_entry(&subkey, &subkey_name) {
                        applications.push(app_info);
                    }
                }
            }
        }
    }

    // Also check HKLM for system-wide applications
    if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE) {
        if let Ok(uninstall_key) =
            hklm.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall")
        {
            for subkey_name in uninstall_key.enum_keys().flatten() {
                if let Ok(subkey) = uninstall_key.open_subkey(&subkey_name) {
                    if let Some(app_info) = parse_windows_registry_entry(&subkey, &subkey_name) {
                        applications.push(app_info);
                    }
                }
            }
        }
    }

    Ok(applications)
}

#[cfg(target_os = "windows")]
fn parse_windows_registry_entry(subkey: &RegKey, key_name: &str) -> Option<ApplicationInfo> {
    use winreg::RegValue;

    // Get display name
    let display_name: Result<String, _> = subkey.get_value("DisplayName");
    let name = match display_name {
        Ok(name) => name,
        Err(_) => return None, // Skip entries without display names
    };

    // Get version
    let version: Option<String> = subkey.get_value("DisplayVersion").ok();

    // Get install location
    let install_location: Option<String> = subkey.get_value("InstallLocation").ok();

    // Get estimated size (in KB, convert to bytes)
    let size_bytes = subkey
        .get_value::<u32, _>("EstimatedSize")
        .ok()
        .map(|size_kb| (size_kb as u64) * 1024);

    // For now, we'll use the install location or a placeholder
    let path = install_location.unwrap_or_else(|| format!("Registry: {}", key_name));

    Some(ApplicationInfo::new(
        &name, version, &path, size_bytes, None, // Category not easily available from registry
    ))
}

fn evaluate_application_conditions(app: &ApplicationInfo, conditions: &[Condition]) -> bool {
    for condition in conditions {
        let result = evaluate_single_application_condition(app, condition);
        let final_result = if condition.negated { !result } else { result };

        if !final_result {
            return false;
        }
    }
    true
}

fn evaluate_single_application_condition(app: &ApplicationInfo, condition: &Condition) -> bool {
    use crate::utils::like_match;

    match condition.field.as_str() {
        "name" => {
            if condition.operator == "LIKE" {
                like_match(&app.name, &condition.value)
            } else {
                crate::utils::compare_strings(&app.name, &condition.operator, &condition.value)
            }
        }
        "version" => {
            if let Some(version) = &app.version {
                if condition.operator == "LIKE" {
                    like_match(version, &condition.value)
                } else {
                    crate::utils::compare_strings(version, &condition.operator, &condition.value)
                }
            } else {
                false
            }
        }
        "path" => {
            if condition.operator == "LIKE" {
                like_match(&app.path, &condition.value)
            } else {
                crate::utils::compare_strings(&app.path, &condition.operator, &condition.value)
            }
        }
        "category" => {
            if let Some(category) = &app.category {
                if condition.operator == "LIKE" {
                    like_match(category, &condition.value)
                } else {
                    crate::utils::compare_strings(category, &condition.operator, &condition.value)
                }
            } else {
                false
            }
        }
        "size" => {
            if let Some(size_str) = &app.size {
                // Parse size for comparison (this is a simplified implementation)
                // In a full implementation, we'd need proper size parsing logic
                if condition.operator == "LIKE" {
                    like_match(size_str, &condition.value)
                } else {
                    crate::utils::compare_strings(size_str, &condition.operator, &condition.value)
                }
            } else {
                false
            }
        }
        _ => false,
    }
}

fn sort_application_results(
    apps: &mut Vec<ApplicationInfo>,
    order_by: &str,
    direction: &crate::models::SortDirection,
) -> Result<(), String> {
    use crate::models::SortDirection;

    apps.sort_by(|a, b| {
        let cmp = match order_by {
            "name" => a.name.cmp(&b.name),
            "version" => a
                .version
                .as_ref()
                .unwrap_or(&"".to_string())
                .cmp(b.version.as_ref().unwrap_or(&"".to_string())),
            "path" => a.path.cmp(&b.path),
            "category" => a
                .category
                .as_ref()
                .unwrap_or(&"".to_string())
                .cmp(b.category.as_ref().unwrap_or(&"".to_string())),
            "size" => {
                // Simple string comparison for size - in a full implementation,
                // we'd parse sizes for proper numeric comparison
                a.size
                    .as_ref()
                    .unwrap_or(&"".to_string())
                    .cmp(b.size.as_ref().unwrap_or(&"".to_string()))
            }
            _ => return std::cmp::Ordering::Equal,
        };

        match direction {
            SortDirection::Ascending => cmp,
            SortDirection::Descending => cmp.reverse(),
        }
    });

    Ok(())
}

fn get_directory_size_fast(path: &Path) -> u64 {
    use std::fs;

    // For performance, just get the apparent size of the directory itself
    // rather than recursively walking through all contents
    if let Ok(metadata) = fs::metadata(path) {
        metadata.len()
    } else {
        0
    }
}

fn get_directory_size(path: &Path) -> u64 {
    use std::fs;

    let mut total_size = 0u64;

    if let Ok(metadata) = fs::metadata(path) {
        total_size += metadata.len();
    }

    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                total_size += get_directory_size(&entry.path());
            }
        }
    }

    total_size
}

fn get_file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Condition;

    #[test]
    fn test_application_info_new() {
        let app = ApplicationInfo::new(
            "Test App",
            Some("1.0.0".to_string()),
            "/path/to/app",
            Some(1024),
            Some("Utility".to_string()),
        );

        assert_eq!(app.name, "Test App");
        assert_eq!(app.version, Some("1.0.0".to_string()));
        assert_eq!(app.path, "/path/to/app");
        assert_eq!(app.size, Some("1 KB".to_string()));
        assert_eq!(app.category, Some("Utility".to_string()));
    }

    #[test]
    fn test_evaluate_application_conditions() {
        let app = ApplicationInfo::new(
            "Chrome",
            Some("100.0".to_string()),
            "/Applications/Google Chrome.app",
            Some(1024 * 1024),
            Some("Browser".to_string()),
        );

        let conditions = vec![Condition {
            field: "name".to_string(),
            operator: "LIKE".to_string(),
            value: "%Chrome%".to_string(),
            negated: false,
        }];

        assert!(evaluate_application_conditions(&app, &conditions));

        let bad_conditions = vec![Condition {
            field: "name".to_string(),
            operator: "=".to_string(),
            value: "Firefox".to_string(),
            negated: false,
        }];

        assert!(!evaluate_application_conditions(&app, &bad_conditions));
    }
}
