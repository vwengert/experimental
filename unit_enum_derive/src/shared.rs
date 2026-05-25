use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

#[derive(Default)]
pub struct Attr {
    pub name: String,
    pub args: Vec<(String, String)>,
}

pub fn parse_named_decl(
    tokens: &[TokenTree],
    keyword: &str,
    decl_error: &str,
    body_error: &str,
) -> Result<(usize, String, Group), String> {
    let Some(decl_index) = tokens
        .iter()
        .position(|token| matches!(token, TokenTree::Ident(ident) if ident.to_string() == keyword))
    else {
        return Err(decl_error.to_string());
    };

    let name = match tokens.get(decl_index + 1) {
        Some(TokenTree::Ident(name)) => name.to_string(),
        _ => return Err(decl_error.to_string()),
    };

    for token in tokens.iter().skip(decl_index + 2) {
        if let TokenTree::Group(group) = token {
            if group.delimiter() == Delimiter::Brace {
                return Ok((decl_index, name, group.clone()));
            }
        }
    }

    Err(body_error.to_string())
}

pub fn parse_kv_args(stream: TokenStream) -> Vec<(String, String)> {
    let mut out = Vec::new();

    for segment in split_by_top_level_comma(stream) {
        if segment.len() < 3 {
            continue;
        }

        let key = match &segment[0] {
            TokenTree::Ident(ident) => ident.to_string(),
            _ => continue,
        };

        if !is_punct(&segment[1], '=') {
            continue;
        }

        let value = segment[2].to_string();
        out.push((key, value));
    }

    out
}

pub fn parse_attr_prefix(tokens: &[TokenTree]) -> Result<(Vec<Attr>, usize), String> {
    let mut attrs = Vec::new();
    let mut i = 0usize;

    while i + 1 < tokens.len() {
        if !is_punct(&tokens[i], '#') {
            break;
        }

        let group = match &tokens[i + 1] {
            TokenTree::Group(group) if group.delimiter() == Delimiter::Bracket => group,
            _ => return Err("Malformed attribute syntax".to_string()),
        };

        attrs.push(parse_attr(group)?);
        i += 2;
    }

    Ok((attrs, i))
}

pub fn parse_attr(group: &Group) -> Result<Attr, String> {
    let mut tokens: Vec<TokenTree> = group.stream().into_iter().collect();
    if tokens.is_empty() {
        return Err("Malformed attribute syntax".to_string());
    }

    let name = match tokens.remove(0) {
        TokenTree::Ident(ident) => ident.to_string(),
        _ => return Err("Malformed attribute syntax".to_string()),
    };

    let mut args = Vec::new();
    if !tokens.is_empty() {
        if tokens.len() != 1 {
            return Err("Malformed attribute syntax".to_string());
        }

        let arg_group = match &tokens[0] {
            TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g,
            _ => return Err("Malformed attribute syntax".to_string()),
        };

        args = parse_kv_args(arg_group.stream());
    }

    Ok(Attr { name, args })
}

pub fn split_by_top_level_comma(stream: TokenStream) -> Vec<Vec<TokenTree>> {
    let mut parts = Vec::new();
    let mut current = Vec::new();
    let mut angle_depth = 0usize;

    for token in stream {
        if let TokenTree::Punct(p) = &token {
            match p.as_char() {
                '<' => angle_depth += 1,
                '>' => {
                    angle_depth = angle_depth.saturating_sub(1);
                }
                ',' if angle_depth == 0 => {
                    parts.push(current);
                    current = Vec::new();
                    continue;
                }
                _ => {}
            }
        }

        if is_punct(&token, ',') && angle_depth == 0 {
            parts.push(current);
            current = Vec::new();
            continue;
        }

        current.push(token);
    }

    parts.push(current);
    parts
}

pub fn is_punct(token: &TokenTree, ch: char) -> bool {
    matches!(token, TokenTree::Punct(p) if p.as_char() == ch)
}

pub fn compile_error(message: &str) -> TokenStream {
    let escaped = string_lit(message);
    let src = format!("compile_error!({escaped});");
    src.parse().unwrap_or_default()
}

pub fn string_lit(value: &str) -> String {
    format!("{:?}", value)
}

pub fn parse_string_literal(raw: &str) -> Option<String> {
    let text = raw.trim();
    if !(text.starts_with('"') && text.ends_with('"')) {
        return None;
    }

    let inner = &text[1..text.len() - 1];
    let mut result = String::with_capacity(inner.len());
    let mut chars = inner.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }

        match chars.next() {
            Some('"') => result.push('"'),
            Some('\\') => result.push('\\'),
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('t') => result.push('\t'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }

    Some(result)
}
