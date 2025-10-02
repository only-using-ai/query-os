use chrono::{DateTime, Utc};
use clap::Parser;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub file_type: String,
    pub modified_date: DateTime<Utc>,
    pub permissions: String,
    pub size: String,
    pub path: String,
    pub depth: usize,
    pub extension: Option<String>,
}

impl FileInfo {
    // Extract extension from filename (everything after the last dot)
    // Returns None if no extension or if it's a directory
    pub fn extract_extension(filename: &str, is_directory: bool) -> Option<String> {
        if is_directory {
            return None;
        }

        // Find the last dot in the filename
        if let Some(dot_pos) = filename.rfind('.') {
            // Make sure the dot is not at the beginning of the filename
            // and there's at least one character after the dot
            if dot_pos > 0 && dot_pos < filename.len() - 1 {
                let extension = &filename[dot_pos + 1..];
                // Convert to lowercase for consistent sorting/filtering
                Some(extension.to_lowercase())
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn new(path: &Path, root_path: &Path) -> Option<Self> {
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return None, // Treat permission errors like file doesn't exist
        };

        let name = path.file_name()?.to_string_lossy().to_string();
        let file_type = if metadata.is_dir() {
            "directory"
        } else {
            "file"
        };

        let modified_date = match metadata.modified() {
            Ok(t) => DateTime::<Utc>::from(t),
            Err(_) => DateTime::<Utc>::from(std::time::SystemTime::UNIX_EPOCH),
        };

        let permissions = format!("{:o}", metadata.permissions().mode());

        let size_bytes = metadata.len();
        let size = Self::format_size(size_bytes);

        let relative_path = path.strip_prefix(root_path).unwrap_or(path);
        let path_str = relative_path.to_string_lossy().to_string();

        // Calculate depth: count path components from root
        let depth = if relative_path == Path::new("") {
            0 // Root path itself
        } else {
            relative_path.components().count()
        };

        let extension = Self::extract_extension(&name, file_type == "directory");

        Some(FileInfo {
            name,
            file_type: file_type.to_string(),
            modified_date,
            permissions,
            size,
            path: path_str,
            depth,
            extension,
        })
    }

    // Lightweight version that only gets name and path for filtering
    pub fn new_lightweight(path: &Path, root_path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().to_string();
        let relative_path = path.strip_prefix(root_path).unwrap_or(path);
        let path_str = relative_path.to_string_lossy().to_string();

        // Get minimal metadata just for file type
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return None,
        };

        let file_type = if metadata.is_dir() {
            "directory"
        } else {
            "file"
        };

        // Calculate depth: count path components from root
        let depth = if relative_path == Path::new("") {
            0 // Root path itself
        } else {
            relative_path.components().count()
        };

        let extension = Self::extract_extension(&name, file_type == "directory");

        // For lightweight version, use defaults for other fields
        Some(FileInfo {
            name,
            file_type: file_type.to_string(),
            modified_date: DateTime::<Utc>::from(std::time::SystemTime::UNIX_EPOCH),
            permissions: "0".to_string(),
            size: "0 B".to_string(),
            path: path_str,
            depth,
            extension,
        })
    }

    // Upgrade lightweight FileInfo to full version with all metadata
    pub fn upgrade_to_full(&mut self, path: &Path) {
        if let Ok(metadata) = std::fs::metadata(path) {
            self.modified_date = match metadata.modified() {
                Ok(t) => DateTime::<Utc>::from(t),
                Err(_) => DateTime::<Utc>::from(std::time::SystemTime::UNIX_EPOCH),
            };
            self.permissions = format!("{:o}", metadata.permissions().mode());
            self.size = Self::format_size(metadata.len());
        }
    }

    pub fn format_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

        if bytes == 0 {
            return "0 B".to_string();
        }

        let base = 1024_f64;
        let log = (bytes as f64).log(base);
        let unit_index = log.floor() as usize;

        if unit_index >= UNITS.len() {
            return format!(
                "{:.2} {}",
                bytes as f64 / base.powi(UNITS.len() as i32 - 1),
                UNITS[UNITS.len() - 1]
            );
        }

        let size = bytes as f64 / base.powi(unit_index as i32);
        if size.fract() == 0.0 {
            format!("{:.0} {}", size, UNITS[unit_index])
        } else {
            format!("{:.2} {}", size, UNITS[unit_index])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(FileInfo::format_size(0), "0 B");
        assert_eq!(FileInfo::format_size(512), "512 B");
        assert_eq!(FileInfo::format_size(1024), "1 KB");
        assert_eq!(FileInfo::format_size(1024 * 1024), "1 MB");
    }

    #[test]
    fn test_new_lightweight() {
        // Create a temporary file for testing
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let root_path = path.parent().unwrap();

        let file_info = FileInfo::new_lightweight(path, root_path).unwrap();

        assert_eq!(file_info.name, path.file_name().unwrap().to_string_lossy());
        assert_eq!(file_info.file_type, "file");
        // Lightweight version should have default values for other fields
        assert_eq!(file_info.size, "0 B");
        assert_eq!(file_info.permissions, "0");
        assert_eq!(file_info.depth, 1); // File is 1 level deep from parent directory
    }

    #[test]
    fn test_upgrade_to_full() {
        // Create a temporary file for testing with content
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut temp_file, b"test content for upgrade").unwrap();
        let path = temp_file.path();
        let root_path = path.parent().unwrap();

        let mut file_info = FileInfo::new_lightweight(path, root_path).unwrap();

        // Before upgrade, should have default values
        assert_eq!(file_info.size, "0 B");
        assert_eq!(file_info.permissions, "0");

        file_info.upgrade_to_full(path);

        // After upgrade, should have real values
        assert_ne!(file_info.size, "0 B");
        assert_ne!(file_info.permissions, "0");
    }

