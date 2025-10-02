use crate::models::SqlQuery;
use crate::parser::parse_query;
use regex::Regex;
use std::fs;
use std::path::PathBuf;

pub fn save_template(name: &str, query: &str) -> Result<(), String> {
    let template_dir = get_template_dir()?;
    fs::create_dir_all(&template_dir)
        .map_err(|e| format!("Failed to create template directory: {}", e))?;

    let mut template_path = template_dir.join(format!("{}.sql", name));

    // Check if template already exists
    if template_path.exists() {
        println!(
            "Template '{}' already exists. Overwrite? (y/N) or enter new name: ",
            name
        );
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| format!("Failed to read input: {}", e))?;
        let input = input.trim().to_lowercase();

        if input == "y" || input == "yes" {
            // Overwrite existing template
        } else if input.is_empty() || input == "n" || input == "no" {
            return Err("Template save cancelled.".to_string());
        } else {
            // Use new name
            let new_path = template_dir.join(format!("{}.sql", input));
            if new_path.exists() {
                return Err(format!(
                    "Template '{}' also exists. Operation cancelled.",
                    input
                ));
            }
            template_path = new_path;
        }
    }

    fs::write(&template_path, query).map_err(|e| format!("Failed to save template: {}", e))?;
    Ok(())
}

pub fn load_template(name: &str) -> Result<SqlQuery, String> {
    load_template_with_args(name, &[])
}

pub fn load_template_content(name: &str) -> Result<String, String> {
    let template_dir = get_template_dir()?;
    let template_path = template_dir.join(format!("{}.sql", name));

    if !template_path.exists() {
        return Err(format!("Template '{}' not found.", name));
    }

    fs::read_to_string(&template_path).map_err(|e| format!("Failed to read template: {}", e))
}

pub fn load_template_with_args(name: &str, args: &[String]) -> Result<SqlQuery, String> {
    let template_dir = get_template_dir()?;
    let template_path = template_dir.join(format!("{}.sql", name));

    if !template_path.exists() {
        return Err(format!("Template '{}' not found.", name));
    }

    let mut query = fs::read_to_string(&template_path)
        .map_err(|e| format!("Failed to read template: {}", e))?;

    if !args.is_empty() {
        query = substitute_variables(&query, args)?;
    }

    parse_query(&query)
}

fn substitute_variables(query: &str, args: &[String]) -> Result<String, String> {
    let mut result = query.to_string();

    // Find all $N patterns in the query
    let placeholder_regex = Regex::new(r"\$([1-9][0-9]*)").unwrap();
    let mut used_indices = Vec::new();

    for cap in placeholder_regex.captures_iter(query) {
        let full_match = &cap[0];
        let index_str = &cap[1];
        let index: usize = index_str
            .parse()
            .map_err(|_| format!("Invalid placeholder: {}", full_match))?;

        if index == 0 {
            return Err("Placeholders must start from $1, not $0".to_string());
        }

        if index > args.len() {
            return Err(format!("Not enough arguments provided. Template requires at least {} arguments, but only {} were given.", index, args.len()));
        }

        if !used_indices.contains(&index) {
            used_indices.push(index);
        }
        let replacement = &args[index - 1]; // Convert to 0-based indexing
        result = result.replace(full_match, replacement);
    }

    // Check if all arguments were used
    if used_indices.len() < args.len() {
        return Err(format!("Too many arguments provided. Template only uses {} placeholders, but {} arguments were given.", used_indices.len(), args.len()));
    }

    Ok(result)
}

pub fn get_template_dir() -> Result<PathBuf, String> {
    let home_dir = dirs::home_dir().ok_or("Could not determine home directory")?;
    Ok(home_dir.join(".q").join("templates"))
}
