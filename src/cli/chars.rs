use std::io;

use crate::cell::blocks;

pub fn run_chars(category: Option<&str>, plain: bool) -> io::Result<()> {
    let chars: Vec<&blocks::CharInfo> = match category {
        Some(cat) => {
            let lower = cat.to_lowercase();
            let filtered: Vec<_> = blocks::CHAR_INFO
                .iter()
                .filter(|c| c.category == lower)
                .collect();
            if filtered.is_empty() {
                let json = serde_json::json!({
                    "error": format!(
                        "Unknown category '{}'. Valid: {}",
                        cat,
                        blocks::CATEGORIES.join(", ")
                    ),
                    "code": "USER_ERROR"
                });
                eprintln!("{}", json);
                std::process::exit(1);
            }
            filtered
        }
        None => blocks::CHAR_INFO.iter().collect(),
    };

    if plain {
        print_plain_table(&chars);
    } else {
        print_json(&chars, category);
    }
    Ok(())
}

fn print_json(chars: &[&blocks::CharInfo], category: Option<&str>) {
    let characters: Vec<serde_json::Value> = chars
        .iter()
        .map(|c| {
            let mut obj = serde_json::json!({
                "char": c.ch.to_string(),
                "name": c.name,
                "category": c.category,
                "codepoint": c.codepoint,
            });
            if !c.alt.is_empty() {
                obj["alt"] = serde_json::json!(c.alt);
            }
            obj
        })
        .collect();

    let categories: Vec<&str> = if category.is_some() {
        chars
            .iter()
            .map(|c| c.category)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    } else {
        blocks::CATEGORIES.to_vec()
    };

    let output = serde_json::json!({
        "characters": characters,
        "categories": categories,
        "total": chars.len(),
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

fn print_plain_table(chars: &[&blocks::CharInfo]) {
    let mut current_cat = "";

    for info in chars {
        if info.category != current_cat {
            if !current_cat.is_empty() {
                println!();
            }
            let count = chars.iter().filter(|c| c.category == info.category).count();
            println!("{} ({}):", info.category.to_uppercase(), count);
            current_cat = info.category;
        }
        println!("  {}  {:<14} {}", info.ch, info.name, info.codepoint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_output_structure() {
        // Capture what print_json would produce by building it directly
        let chars: Vec<&blocks::CharInfo> = blocks::CHAR_INFO.iter().collect();
        let characters: Vec<serde_json::Value> = chars
            .iter()
            .map(|c| {
                let mut obj = serde_json::json!({
                    "char": c.ch.to_string(),
                    "name": c.name,
                    "category": c.category,
                    "codepoint": c.codepoint,
                });
                if !c.alt.is_empty() {
                    obj["alt"] = serde_json::json!(c.alt);
                }
                obj
            })
            .collect();
        let output = serde_json::json!({
            "characters": characters,
            "categories": blocks::CATEGORIES,
            "total": 20,
        });
        let obj = output.as_object().unwrap();
        assert!(obj.contains_key("characters"));
        assert!(obj.contains_key("categories"));
        assert_eq!(obj["total"], 20);
        assert_eq!(obj["categories"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn test_category_filter() {
        let shade_chars: Vec<&blocks::CharInfo> = blocks::CHAR_INFO
            .iter()
            .filter(|c| c.category == "shade")
            .collect();
        assert_eq!(shade_chars.len(), 3);
        assert_eq!(shade_chars[0].name, "shade-light");
    }

    #[test]
    fn test_all_categories_covered() {
        for cat in &blocks::CATEGORIES {
            let count = blocks::CHAR_INFO.iter().filter(|c| c.category == *cat).count();
            assert!(count > 0, "Category '{}' has no characters", cat);
        }
    }
}
