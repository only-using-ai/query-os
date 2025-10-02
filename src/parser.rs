use crate::models::{Condition, SqlQuery, Subquery};
use crate::utils::expand_path;
use crate::web::is_url;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "q.pest"]
struct FqParser;

pub fn parse_query(query: &str) -> Result<SqlQuery, String> {
    // Preprocess query to uppercase SQL keywords for case-insensitive parsing
    let processed_query = uppercase_keywords(query);

    let pairs = FqParser::parse(Rule::query, &processed_query)
        .map_err(|e| format!("Parse error: {}", e))?;

    let query_pair = pairs.into_iter().next().unwrap();

    // The query rule should contain either select_query or delete_query as inner pairs
    let inner_pairs: Vec<_> = query_pair.into_inner().collect();

    if let Some(inner_pair) = inner_pairs.into_iter().next() {
        match inner_pair.as_rule() {
            Rule::select_query => parse_select_query(inner_pair, query),
            Rule::delete_query => parse_delete_query(inner_pair),
            _ => Err("Invalid query type".to_string()),
        }
    } else {
        Err("No inner pairs found".to_string())
    }
}

fn uppercase_keywords(query: &str) -> String {
    let keywords = [
        "SELECT", "FROM", "WHERE", "DELETE", "ORDER", "BY", "LIMIT", "AND", "AS", "LIKE", "NOT",
        "EXISTS", "IN", "DISTINCT", "IS", "NULL",
    ];

    let mut result = query.to_string();
    for keyword in &keywords {
        // Use word boundaries to avoid partial matches, case insensitive
        let pattern = regex::Regex::new(&format!(r"(?i)\b{}\b", regex::escape(keyword))).unwrap();
        result = pattern.replace_all(&result, *keyword).to_string();
    }
    result
}

fn parse_select_query(
    pair: pest::iterators::Pair<Rule>,
    original_query: &str,
) -> Result<SqlQuery, String> {
    use crate::models::QueryType;

    let mut distinct = original_query.to_uppercase().contains("DISTINCT");
    let mut from_path = String::new();
    let mut select_fields = Vec::new();
    let mut select_field_aliases = Vec::new();
    let mut select_subqueries = Vec::new();
    let mut where_clause = None;
    let mut where_subqueries = Vec::new();
    let mut order_by = None;
    let mut order_direction = crate::models::SortDirection::Ascending; // Default to ascending
    let mut limit = None;

    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::DISTINCT => {
                distinct = true;
            }
            Rule::fields => {
                let (fields, aliases, subqueries) = parse_fields(inner_pair)?;
                select_fields = fields;
                select_field_aliases = aliases;
                select_subqueries = subqueries;
            }
            Rule::path => {
                from_path = parse_path(inner_pair)?;
            }
            Rule::condition => {
                let (clause, subqueries) = parse_condition(inner_pair)?;
                where_clause = Some(clause);
                where_subqueries = subqueries;
            }
            Rule::order_by_clause => {
                let (field, direction) = parse_order_by_clause(inner_pair)?;
                order_by = Some(field);
                order_direction = direction;
            }
            Rule::number => {
                limit = Some(
                    inner_pair
                        .as_str()
                        .parse()
                        .map_err(|_| "Invalid limit value")?,
                );
            }
            _ => {}
        }
    }

    // Handle * expansion like the original parser
    if select_fields == vec!["*"] {
        if from_path == "ps" {
            select_fields = vec![
                "pid".to_string(),
                "name".to_string(),
                "cpu_usage".to_string(),
                "memory_usage".to_string(),
                "status".to_string(),
            ];
        } else if from_path == "net" {
            select_fields = vec!["name".to_string(), "port".to_string(), "pid".to_string()];
        } else if from_path == "applications" {
            select_fields = vec![
                "name".to_string(),
                "version".to_string(),
                "path".to_string(),
                "size".to_string(),
                "category".to_string(),
            ];
        } else {
            select_fields = vec![
                "name".to_string(),
                "type".to_string(),
                "modified_date".to_string(),
                "permissions".to_string(),
                "size".to_string(),
                "path".to_string(),
            ];
        }
        select_field_aliases = vec![None; select_fields.len()];
    }

    Ok(SqlQuery {
        query_type: QueryType::Select,
        distinct,
        select_fields,
        select_field_aliases,
        select_subqueries,
        from_path,
        where_clause,
        where_subqueries,
        order_by,
        order_direction,
        limit,
    })
}

