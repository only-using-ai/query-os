use clap::Parser;
use query_os::models::QueryType;
use query_os::{
    display_application_results, display_network_results, display_process_results, display_results,
    execute_query, gui, load_template_with_args, parse_query, save_template, web, Args,
    QueryResult,
};
use std::time::Instant;

fn main() {
    let args = Args::parse();

    // Handle GUI mode
    if args.gui {
        if let Err(e) = gui::run_gui() {
            eprintln!("Error running GUI: {}", e);
            std::process::exit(1);
        }
        return;
    }

    // Handle template mode
    if let Some(template_name) = &args.template {
        match load_template_with_args(template_name, &args.template_args) {
            Ok(query) => {
                let start_time = Instant::now();
                match execute_query(&query) {
                    Ok(results) => {
                        // For DELETE queries, results are already printed by the execution functions
                        if query.query_type == QueryType::Select {
                            display_query_results(&results, &query.select_fields, &query.from_path);
                        }
                        let duration = start_time.elapsed();
                        println!(
                            "\x1b[32mQuery executed in {:.3}ms\x1b[0m",
                            duration.as_millis()
                        );
                    }
                    Err(e) => eprintln!("Error executing query: {}", e),
                }
            }
            Err(e) => eprintln!("Error loading template '{}': {}", template_name, e),
        }
        return;
    }

    // Handle save mode or regular query execution
    if let Some(query_str) = &args.query {
        let start_time = Instant::now();

        match parse_query(query_str) {
            Ok(query) => {
                // If save flag is present, save the query before executing
                if let Some(template_name) = &args.save {
                    if let Err(e) = save_template(template_name, query_str) {
                        eprintln!("Error saving template: {}", e);
                        return;
                    }
                    println!("Template '{}' saved successfully.", template_name);
                }

                match execute_query(&query) {
                    Ok(results) => {
                        // For DELETE queries, results are already printed by the execution functions
                        if query.query_type == QueryType::Select {
                            display_query_results(&results, &query.select_fields, &query.from_path);
                        }
                        let duration = start_time.elapsed();
                        println!(
                            "\x1b[32mQuery executed in {:.3}ms\x1b[0m",
                            duration.as_millis()
                        );
                    }
                    Err(e) => eprintln!("Error executing query: {}", e),
                }
            }
            Err(e) => eprintln!("Error parsing query: {}", e),
        }
    } else if args.save.is_none() && args.template.is_none() {
        eprintln!("Error: No query provided. Use --help for usage information.");
        std::process::exit(1);
    }
}

fn display_query_results(results: &QueryResult, select_fields: &[String], from_path: &str) {
    match results {
        QueryResult::Files(files) => {
            // Check if this is web content that should be displayed as raw HTML
            if select_fields.len() == 1 && select_fields[0] == "*" && web::is_url(from_path) {
                // Display raw HTML content
                for file in files {
                    println!("{}", file.path);
                }
            } else if files.iter().any(|f| f.file_type == "web_content") {
                // Display web content results (CSS selector results) in table format
                display_results(files, select_fields);
            } else {
                // Regular file results
                display_results(files, select_fields);
            }
        }
        QueryResult::Processes(processes) => display_process_results(processes, select_fields),
        QueryResult::Network(network_info) => display_network_results(network_info, select_fields),
        QueryResult::Applications(apps) => display_application_results(apps, select_fields),
    }
}