    #[test]
    fn test_depth_calculation() {
        // Test with actual existing paths
        let temp_dir = tempfile::TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a test file in the temp directory
        let test_file_path = temp_path.join("test.txt");
        std::fs::write(&test_file_path, "test content").unwrap();

        // Test root level file
        let root_file = FileInfo::new_lightweight(&test_file_path, temp_path).unwrap();
        assert_eq!(root_file.depth, 1); // temp/test.txt relative to temp is 1 level deep

        // Test root directory itself (edge case)
        let root_dir = FileInfo::new_lightweight(temp_path, temp_path).unwrap();
        assert_eq!(root_dir.depth, 0); // Root directory itself has depth 0
    }

    #[test]
    fn test_depth_with_select_star() {
        // Create a temporary directory structure for testing
        let temp_dir = tempfile::TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create nested structure: temp/dir1/dir2/file.txt
        let dir1_path = temp_path.join("dir1");
        std::fs::create_dir(&dir1_path).unwrap();
        let dir2_path = dir1_path.join("dir2");
        std::fs::create_dir(&dir2_path).unwrap();
        let file_path = dir2_path.join("file.txt");
        std::fs::write(&file_path, "test content").unwrap();

        // Test depth calculation for nested file
        let file_info = FileInfo::new_lightweight(&file_path, temp_path).unwrap();
        assert_eq!(file_info.depth, 3); // temp/dir1/dir2/file.txt is 3 levels deep

        // Test depth calculation for directory
        let dir1_info = FileInfo::new_lightweight(&dir1_path, temp_path).unwrap();
        assert_eq!(dir1_info.depth, 1); // temp/dir1 is 1 level deep

        // Test depth calculation for root temp directory
        let temp_info = FileInfo::new_lightweight(temp_path, temp_path).unwrap();
        assert_eq!(temp_info.depth, 0); // Root directory has depth 0
    }

    #[test]
    fn test_extract_extension() {
        // Test regular file with extension
        assert_eq!(
            FileInfo::extract_extension("test.txt", false),
            Some("txt".to_string())
        );
        assert_eq!(
            FileInfo::extract_extension("document.pdf", false),
            Some("pdf".to_string())
        );
        assert_eq!(
            FileInfo::extract_extension("script.js", false),
            Some("js".to_string())
        );

        // Test file with multiple dots (should get last part)
        assert_eq!(
            FileInfo::extract_extension("archive.tar.gz", false),
            Some("gz".to_string())
        );
        assert_eq!(
            FileInfo::extract_extension("file.name.with.dots.txt", false),
            Some("txt".to_string())
        );

        // Test files without extension
        assert_eq!(FileInfo::extract_extension("README", false), None);
        assert_eq!(FileInfo::extract_extension("Makefile", false), None);
        assert_eq!(FileInfo::extract_extension("Dockerfile", false), None);

        // Test edge cases
        assert_eq!(FileInfo::extract_extension(".hidden", false), None); // starts with dot
        assert_eq!(FileInfo::extract_extension("file.", false), None); // ends with dot
        assert_eq!(FileInfo::extract_extension(".", false), None); // just a dot
        assert_eq!(FileInfo::extract_extension("", false), None); // empty string

        // Test directories (should always return None)
        assert_eq!(FileInfo::extract_extension("somedir", true), None);
        assert_eq!(FileInfo::extract_extension("dir.with.dots", true), None);

        // Test case conversion to lowercase
        assert_eq!(
            FileInfo::extract_extension("FILE.TXT", false),
            Some("txt".to_string())
        );
        assert_eq!(
            FileInfo::extract_extension("file.PDF", false),
            Some("pdf".to_string())
        );
    }
}

#[derive(Debug, Clone)]
pub enum QueryResult {
    Files(Vec<FileInfo>),
    Processes(Vec<ProcessInfo>),
    Network(Vec<NetInfo>),
    Applications(Vec<ApplicationInfo>),
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: String,
    pub name: String,
    pub cpu_usage: String,
    pub memory_usage: String,
    pub status: String,
}

impl ProcessInfo {
    pub fn new(pid: u32, name: &str, cpu_usage: f32, memory_bytes: u64, status: &str) -> Self {
        ProcessInfo {
            pid: pid.to_string(),
            name: name.to_string(),
            cpu_usage: format!("{:.1}%", cpu_usage),
            memory_usage: Self::format_memory(memory_bytes),
            status: status.to_string(),
        }
    }

