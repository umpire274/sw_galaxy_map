pub fn truncate_ellipsis(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let len = s.chars().count();
    if len <= width {
        return s.to_string();
    }
    if width == 1 {
        return "…".to_string();
    }
    let take = width - 1;
    let mut out = String::with_capacity(s.len().min(width + 4));
    out.extend(s.chars().take(take));
    out.push('…');
    out
}

pub fn print_kv_block_colored_keys<F>(pairs: &[(&str, String)], color_key: F)
where
    F: Fn(&str) -> String,
{
    let key_w = pairs
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);

    for (k, v) in pairs {
        let key_padded = format!("{:>key_w$}", k, key_w = key_w);
        let key_col = color_key(&key_padded);

        let v = v.trim_end_matches('\n');
        if v.contains('\n') {
            let mut it = v.lines();
            let first = it.next().unwrap_or("");
            println!("{}: {}", key_col, first);
            for line in it {
                println!("{:>key_w$}  {}", "", line, key_w = key_w);
            }
        } else {
            println!("{}: {}", key_col, v);
        }
    }
}
