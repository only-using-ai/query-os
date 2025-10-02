use crate::models::{QueryResult, SqlQuery};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::time::Duration;
use url::Url;

/// Check if a string is a valid HTTP/HTTPS URL
pub fn is_url(s: &str) -> bool {
    Url::parse(s).is_ok()
        && matches!(Url::parse(s), Ok(url) if url.scheme() == "http" || url.scheme() == "https")
}

/// Validate URL for security (block localhost, private IPs, etc.)
pub fn validate_url(url_str: &str) -> Result<(), String> {
    let url = Url::parse(url_str).map_err(|e| format!("Invalid URL: {}", e))?;

    // Only allow HTTP and HTTPS
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err("Only HTTP and HTTPS URLs are allowed".to_string());
    }

    // Block localhost and private IP ranges
    if let Some(host) = url.host_str() {
        if host == "localhost" || host.starts_with("127.") || host == "0.0.0.0" {
            return Err("Localhost URLs are not allowed for security reasons".to_string());
        }

        // Block private IP ranges (simplified check)
        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            match ip {
                std::net::IpAddr::V4(ipv4) => {
                    let octets = ipv4.octets();
                    if octets[0] == 10
                        || (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31)
                        || (octets[0] == 192 && octets[1] == 168)
                        || (octets[0] == 169 && octets[1] == 254)
                    {
                        return Err("Private IP addresses are not allowed".to_string());
                    }
                }
                std::net::IpAddr::V6(_) => {
                    // For simplicity, block all IPv6 for now
                    return Err("IPv6 addresses are not supported".to_string());
                }
            }
        }
    }

    Ok(())
}

/// Execute a web scraping query
pub fn execute_web_query(query: &SqlQuery) -> Result<QueryResult, String> {
    // Validate URL
    validate_url(&query.from_path)?;

    // Create progress bar for user feedback
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message("Fetching webpage...");
    pb.enable_steady_tick(Duration::from_millis(100));

    // Fetch the webpage
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("query-os/1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&query.from_path)
        .send()
        .map_err(|e| format!("Failed to fetch URL: {}", e))?;

    if !response.status().is_success() {
        pb.finish_and_clear();
        return Err(format!(
            "HTTP error {}: {}",
            response.status().as_u16(),
            response.status().canonical_reason().unwrap_or("Unknown")
        ));
    }

    let html_content = response
        .text()
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    // Limit response size to prevent memory exhaustion
    if html_content.len() > 10 * 1024 * 1024 {
        pb.finish_and_clear();
        return Err("Response too large (>10MB). Use a more specific selector.".to_string());
    }

    pb.set_message("Parsing content...");

    // Parse HTML
    let document = Html::parse_document(&html_content);

    // Process selectors
    let mut results = Vec::new();

    for selector_str in &query.select_fields {
        if selector_str == "*" {
            // Return raw HTML
            pb.finish_and_clear();
            return Ok(QueryResult::Files(vec![crate::models::FileInfo {
                name: query.from_path.clone(),
                file_type: "webpage".to_string(),
                modified_date: chrono::Utc::now(),
                permissions: "644".to_string(),
                size: format!("{} bytes", html_content.len()),
                path: query.from_path.clone(),
                depth: 0,
                extension: None,
            }]));
        }

        // Parse CSS selector
        let (css_selector, extract_text) = if selector_str.ends_with("::text") {
            (&selector_str[..selector_str.len() - 6], true)
        } else {
            (selector_str.as_str(), false)
        };

        let selector = Selector::parse(css_selector)
            .map_err(|e| format!("Invalid CSS selector '{}': {}", css_selector, e))?;

        // Extract elements
        for element in document.select(&selector) {
            let content = if extract_text {
                element
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string()
            } else {
                element.html().trim().to_string()
            };

            if !content.is_empty() {
                results.push(content);
            }
        }
    }

    pb.finish_and_clear();

    // For now, return as files for compatibility with existing display logic
    // This could be enhanced to return structured data
    let file_results = results
        .into_iter()
        .enumerate()
        .map(|(i, content)| crate::models::FileInfo {
            name: format!("result_{}", i + 1),
            file_type: "web_content".to_string(),
            modified_date: chrono::Utc::now(),
            permissions: "644".to_string(),
            size: format!("{} chars", content.len()),
            path: content,
            depth: 0,
            extension: None,
        })
        .collect();

    Ok(QueryResult::Files(file_results))
}
