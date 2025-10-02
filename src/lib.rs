pub mod applications;
pub mod filesystem;
pub mod gui;
pub mod models;
pub mod network;
pub mod parser;
pub mod processes;
pub mod templates;
pub mod utils;
pub mod web;

// Re-export commonly used types and functions for convenience
pub use applications::execute_application_query;
pub use filesystem::execute_query;
pub use models::{
    ApplicationInfo, Args, Condition, FileInfo, NetInfo, ProcessInfo, QueryResult, SqlQuery, Subquery, SubqueryType,
};
pub use parser::{parse_compound_conditions, parse_query};
pub use templates::{
    get_template_dir, load_template, load_template_content, load_template_with_args, save_template,
};
pub use utils::{
    display_application_results, display_network_results, display_process_results, display_results, evaluate_conditions,
    evaluate_single_condition, expand_path, sort_process_results,
};