fn parse_delete_query(pair: pest::iterators::Pair<Rule>) -> Result<SqlQuery, String> {
    use crate::models::QueryType;

    let mut from_path = String::new();
    let mut where_clause = None;
    let mut where_subqueries = Vec::new();

    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::path => {
                from_path = parse_path(inner_pair)?;
            }
            Rule::condition => {
                let (clause, subqueries) = parse_condition(inner_pair)?;
                where_clause = Some(clause);
                where_subqueries = subqueries;
            }
            _ => {}
        }
    }

    Ok(SqlQuery {
        query_type: QueryType::Delete,
        distinct: false,
        select_fields: Vec::new(),
        select_field_aliases: Vec::new(),
        select_subqueries: Vec::new(),
        from_path,
        where_clause,
        where_subqueries,
        order_by: None,
        order_direction: crate::models::SortDirection::Ascending,
        limit: None,
    })
}

fn parse_path(pair: pest::iterators::Pair<Rule>) -> Result<String, String> {
    let path_str = pair.as_str().trim_matches('\'');
    if is_url(path_str) {
        Ok(path_str.to_string())
    } else {
        Ok(expand_path(path_str))
    }
}

fn parse_fields(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(Vec<String>, Vec<Option<String>>, Vec<Subquery>), String> {
    let mut fields = Vec::new();
    let mut aliases = Vec::new();
    let mut subqueries = Vec::new();

    if pair.as_str().trim() == "*" {
        // This will be handled in the calling function based on the from_path
        return Ok((vec!["*".to_string()], vec![None], Vec::new()));
    }

    for field_pair in pair.into_inner() {
        if field_pair.as_rule() == Rule::field_list {
            for field in field_pair.into_inner() {
                if field.as_rule() == Rule::field {
                    let (field_name, alias, subquery) = parse_field(field)?;
                    if let Some(sq) = subquery {
                        subqueries.push(sq);
                        fields.push(
                            alias
                                .clone()
                                .unwrap_or_else(|| "subquery_result".to_string()),
                        );
                        aliases.push(alias);
                    } else {
                        fields.push(field_name.clone());
                        aliases.push(alias);
                    }
                }
            }
        }
    }

    Ok((fields, aliases, subqueries))
}

fn parse_field(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(String, Option<String>, Option<Subquery>), String> {
    let mut field_name = String::new();
    let mut alias = None;

    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::identifier => {
                if field_name.is_empty() {
                    field_name = inner_pair.as_str().to_string();
                } else if alias.is_none() {
                    alias = Some(inner_pair.as_str().to_string());
                }
            }
            _ => {}
        }
    }

    Ok((field_name, alias, None))
}

fn parse_condition(pair: pest::iterators::Pair<Rule>) -> Result<(String, Vec<Subquery>), String> {
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::comparison => {
                return parse_comparison_condition(inner_pair);
            }
            Rule::like_condition => {
                return parse_like_condition(inner_pair);
            }
            Rule::not_like_condition => {
                return parse_not_like_condition(inner_pair);
            }
            Rule::null_condition | Rule::is_null_condition | Rule::simple_null_condition => {
                return parse_null_condition(inner_pair);
            }
            Rule::not_null_condition
            | Rule::is_not_null_condition
            | Rule::simple_not_null_condition => {
                return parse_not_null_condition(inner_pair);
            }
            _ => {}
        }
    }

    Err("Invalid condition".to_string())
}

fn parse_comparison_condition(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(String, Vec<Subquery>), String> {
    let mut field = String::new();
    let mut operator = String::new();
    let mut value = String::new();

    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::identifier => field = inner_pair.as_str().to_string(),
            Rule::EQUALS => operator = "=".to_string(),
            Rule::NOT_EQUALS => operator = "!=".to_string(),
            Rule::GREATER => operator = ">".to_string(),
            Rule::GREATER_EQUALS => operator = ">=".to_string(),
            Rule::LESS => operator = "<".to_string(),
            Rule::LESS_EQUALS => operator = "<=".to_string(),
            Rule::value => value = inner_pair.as_str().to_string(),
            _ => {}
        }
    }

    Ok((format!("{} {} {}", field, operator, value), Vec::new()))
}

