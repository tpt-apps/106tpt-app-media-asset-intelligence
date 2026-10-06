//! The shared query language: AST, parser and errors (spec §14).
//!
//! One grammar for the GUI search bar and the CLI `search` command, so both
//! surfaces accept exactly the same syntax (spec §3.6). Parsing never panics on
//! adversarial input — it is a fuzz target (spec §19.5).

use serde::{Deserialize, Serialize};
use std::fmt;

/// A parsed search query (spec §14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Query {
    /// Match all assets (empty query / bare `*`).
    MatchAll,
    /// A single term.
    Term(QueryTerm),
    /// Both sides must match.
    And(Box<Query>, Box<Query>),
    /// Either side must match.
    Or(Box<Query>, Box<Query>),
    /// The left side must match and the right side must not.
    Not(Box<Query>, Box<Query>),
}

impl Query {
    /// Number of leaf terms in this query.
    pub fn term_count(&self) -> usize {
        match self {
            Query::MatchAll => 0,
            Query::Term(_) => 1,
            Query::And(a, b) | Query::Or(a, b) | Query::Not(a, b) => {
                a.term_count() + b.term_count()
            }
        }
    }
}

/// A single search term (spec §14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryTerm {
    /// Optional field restriction (`codec:`, `tag:`, `path:`, `ext:`, …).
    pub field: Option<String>,
    /// The value to match; a bare value searches the default fields.
    pub value: String,
    /// `true` when the value came from a quoted phrase and must match as text,
    /// not as a token.
    pub is_phrase: bool,
}

impl fmt::Display for QueryTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(field) = &self.field {
            write!(f, "{field}:")?;
        }
        if self.is_phrase || self.value.contains(' ') {
            write!(f, "\"{}\"", self.value)?;
        } else {
            f.write_str(&self.value)?;
        }
        Ok(())
    }
}

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Query::MatchAll => f.write_str("*"),
            Query::Term(t) => write!(f, "{t}"),
            Query::And(a, b) => write!(f, "({a} AND {b})"),
            Query::Or(a, b) => write!(f, "({a} OR {b})"),
            Query::Not(a, b) => write!(f, "({a} NOT {b})"),
        }
    }
}

/// A query parse failure with byte position, for user-facing messages (spec §13.2).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("query error at byte {position}: {message}")]
pub struct QueryError {
    /// Byte offset into the input where the problem was detected.
    pub position: usize,
    /// What went wrong, phrased for the user.
    pub message: String,
}

/// Parses a query string (spec §14).
///
/// Operator keywords (`AND`, `OR`, `NOT`) are case-sensitive uppercase so
/// lowercase words like "and" in filenames stay searchable terms. An empty
/// query — or a bare `*` — matches everything.
pub fn parse_query(input: &str) -> Result<Query, QueryError> {
    let tokens = lexer::tokenize(input)?;
    if tokens.is_empty() {
        return Ok(Query::MatchAll);
    }
    // A lone `*` is the explicit match-all term.
    if let [Token {
        kind: TokenKind::Word(w),
        ..
    }] = &tokens[..]
    {
        if w == "*" {
            return Ok(Query::MatchAll);
        }
    }
    let mut parser = Parser { tokens, pos: 0 };
    let query = parser.parse_or()?;
    if let Some(tok) = parser.peek() {
        let position = tok.position;
        return Err(QueryError {
            position,
            message: format!("unexpected {}", describe(tok)),
        });
    }
    Ok(query)
}

mod lexer {
    use super::{QueryError, Token, TokenKind};

    pub fn tokenize(input: &str) -> Result<Vec<Token>, QueryError> {
        let mut tokens = Vec::new();
        let bytes = input.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b' ' | b'\t' | b'\r' | b'\n' => i += 1,
                b'(' => {
                    tokens.push(Token {
                        kind: TokenKind::OpenParen,
                        position: i,
                    });
                    i += 1;
                }
                b')' => {
                    tokens.push(Token {
                        kind: TokenKind::CloseParen,
                        position: i,
                    });
                    i += 1;
                }
                b'"' => {
                    let start = i;
                    i += 1;
                    let value_start = i;
                    while i < bytes.len() && bytes[i] != b'"' {
                        i += 1;
                    }
                    if i >= bytes.len() {
                        return Err(QueryError {
                            position: start,
                            message: "unterminated quoted phrase".to_string(),
                        });
                    }
                    let value = input[value_start..i].to_string();
                    i += 1; // closing quote
                    tokens.push(Token {
                        kind: TokenKind::Phrase(value),
                        position: start,
                    });
                }
                _ => {
                    let start = i;
                    while i < bytes.len()
                        && !bytes[i].is_ascii_whitespace()
                        && !matches!(bytes[i], b'(' | b')' | b'"')
                    {
                        i += 1;
                    }
                    let word = &input[start..i];
                    match word {
                        "AND" => tokens.push(Token {
                            kind: TokenKind::And,
                            position: start,
                        }),
                        "OR" => tokens.push(Token {
                            kind: TokenKind::Or,
                            position: start,
                        }),
                        "NOT" => tokens.push(Token {
                            kind: TokenKind::Not,
                            position: start,
                        }),
                        _ => tokens.push(Token {
                            kind: TokenKind::Word(word.to_string()),
                            position: start,
                        }),
                    }
                }
            }
        }
        Ok(tokens)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
    position: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenKind {
    Word(String),
    Phrase(String),
    And,
    Or,
    Not,
    OpenParen,
    CloseParen,
}

