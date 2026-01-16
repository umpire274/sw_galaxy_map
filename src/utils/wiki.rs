pub fn fandom_planet_url(name: &str) -> String {
    let normalized = name
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join("_");

    format!("https://starwars.fandom.com/wiki/{}", normalized)
}
