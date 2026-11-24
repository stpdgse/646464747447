//! English translations, for the few messages the server renders itself.

pub use crate::generated::lang::EN_US;

/// The English text for a translation key.
pub fn en_us(key: &str) -> Option<&'static str> {
    EN_US.binary_search_by(|(k, _)| (*k).cmp(key)).ok().map(|i| EN_US[i].1)
}

/// Renders `key` with Minecraft's `%s` / `%1$s` substitution; unknown keys render as the key.
pub fn render(key: &str, args: &[String]) -> String {
    let Some(fmt) = en_us(key) else {
        return key.to_string();
    };
    let mut out = String::new();
    let mut next = 0;
    let mut it = fmt.chars().peekable();
    while let Some(c) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let mut digits = String::new();
        while let Some(d) = it.peek().copied().filter(char::is_ascii_digit) {
            digits.push(d);
            it.next();
        }
        match it.next() {
            Some('%') => out.push('%'),
            Some('$') => {
                if let (Ok(n), Some('s')) = (digits.parse::<usize>(), it.next()) {
                    out.push_str(args.get(n.wrapping_sub(1)).map_or("", String::as_str));
                }
            }
            Some('s') => {
                out.push_str(args.get(next).map_or("", String::as_str));
                next += 1;
            }
            Some(o) => {
                out.push('%');
                out.push_str(&digits);
                out.push(o);
            }
            None => out.push('%'),
        }
    }
    out
}
