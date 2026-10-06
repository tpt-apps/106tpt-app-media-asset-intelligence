//! Minimal `field:value` + AND query parser (spec §14 CLI search syntax).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("empty query")]
    Empty,
    #[error("malformed clause: {0}")]
    Malformed(String),
}

#[derive(Debug, Clone)]
pub struct Clause {
    pub field: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Query {
    pub clauses: Vec<Clause>,
    pub raw: String,
}

/// Parse e.g. `codec:av1 AND tag:interview`. Bare words search all fields.
/// `AND` (case-insensitive) separates clauses; anything else is an error.
pub fn parse_query(raw: &str) -> Result<Query, QueryError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(QueryError::Empty);
    }
    let mut clauses = Vec::new();
    // Split on AND case-insensitively by scanning words.
    let mut current = String::new();
    for token in trimmed.split_whitespace() {
        if token.eq_ignore_ascii_case("and") {
            push_clause(&mut clauses, current.trim())?;
            current.clear();
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(token);
        }
    }
    push_clause(&mut clauses, current.trim())?;
    Ok(Query {
        clauses,
        raw: trimmed.to_string(),
    })
}

fn push_clause(clauses: &mut Vec<Clause>, text: &str) -> Result<(), QueryError> {
    if text.is_empty() {
        return Err(QueryError::Malformed("empty clause".to_string()));
    }
    if let Some((field, value)) = text.split_once(':') {
        let (field, value) = (field.trim(), value.trim());
        if field.is_empty() || value.is_empty() {
            return Err(QueryError::Malformed(text.to_string()));
        }
        clauses.push(Clause {
            field: field.to_ascii_lowercase(),
            value: value.to_string(),
        });
    } else {
        clauses.push(Clause {
            field: "text".to_string(),
            value: text.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_field_queries() {
        let q = parse_query("codec:av1 AND tag:interview").unwrap();
        assert_eq!(q.clauses.len(), 2);
        assert_eq!(q.clauses[0].field, "codec");
    }

    #[test]
    fn bare_word_searches_all_fields() {
        let q = parse_query("interview").unwrap();
        assert_eq!(q.clauses[0].field, "text");
    }

    #[test]
    fn empty_query_errors() {
        assert!(matches!(parse_query("  "), Err(QueryError::Empty)));
    }

    #[test]
    fn dangling_colon_errors() {
        assert!(parse_query("codec:").is_err());
    }
}
