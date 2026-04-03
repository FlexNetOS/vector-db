//! Single-pass JavaScript bundle parser.
//!
//! Extracts top-level declarations, string literals, property accesses,
//! and cross-references from minified JS without a full AST.
//!
//! Performance: Uses a single-pass scanner with brace-depth tracking
//! instead of per-declaration regex scanning. This reduces O(n*m) to O(n)
//! for large files (n=file size, m=declarations).

use std::collections::HashSet;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::error::{DecompilerError, Result};
use crate::types::{DeclKind, Declaration};

// Cached compiled regexes -- compiled once, reused across all calls.
static VAR_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:^|[;}\s])(var|let|const)\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*=")
        .expect("valid regex")
});

static FN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:^|[;}\s])function\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*\(")
        .expect("valid regex")
});

static CLASS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:^|[;}\s])class\s+([a-zA-Z_$][a-zA-Z0-9_$]*)\s*[{\(]")
        .expect("valid regex")
});

static EXPORT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:^|[;}\s])export\s+(?:default\s+)?(?:function|class|const|let|var)\s+([a-zA-Z_$][a-zA-Z0-9_$]*)")
        .expect("valid regex")
});

/// Parse a minified JavaScript bundle and extract declarations.
pub fn parse_bundle(source: &str) -> Result<Vec<Declaration>> {
    if source.trim().is_empty() {
        return Err(DecompilerError::EmptyBundle(
            "source is empty".to_string(),
        ));
    }

    let decls = extract_declarations(source);
    if decls.is_empty() {
        return Err(DecompilerError::NoDeclarations);
    }

    Ok(decls)
}

/// Extract top-level declarations from source using regex heuristics
/// combined with a single-pass metadata scanner.
fn extract_declarations(source: &str) -> Vec<Declaration> {
    let mut declarations = Vec::new();
    let mut all_names: HashSet<String> = HashSet::new();

    // --- var/let/const ---
    for cap in VAR_RE.captures_iter(source) {
        let kind = match &cap[1] {
            "var" => DeclKind::Var,
            "let" => DeclKind::Let,
            "const" => DeclKind::Const,
            _ => continue,
        };
        let name = cap[2].to_string();
        let match_start = cap.get(0).map_or(0, |m| m.start());
        let body_end = find_declaration_end(source, match_start);

        all_names.insert(name.clone());
        declarations.push(Declaration {
            name,
            kind,
            byte_range: (match_start, body_end),
            string_literals: Vec::new(),
            property_accesses: Vec::new(),
            references: Vec::new(),
        });
    }

    // --- function ---
    for cap in FN_RE.captures_iter(source) {
        let name = cap[1].to_string();
        let match_start = cap.get(0).map_or(0, |m| m.start());
        let body_end = find_declaration_end(source, match_start);

        all_names.insert(name.clone());
        declarations.push(Declaration {
            name,
            kind: DeclKind::Function,
            byte_range: (match_start, body_end),
            string_literals: Vec::new(),
            property_accesses: Vec::new(),
            references: Vec::new(),
        });
    }

    // --- class ---
    for cap in CLASS_RE.captures_iter(source) {
        let name = cap[1].to_string();
        let match_start = cap.get(0).map_or(0, |m| m.start());
        let body_end = find_declaration_end(source, match_start);

        all_names.insert(name.clone());
        declarations.push(Declaration {
            name,
            kind: DeclKind::Class,
            byte_range: (match_start, body_end),
            string_literals: Vec::new(),
            property_accesses: Vec::new(),
            references: Vec::new(),
        });
    }

    // --- export declarations (ES modules) ---
    for cap in EXPORT_RE.captures_iter(source) {
        let name = cap[1].to_string();
        if all_names.contains(&name) {
            continue;
        }
        let match_start = cap.get(0).map_or(0, |m| m.start());
        let body_end = find_declaration_end(source, match_start);

        all_names.insert(name.clone());
        declarations.push(Declaration {
            name,
            kind: DeclKind::Const,
            byte_range: (match_start, body_end),
            string_literals: Vec::new(),
            property_accesses: Vec::new(),
            references: Vec::new(),
        });
    }

    // Single-pass metadata extraction: scan each declaration's body ONCE
    // to collect strings, properties, and identifiers simultaneously.
    for decl in &mut declarations {
        let (start, end) = decl.byte_range;
        let end = end.min(source.len());
        let body = &source[start..end];

        let (strings, props, idents) = scan_body_single_pass(body);
        decl.string_literals = strings;

        // Deduplicate properties.
        let mut seen_props: HashSet<String> = HashSet::new();
        for prop in props {
            if seen_props.insert(prop.clone()) {
                decl.property_accesses.push(prop);
            }
        }

        // Cross-references: identifiers that match other declaration names.
        let mut seen_refs: HashSet<String> = HashSet::new();
        for ident in idents {
            if ident != decl.name
                && all_names.contains(&ident)
                && seen_refs.insert(ident.clone())
            {
                decl.references.push(ident);
            }
        }
    }

    declarations
}

