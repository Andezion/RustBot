use std::vec::Vec;

/// Very small argument parser: splits on whitespace but supports double-quoted strings.
pub fn parse_args(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut esc = false;

    for c in s.chars() {
        if esc {
            cur.push(c);
            esc = false;
            continue;
        }
        match c {
            '\\' => { esc = true; }
            '"' => { in_quotes = !in_quotes; }
            c if c.is_whitespace() && !in_quotes => {
                if !cur.is_empty() { args.push(cur.clone()); cur.clear(); }
            }
            _ => { cur.push(c); }
        }
    }
    if !cur.is_empty() { args.push(cur); }
    args
}