    fn format_memory(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

        if bytes == 0 {
            return "0 B".to_string();
        }

        let base = 1024_f64;
        let log = (bytes as f64).log(base);
        let unit_index = log.floor() as usize;

        if unit_index >= UNITS.len() {
            return format!(
                "{:.2} {}",
                bytes as f64 / base.powi(UNITS.len() as i32 - 1),
                UNITS[UNITS.len() - 1]
            );
        }

        let size = bytes as f64 / base.powi(unit_index as i32);
        if size.fract() == 0.0 {
            format!("{:.0} {}", size, UNITS[unit_index])
        } else {
            format!("{:.2} {}", size, UNITS[unit_index])
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetInfo {
    pub name: String,
    pub port: String,
    pub pid: String,
}

impl NetInfo {
    pub fn new(name: &str, port: u16, pid: u32) -> Self {
        NetInfo {
            name: name.to_string(),
            port: port.to_string(),
            pid: pid.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApplicationInfo {
    pub name: String,
    pub version: Option<String>,
    pub path: String,
    pub size: Option<String>,
    pub category: Option<String>,
}

impl ApplicationInfo {
    pub fn new(
        name: &str,
        version: Option<String>,
        path: &str,
        size: Option<u64>,
        category: Option<String>,
    ) -> Self {
        ApplicationInfo {
            name: name.to_string(),
            version,
            path: path.to_string(),
            size: size.map(|s| Self::format_size(s)),
            category,
        }
    }

    fn format_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

        if bytes == 0 {
            return "0 B".to_string();
        }

        let base = 1024_f64;
        let log = (bytes as f64).log(base);
        let unit_index = log.floor() as usize;

        if unit_index >= UNITS.len() {
            return format!(
                "{:.2} {}",
                bytes as f64 / base.powi(UNITS.len() as i32 - 1),
                UNITS[UNITS.len() - 1]
            );
        }

        let size = bytes as f64 / base.powi(unit_index as i32);
        if size.fract() == 0.0 {
            format!("{:.0} {}", size, UNITS[unit_index])
        } else {
            format!("{:.2} {}", size, UNITS[unit_index])
        }
    }
}

#[cfg(test)]
mod process_tests {
    use super::*;

    #[test]
    fn test_process_info_new() {
        let process = ProcessInfo::new(1234, "node", 5.5, 1024 * 1024, "running");
        assert_eq!(process.pid, "1234");
        assert_eq!(process.name, "node");
        assert_eq!(process.cpu_usage, "5.5%");
        assert_eq!(process.memory_usage, "1 MB");
        assert_eq!(process.status, "running");
    }

    #[test]
    fn test_process_info_format_memory() {
        assert_eq!(ProcessInfo::format_memory(0), "0 B");
        assert_eq!(ProcessInfo::format_memory(512), "512 B");
        assert_eq!(ProcessInfo::format_memory(1024), "1 KB");
        assert_eq!(ProcessInfo::format_memory(1024 * 1024), "1 MB");
        assert_eq!(ProcessInfo::format_memory(1024 * 1024 * 1024), "1 GB");
    }
}

#[derive(Parser)]
#[command(name = "q")]
#[command(about = "Query filesystem with SQL-like syntax")]
pub struct Args {
    /// SQL query string
    #[arg(long, value_name = "QUERY")]
    pub query: Option<String>,

    /// Save the query as a template for later use
    #[arg(long, value_name = "NAME")]
    pub save: Option<String>,

    /// Execute a saved query template
    #[arg(long, value_name = "NAME")]
    pub template: Option<String>,

    /// Arguments to inject into template (used with --template)
    #[arg(trailing_var_arg = true, hide = true)]
    pub template_args: Vec<String>,

    /// Launch GUI interface
    #[arg(long)]
    pub gui: bool,
}

#[derive(Debug, PartialEq)]
pub enum QueryType {
    Select,
    Delete,
}

#[derive(Debug, PartialEq, Clone)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, PartialEq)]
pub enum SubqueryType {
    Scalar, // Returns single value for SELECT subqueries
    Exists, // For EXISTS/NOT EXISTS conditions
    In,     // For IN/NOT IN conditions
}

#[derive(Debug)]
pub struct Subquery {
    pub query: Box<SqlQuery>,
    pub subquery_type: SubqueryType,
}

#[derive(Debug)]
pub struct SqlQuery {
    pub query_type: QueryType,
    pub distinct: bool,
    pub select_fields: Vec<String>,
    pub select_field_aliases: Vec<Option<String>>, // Aliases for SELECT fields
    pub select_subqueries: Vec<Subquery>,          // Scalar subqueries in SELECT
    pub from_path: String,
    pub where_clause: Option<String>,
    pub where_subqueries: Vec<Subquery>, // Subqueries in WHERE conditions
    pub order_by: Option<String>,
    pub order_direction: SortDirection,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Condition {
    pub field: String,
    pub operator: String,
    pub value: String,
    pub negated: bool,
}
