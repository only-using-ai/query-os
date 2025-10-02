fn like_match(text: &str, pattern: &str) -> bool {
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
                match ch {
                    '.' | '*' | '+' | '?' | '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\' => {
                        regex_pattern.push('\\');
                        regex_pattern.push(ch);
                    }
                    _ => regex_pattern.push(ch),
                }
            }
        }
    }

    println!("Pattern: '{}', Text: '{}', Regex: '^{}$'", pattern, text, regex_pattern);
    
    if let Ok(regex) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
        let result = regex.is_match(text);
        println!("  Match result: {}", result);
        result
    } else {
        println!("  Regex compilation failed");
        false
    }
}

fn main() {
    println!("Testing like_match function:");
    like_match("file.md", "%.md%");
    like_match("file.mdx", "%.md%");
    like_match("readme.md", "%.md%");
    like_match("file.txt", "%.md%");
}