/// Scan a declaration body in a SINGLE PASS to extract:
/// - String literals
/// - Property accesses (after '.')
/// - Identifiers (for cross-reference detection)
///
/// This replaces three separate regex passes (STRING_RE, PROP_RE, IDENT_RE)
/// with one character-level scan, reducing time from O(3*n) to O(n).
fn scan_body_single_pass(body: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
    let bytes = body.as_bytes();
    let len = bytes.len();
    let mut strings = Vec::new();
    let mut props = Vec::new();
    let mut idents = Vec::new();

    let mut i = 0;
    while i < len {
        let ch = bytes[i];

        // --- String literal ---
        if ch == b'"' || ch == b'\'' {
            let quote = ch;
            i += 1;
            let str_start = i;
            while i < len {
                if bytes[i] == b'\\' {
                    i += 2; // skip escape
                    continue;
                }
                if bytes[i] == quote {
                    break;
                }
                i += 1;
            }
            if i > str_start {
                let s = String::from_utf8_lossy(&bytes[str_start..i]).to_string();
                if !s.is_empty() {
                    strings.push(s);
                }
            }
            if i < len {
                i += 1; // skip closing quote
            }
            continue;
        }

        // --- Template literal (skip, don't parse contents as code) ---
        if ch == b'`' {
            i += 1;
            while i < len {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'`' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // --- Property access (after '.') ---
        if ch == b'.' && i + 1 < len && is_ident_start(bytes[i + 1]) {
            i += 1;
            let prop_start = i;
            while i < len && is_ident_char(bytes[i]) {
                i += 1;
            }
            let prop = String::from_utf8_lossy(&bytes[prop_start..i]).to_string();
            props.push(prop);
            continue;
        }

        // --- Identifier ---
        if is_ident_start(ch) {
            let ident_start = i;
            while i < len && is_ident_char(bytes[i]) {
                i += 1;
            }
            let ident = String::from_utf8_lossy(&bytes[ident_start..i]).to_string();
            idents.push(ident);
            continue;
        }

        i += 1;
    }

    (strings, props, idents)
}

#[inline]
fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$'
}

#[inline]
fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// Find the end of a declaration body by tracking brace depth,
/// or falling back to the next semicolon at depth 0.
fn find_declaration_end(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let mut brace_depth = 0i32;
    let mut paren_depth = 0i32;
    let mut in_string = false;
    let mut string_char = b'"';
    let mut found_brace = false;

    let mut i = start;
    while i < bytes.len() {
        let ch = bytes[i];

        // Handle string escapes.
        if in_string {
            if ch == b'\\' {
                i += 2;
                continue;
            }
            if ch == string_char {
                in_string = false;
            }
            i += 1;
            continue;
        }

        match ch {
            b'"' | b'\'' | b'`' => {
                in_string = true;
                string_char = ch;
            }
            b'{' => {
                brace_depth += 1;
                found_brace = true;
            }
            b'}' => {
                brace_depth -= 1;
                if found_brace && brace_depth <= 0 {
                    // Consume trailing semicolon if present.
                    if i + 1 < bytes.len() && bytes[i + 1] == b';' {
                        return i + 2;
                    }
                    return i + 1;
                }
            }
            b'(' => {
                paren_depth += 1;
            }
            b')' => {
                paren_depth -= 1;
            }
            b';' if brace_depth <= 0 && paren_depth <= 0 && i > start + 2 => {
                return i + 1;
            }
            _ => {}
        }

        i += 1;
    }

    source.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_var_declarations() {
        let src = r#"var a=function(){return"hello"};var b=42;"#;
        let decls = parse_bundle(src).unwrap();
        assert!(decls.len() >= 2);
        assert_eq!(decls[0].name, "a");
        assert_eq!(decls[0].kind, DeclKind::Var);
        assert!(decls[0].string_literals.contains(&"hello".to_string()));
    }

    #[test]
    fn test_parse_class() {
        let src = r#"var x=1;class Foo{constructor(){this.name="test"}}"#;
        let decls = parse_bundle(src).unwrap();
        let class_decl = decls.iter().find(|d| d.kind == DeclKind::Class);
        assert!(class_decl.is_some());
        assert_eq!(class_decl.unwrap().name, "Foo");
    }

    #[test]
    fn test_cross_references() {
        let src = r#"var a=function(){return 1};var b=function(){return a()}"#;
        let decls = parse_bundle(src).unwrap();
        let b_decl = decls.iter().find(|d| d.name == "b").unwrap();
        assert!(b_decl.references.contains(&"a".to_string()));
    }

    #[test]
    fn test_empty_bundle() {
        let result = parse_bundle("");
        assert!(result.is_err());
    }

    #[test]
    fn test_single_pass_scanner() {
        let body = r#"function(){return"hello"+x.name+y}"#;
        let (strings, props, idents) = scan_body_single_pass(body);
        assert!(strings.contains(&"hello".to_string()));
        assert!(props.contains(&"name".to_string()));
        assert!(idents.contains(&"function".to_string()));
        assert!(idents.contains(&"return".to_string()));
        assert!(idents.contains(&"x".to_string()));
        assert!(idents.contains(&"y".to_string()));
    }
}
