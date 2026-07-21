pub fn slugify(title: &str) -> String {
    let transliterated = deunicode::deunicode(&title.trim().to_lowercase());
    let mut slug = String::new();
    let mut separator = false;
    for character in transliterated.chars() {
        if character.is_ascii_alphanumeric() {
            if separator && !slug.is_empty() && slug.len() < 48 {
                slug.push('-');
            }
            separator = false;
            if slug.len() < 48 {
                slug.push(character);
            }
        } else {
            separator = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() { "task".into() } else { slug }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_unicode_and_fallback_slugs() {
        assert_eq!(slugify("  Café déjà vu! "), "cafe-deja-vu");
        assert_eq!(slugify("###"), "task");
        assert!(slugify(&"a".repeat(80)).len() <= 48);
    }
}