fn describe(tok: &Token) -> String {
    describe_token_kind(&tok.kind)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn parse_or(&mut self) -> Result<Query, QueryError> {
        let mut left = self.parse_and()?;
        while matches!(self.peek().map(|t| &t.kind), Some(TokenKind::Or)) {
            self.pos += 1;
            let right = self.parse_and()?;
            left = Query::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Query, QueryError> {
        let mut left = self.parse_not()?;
        loop {
            // Clone the discriminant out so the token borrow ends before we
            // advance; the token list itself is never mutated.
            let action = match self.peek().map(|t| t.kind.clone()) {
                Some(TokenKind::And) => AndAction::Explicit,
                Some(
                    TokenKind::Word(_)
                    | TokenKind::Phrase(_)
                    | TokenKind::OpenParen
                    | TokenKind::Not,
                ) => {
                    // Adjacent terms/parentheses imply AND (e.g. `prores (tag:interview)`),
                    // and infix NOT is an AND with a negated right side (`a NOT b`).
                    AndAction::Implicit
                }
                _ => break,
            };
            if action == AndAction::Explicit {
                self.pos += 1;
            }
            let right = self.parse_not()?;
            left = Query::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// Prefix NOT (`NOT tag:b`) and primary terms; infix NOT arrives here via
    /// the implicit-AND branch above.
    fn parse_not(&mut self) -> Result<Query, QueryError> {
        if matches!(self.peek().map(|t| &t.kind), Some(TokenKind::Not)) {
            self.pos += 1;
            let positive = self.parse_primary()?;
            return Ok(Query::Not(Box::new(Query::MatchAll), Box::new(positive)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Query, QueryError> {
        let Some(tok) = self.peek().cloned() else {
            return Err(QueryError {
                position: input_end(&self.tokens),
                message: "unexpected end of query".to_string(),
            });
        };
        let position = tok.position;
        match tok.kind {
            TokenKind::OpenParen => {
                self.pos += 1;
                let inner = self.parse_or()?;
                if matches!(self.peek().map(|t| &t.kind), Some(TokenKind::CloseParen)) {
                    self.pos += 1;
                } else {
                    return Err(QueryError {
                        position,
                        message: "missing closing parenthesis".to_string(),
                    });
                }
                Ok(inner)
            }
            TokenKind::Phrase(value) => {
                let term = QueryTerm {
                    field: None,
                    value,
                    is_phrase: true,
                };
                self.pos += 1;
                Ok(Query::Term(term))
            }
            TokenKind::Word(word) => {
                let term = self.split_field(&word);
                self.pos += 1;
                Ok(Query::Term(term))
            }
            other => Err(QueryError {
                position,
                message: format!("unexpected {}", describe_token_kind(&other)),
            }),
        }
    }

    /// Splits `field:value` into its parts; a trailing colon alone is an error.
    fn split_field(&self, word: &str) -> QueryTerm {
        if let Some(idx) = word.find(':') {
            let field = &word[..idx];
            let value = &word[idx + 1..];
            if !field.is_empty() && !value.is_empty() {
                return QueryTerm {
                    field: Some(field.to_ascii_lowercase()),
                    value: value.to_string(),
                    is_phrase: false,
                };
            }
        }
        QueryTerm {
            field: None,
            value: word.to_string(),
            is_phrase: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AndAction {
    Explicit,
    Implicit,
}

fn describe_token_kind(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Word(w) => format!("term `{w}`"),
        TokenKind::Phrase(p) => format!("phrase \"{p}\""),
        TokenKind::And => "`AND`".to_string(),
        TokenKind::Or => "`OR`".to_string(),
        TokenKind::Not => "`NOT`".to_string(),
        TokenKind::OpenParen => "`(`".to_string(),
        TokenKind::CloseParen => "`)`".to_string(),
    }
}

fn input_end(tokens: &[Token]) -> usize {
    tokens.last().map(|t| t.position + 1).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_spec_example() {
        // `codec:prores AND tag:interview` (spec §14).
        let q = parse_query("codec:prores AND tag:interview").unwrap();
        assert_eq!(q.term_count(), 2);
        assert_eq!(
            q,
            Query::And(
                Box::new(Query::Term(QueryTerm {
                    field: Some("codec".to_string()),
                    value: "prores".to_string(),
                    is_phrase: false,
                })),
                Box::new(Query::Term(QueryTerm {
                    field: Some("tag".to_string()),
                    value: "interview".to_string(),
                    is_phrase: false,
                })),
            )
        );
    }

    #[test]
    fn adjacent_terms_imply_and() {
        let q = parse_query("prores interview").unwrap();
        assert_eq!(q.term_count(), 2);
        assert!(matches!(q, Query::And(_, _)));
    }

    #[test]
    fn or_binds_looser_than_and() {
        let q = parse_query("a OR b AND c").unwrap();
        // (a OR (b AND c))
        match q {
            Query::Or(l, r) => {
                assert!(matches!(*l, Query::Term(_)));
                assert!(matches!(*r, Query::And(_, _)));
            }
            other => panic!("unexpected shape: {other:?}"),
        }
    }

    #[test]
    fn parentheses_group() {
        let q = parse_query("(a OR b) AND c").unwrap();
        match q {
            Query::And(l, _) => assert!(matches!(*l, Query::Or(_, _))),
            other => panic!("unexpected shape: {other:?}"),
        }
    }

    #[test]
    fn phrases_match_as_text() {
        let q = parse_query("\"press conference\"").unwrap();
        assert_eq!(
            q,
            Query::Term(QueryTerm {
                field: None,
                value: "press conference".to_string(),
                is_phrase: true,
            })
        );
        assert_eq!(q.to_string(), "\"press conference\"");
    }

    #[test]
    fn fields_are_lowercased_values_are_not() {
        let q = parse_query("CODEC:ProRes").unwrap();
        match q {
            Query::Term(t) => {
                assert_eq!(t.field.as_deref(), Some("codec"));
                assert_eq!(t.value, "ProRes");
            }
            other => panic!("unexpected shape: {other:?}"),
        }
    }

    #[test]
    fn display_round_trip_is_reparseable() {
        for input in [
            "codec:prores AND tag:interview",
            "a OR b AND c",
            "(a OR b) AND \"two words\"",
            "*",
        ] {
            let q = parse_query(input).unwrap();
            let displayed = q.to_string();
            let reparsed = parse_query(&displayed).unwrap();
            assert_eq!(
                q, reparsed,
                "round trip failed for {input:?} -> {displayed:?}"
            );
        }
    }

    #[test]
    fn empty_and_star_mean_match_all() {
        assert_eq!(parse_query("").unwrap(), Query::MatchAll);
        assert_eq!(parse_query("   ").unwrap(), Query::MatchAll);
        assert_eq!(parse_query("*").unwrap(), Query::MatchAll);
    }

    #[test]
    fn errors_carry_positions_not_panics() {
        let err = parse_query("(a OR b").unwrap_err();
        assert!(err.message.contains("closing parenthesis"), "got: {err}");
        assert_eq!(err.position, 0);

        let err = parse_query("a AND").unwrap_err();
        assert!(err.message.contains("end of query"), "got: {err}");

        let err = parse_query("\"unterminated").unwrap_err();
        assert!(err.message.contains("unterminated"), "got: {err}");
        assert_eq!(err.position, 0);

        let err = parse_query("a b )").unwrap_err();
        assert!(err.message.contains("unexpected"), "got: {err}");
    }

    #[test]
    fn lowercase_operators_are_plain_terms() {
        // Lowercase "and" must remain searchable text (operator keywords are uppercase).
        let q = parse_query("rock and roll").unwrap();
        assert_eq!(q.term_count(), 3);
    }

    #[test]
    fn arbitrary_input_never_panics() {
        // Deterministic smoke sweep over adversarial shapes; the fuzz target
        // (spec §19.5) generalizes this.
        for input in [
            "",
            " ",
            "((((",
            "))))",
            "\"\"\"\"",
            "a:b:c",
            ":",
            "a:",
            ":b",
            "NOT",
            "a NOT",
            "NOT NOT a",
            "((a)AND(b))",
            "é",
            "aé AND",
            "()",
        ] {
            let _ = parse_query(input);
        }
    }

    #[test]
    fn not_is_structured_for_evaluation() {
        // Infix NOT is an implicit AND with a negated right side: `a AND NOT tag:b`.
        let q = parse_query("a NOT tag:b").unwrap();
        assert_eq!(q.term_count(), 2);
        match &q {
            Query::And(left, right) => {
                assert!(matches!(**left, Query::Term(_)), "got: {left:?}");
                assert!(matches!(**right, Query::Not(_, _)), "got: {right:?}");
            }
            other => panic!("unexpected shape: {other:?}"),
        }

        // Prefix NOT negates its primary against the universe.
        let q = parse_query("NOT tag:b").unwrap();
        assert!(matches!(q, Query::Not(_, _)), "got: {q:?}");
    }
}