fn parse_like_condition(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(String, Vec<Subquery>), String> {
    let mut field = String::new();
    let mut value = String::new();

    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::identifier => field = inner_pair.as_str().to_string(),
            Rule::value => value = inner_pair.as_str().to_string(),
            _ => {}
        }
    }

    Ok((format!("{} LIKE {}", field, value), Vec::new()))
}

fn parse_not_like_condition(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(String, Vec<Subquery>), String> {
    let mut field = String::new();
    let mut value = String::new();

    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::identifier => field = inner_pair.as_str().to_string(),
            Rule::value => value = inner_pair.as_str().to_string(),
            _ => {}
        }
    }

    Ok((format!("{} NOT LIKE {}", field, value), Vec::new()))
}

fn parse_null_condition(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(String, Vec<Subquery>), String> {
    let mut field = String::new();

    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::identifier => field = inner_pair.as_str().to_string(),
            _ => {}
        }
    }

    Ok((format!("{} IS NULL", field), Vec::new()))
}

fn parse_not_null_condition(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(String, Vec<Subquery>), String> {
    let mut field = String::new();

    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::identifier => field = inner_pair.as_str().to_string(),
            _ => {}
        }
    }

    Ok((format!("{} IS NOT NULL", field), Vec::new()))
}

fn parse_order_by_clause(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(String, crate::models::SortDirection), String> {
    let clause_str = pair.as_str();

    // Check if the clause contains DESC
    if clause_str.to_uppercase().contains(" DESC") {
        // Split by DESC and take the field part
        if let Some(field_part) = clause_str.split(" DESC").next() {
            return Ok((
                field_part.trim().to_string(),
                crate::models::SortDirection::Descending,
            ));
        }
    } else if clause_str.to_uppercase().contains(" ASC") {
        if let Some(field_part) = clause_str.split(" ASC").next() {
            return Ok((
                field_part.trim().to_string(),
                crate::models::SortDirection::Ascending,
            ));
        }
    }

    // Default case - no ASC/DESC specified
    Ok((
        clause_str.trim().to_string(),
        crate::models::SortDirection::Ascending,
    ))
}

pub fn parse_compound_conditions(where_clause: &str) -> Result<Vec<Condition>, String> {
    let mut conditions = Vec::new();

    // Pre-compile regexes to avoid compiling in loop
    let not_like_re = regex::Regex::new(r"(?i)(\w+)\s+NOT\s+LIKE\s+(.+)").unwrap();
    let is_null_re = regex::Regex::new(r"(?i)(\w+)\s+IS\s+NULL").unwrap();
    let is_not_null_re = regex::Regex::new(r"(?i)(\w+)\s+IS\s+NOT\s+NULL").unwrap();
    let condition_re = regex::Regex::new(r"(?i)(\w+)\s*([=<>!]+|LIKE)\s*(.+)").unwrap();

    // Split by AND (case-insensitive) first, then handle each part
    let and_re = regex::Regex::new(r"(?i)\s+and\s+").unwrap();
    let and_parts: Vec<&str> = and_re.split(where_clause).collect();

    for part in and_parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Parse the individual condition - handle special cases first
        let (is_negated, condition_part) = if let Some(caps) = not_like_re.captures(part) {
            (true, format!("{} LIKE {}", &caps[1], &caps[2]))
        } else {
            (false, part.to_string())
        };

        // Handle IS NULL and IS NOT NULL conditions
        if let Some(caps) = is_null_re.captures(&condition_part) {
            let field = caps[1].to_lowercase();
            conditions.push(Condition {
                field,
                operator: "IS".to_string(),
                value: "NULL".to_string(),
                negated: false,
            });
        } else if let Some(caps) = is_not_null_re.captures(&condition_part) {
            let field = caps[1].to_lowercase();
            conditions.push(Condition {
                field,
                operator: "IS".to_string(),
                value: "NULL".to_string(),
                negated: true, // IS NOT NULL is negated IS NULL
            });
        } else if let Some(caps) = condition_re.captures(&condition_part) {
            let field = caps[1].to_lowercase();
            let operator = caps[2].to_uppercase();
            let value = caps[3].trim_matches('\'').trim().to_string();

            conditions.push(Condition {
                field,
                operator,
                value,
                negated: is_negated,
            });
        } else {
            return Err(format!("Invalid condition: {}", condition_part));
        }
    }

    Ok(conditions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_select_star() {
        let query = "SELECT * FROM /tmp";
        let result = parse_query(query).unwrap();
        assert_eq!(result.query_type, crate::models::QueryType::Select);
        assert_eq!(result.select_fields.len(), 6);
        assert_eq!(result.from_path, "/tmp");
        assert!(result.where_clause.is_none());
    }

    #[test]
    fn test_parse_query_with_is_operator() {
        // Test that "IS NULL" works correctly
        let query_is_null = parse_query("SELECT * FROM /tmp WHERE name IS NULL").unwrap();
        assert_eq!(query_is_null.where_clause, Some("name IS NULL".to_string()));

        // Test that "IS NOT NULL" works correctly
        let query_is_not_null = parse_query("SELECT * FROM /tmp WHERE name IS NOT NULL").unwrap();
        assert_eq!(
            query_is_not_null.where_clause,
            Some("name IS NOT NULL".to_string())
        );
    }

    #[test]
    fn test_parse_query_with_where() {
        let query = "SELECT name, type FROM /tmp WHERE type = 'file'";
        let result = parse_query(query).unwrap();
        assert_eq!(result.query_type, crate::models::QueryType::Select);
        assert_eq!(result.select_fields, vec!["name", "type"]);
        assert_eq!(result.from_path, "/tmp");
        assert_eq!(result.where_clause, Some("type = 'file'".to_string()));
    }

    #[test]
    fn test_parse_query_with_limit() {
        let query = "SELECT * FROM /tmp LIMIT 10";
        let result = parse_query(query).unwrap();
        assert_eq!(result.query_type, crate::models::QueryType::Select);
        assert_eq!(result.limit, Some(10));
    }

    #[test]
    fn test_parse_query_delete_file() {
        let query = "DELETE FROM . WHERE name = 'test.txt'";
        let result = parse_query(query).unwrap();
        assert_eq!(result.query_type, crate::models::QueryType::Delete);
        assert_eq!(result.from_path, ".");
        assert_eq!(result.where_clause, Some("name = 'test.txt'".to_string()));
    }

    #[test]
    fn test_parse_query_delete_directory() {
        let query = "DELETE FROM ./test";
        let result = parse_query(query).unwrap();
        assert_eq!(result.query_type, crate::models::QueryType::Delete);
        assert_eq!(result.from_path, "./test");
        assert!(result.where_clause.is_none());
    }

    #[test]
    fn test_parse_query_delete_process() {
        let query = "DELETE FROM ps WHERE name = 'node'";
        let result = parse_query(query).unwrap();
        assert_eq!(result.query_type, crate::models::QueryType::Delete);
        assert_eq!(result.from_path, "ps");
        assert_eq!(result.where_clause, Some("name = 'node'".to_string()));
    }

    #[test]
    fn test_parse_compound_conditions() {
        let conditions =
            parse_compound_conditions("name LIKE '%.rs' AND path NOT LIKE '%target/%'").unwrap();
        assert_eq!(conditions.len(), 2);

        assert_eq!(conditions[0].field, "name");
        assert_eq!(conditions[0].operator, "LIKE");
        assert_eq!(conditions[0].value, "%.rs");
        assert!(!conditions[0].negated);

        assert_eq!(conditions[1].field, "path");
        assert_eq!(conditions[1].operator, "LIKE");
        assert_eq!(conditions[1].value, "%target/%");
        assert!(conditions[1].negated);
    }

    #[test]
    fn test_parse_query_case_insensitive_select() {
        let query = "select * from /tmp";
        let result = parse_query(query).unwrap();
        assert_eq!(result.query_type, crate::models::QueryType::Select);
        assert_eq!(result.from_path, "/tmp");
    }

    #[test]
    fn test_parse_query_case_insensitive_delete() {
        let query = "delete from /tmp where name = 'test.txt'";
        let result = parse_query(query).unwrap();
        assert_eq!(result.query_type, crate::models::QueryType::Delete);
        assert_eq!(result.from_path, "/tmp");
        assert_eq!(result.where_clause, Some("name = 'test.txt'".to_string()));
    }

    #[test]
    fn test_parse_query_case_insensitive_where() {
        let query = "SELECT name, type FROM /tmp WHERE type = 'file'";
        let result = parse_query(query).unwrap();
        assert_eq!(result.query_type, crate::models::QueryType::Select);
        assert_eq!(result.select_fields, vec!["name", "type"]);
        assert_eq!(result.from_path, "/tmp");
        assert_eq!(result.where_clause, Some("type = 'file'".to_string()));
    }

    #[test]
    fn test_parse_query_case_insensitive_order_by() {
        let query = "select * from /tmp order by name";
        let result = parse_query(query).unwrap();
        assert_eq!(result.query_type, crate::models::QueryType::Select);
        assert_eq!(result.order_by, Some("name".to_string()));
    }

    #[test]
    fn test_parse_query_case_insensitive_limit() {
        let query = "select * from /tmp limit 10";
        let result = parse_query(query).unwrap();
        assert_eq!(result.query_type, crate::models::QueryType::Select);
        assert_eq!(result.limit, Some(10));
    }

    #[test]
    fn test_parse_query_case_insensitive_mixed() {
        let query = "Select * From /tmp Where type = 'file' Order By name Limit 5";
        let result = parse_query(query).unwrap();
        assert_eq!(result.query_type, crate::models::QueryType::Select);
        assert_eq!(result.from_path, "/tmp");
        assert_eq!(result.where_clause, Some("type = 'file'".to_string()));
        assert_eq!(result.order_by, Some("name".to_string()));
        assert_eq!(
            result.order_direction,
            crate::models::SortDirection::Ascending
        );
        assert_eq!(result.limit, Some(5));
    }

    #[test]
    fn test_parse_query_order_by_asc() {
        let query = "SELECT * FROM /tmp ORDER BY name ASC";
        let result = parse_query(query).unwrap();
        assert_eq!(result.order_by, Some("name".to_string()));
        assert_eq!(
            result.order_direction,
            crate::models::SortDirection::Ascending
        );
    }

    #[test]
    fn test_parse_query_order_by_desc() {
        let query = "SELECT * FROM /tmp ORDER BY name DESC";
        let result = parse_query(query).unwrap();
        assert_eq!(result.order_by, Some("name".to_string()));
        assert_eq!(
            result.order_direction,
            crate::models::SortDirection::Descending
        );
    }

    #[test]
    fn test_parse_query_order_by_default_asc() {
        let query = "SELECT * FROM /tmp ORDER BY name";
        let result = parse_query(query).unwrap();
        assert_eq!(result.order_by, Some("name".to_string()));
        assert_eq!(
            result.order_direction,
            crate::models::SortDirection::Ascending
        );
    }

    #[test]
    fn test_parse_compound_conditions_case_insensitive_and() {
        let conditions =
            parse_compound_conditions("name like '%.rs' and path not like '%target/%'").unwrap();
        assert_eq!(conditions.len(), 2);

        assert_eq!(conditions[0].field, "name");
        assert_eq!(conditions[0].operator, "LIKE");
        assert_eq!(conditions[0].value, "%.rs");
        assert!(!conditions[0].negated);

        assert_eq!(conditions[1].field, "path");
        assert_eq!(conditions[1].operator, "LIKE");
        assert_eq!(conditions[1].value, "%target/%");
        assert!(conditions[1].negated);
    }

    #[test]
    fn test_parse_compound_conditions_case_insensitive_like() {
        let conditions = parse_compound_conditions("name like '%.txt' and type = 'file'").unwrap();
        assert_eq!(conditions.len(), 2);

        assert_eq!(conditions[0].field, "name");
        assert_eq!(conditions[0].operator, "LIKE");
        assert_eq!(conditions[0].value, "%.txt");
        assert!(!conditions[0].negated);

        assert_eq!(conditions[1].field, "type");
        assert_eq!(conditions[1].operator, "=");
        assert_eq!(conditions[1].value, "file");
        assert!(!conditions[1].negated);
    }

    #[test]
    fn test_parse_compound_conditions_case_insensitive_not() {
        let conditions = parse_compound_conditions("name not like '%.tmp'").unwrap();
        assert_eq!(conditions.len(), 1);

        assert_eq!(conditions[0].field, "name");
        assert_eq!(conditions[0].operator, "LIKE");
        assert_eq!(conditions[0].value, "%.tmp");
        assert!(conditions[0].negated);
    }
}
