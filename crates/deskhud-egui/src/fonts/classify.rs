//! 字体文件名 → 家族 / 样式。

/// 解析 stem → (家族键片段, 显示名, 样式名, 搜索别名)。
pub fn classify_stem(stem: &str) -> (String, String, String, Vec<String>) {
    let lower = stem.to_ascii_lowercase();

    // Windows 短文件名
    if let Some(m) = classify_windows_short(&lower) {
        return m;
    }

    // Family-Style：JetBrainsMono-BoldItalic
    if let Some((fam_code, style_raw)) = stem.split_once('-') {
        let style = humanize_style(style_raw);
        let (fam_key, label) = family_from_code(fam_code);
        let mut aliases = vec![
            label.to_lowercase(),
            fam_key.clone(),
            style.to_lowercase(),
            stem.to_ascii_lowercase(),
        ];
        aliases.extend(chinese_aliases(&label));
        return (fam_key, label, style, aliases);
    }

    // 无连字符：尝试后缀 Bold/Light
    if let Some((fam_key, label, style)) = classify_suffix_style(stem, &lower) {
        let mut aliases = vec![label.to_lowercase(), fam_key.clone(), style.to_lowercase()];
        aliases.extend(chinese_aliases(&label));
        return (fam_key, label, style, aliases);
    }

    let (fam_key, label) = family_from_code(stem);
    let mut aliases = vec![label.to_lowercase(), fam_key.clone()];
    aliases.extend(chinese_aliases(&label));
    (fam_key, label, "Regular".into(), aliases)
}

fn classify_windows_short(lower: &str) -> Option<(String, String, String, Vec<String>)> {
    let mapped: Option<(&str, &str, &str, &[&str])> = match lower {
        "msyh" | "microsoftyahei" | "msyhui" => Some((
            "msyh",
            "微软雅黑",
            "Regular",
            &["微软雅黑", "microsoft yahei", "yahei", "msyh"],
        )),
        "msyhbd" | "microsoftyaheibold" => Some((
            "msyh",
            "微软雅黑",
            "Bold",
            &["微软雅黑", "microsoft yahei", "msyhbd", "bold"],
        )),
        "msyhl" | "microsoftyaheilight" => Some((
            "msyh",
            "微软雅黑",
            "Light",
            &["微软雅黑", "microsoft yahei", "msyhl", "light"],
        )),
        "simhei" => Some(("simhei", "黑体", "Regular", &["黑体", "simhei"])),
        "simsun" | "nsimsun" => Some(("simsun", "宋体", "Regular", &["宋体", "simsun"])),
        "simkai" | "kaiu" => Some(("simkai", "楷体", "Regular", &["楷体", "simkai"])),
        "simfang" | "fangsong" => Some(("simfang", "仿宋", "Regular", &["仿宋", "simfang"])),
        "dengxian" | "deng" => Some(("dengxian", "等线", "Regular", &["等线", "dengxian"])),
        "msjh" | "microsoftjhenghei" => Some((
            "msjh",
            "微软正黑体",
            "Regular",
            &["微软正黑体", "microsoft jhenghei", "msjh"],
        )),
        "msjhbd" => Some((
            "msjh",
            "微软正黑体",
            "Bold",
            &["微软正黑体", "microsoft jhenghei", "msjhbd"],
        )),
        _ => None,
    };
    mapped.map(|(k, l, s, a)| {
        (
            k.into(),
            l.into(),
            s.into(),
            a.iter().map(|x| (*x).to_string()).collect(),
        )
    })
}

fn classify_suffix_style(stem: &str, lower: &str) -> Option<(String, String, String)> {
    let endings = [
        ("bolditalic", "Bold Italic"),
        ("extrabolditalic", "ExtraBold Italic"),
        ("semibolditalic", "SemiBold Italic"),
        ("mediumitalic", "Medium Italic"),
        ("lightitalic", "Light Italic"),
        ("extralightitalic", "ExtraLight Italic"),
        ("thinitalic", "Thin Italic"),
        ("extrabold", "ExtraBold"),
        ("semibold", "SemiBold"),
        ("extralight", "ExtraLight"),
        ("bold", "Bold"),
        ("medium", "Medium"),
        ("light", "Light"),
        ("thin", "Thin"),
        ("italic", "Italic"),
        ("regular", "Regular"),
    ];
    for (end, style) in endings {
        if let Some(base) = lower.strip_suffix(end) {
            if base.is_empty() {
                continue;
            }
            // 保留原 stem 前缀大小写长度
            let code = &stem[..base.len().min(stem.len())];
            let (fam_key, label) = family_from_code(code);
            return Some((fam_key, label, style.into()));
        }
    }
    None
}

fn family_from_code(code: &str) -> (String, String) {
    let key = code
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    let label = match key.as_str() {
        "jetbrainsmono" => "JetBrains Mono".into(),
        "jetbrainsmononl" => "JetBrains Mono NL".into(),
        "notosanssc" => "Noto Sans SC".into(),
        "notosans" => "Noto Sans".into(),
        _ => humanize_family_code(code),
    };
    (key, label)
}

fn humanize_family_code(code: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = code.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && ch.is_uppercase() {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).is_some_and(|c| c.is_lowercase());
            if prev.is_lowercase() || next_lower {
                out.push(' ');
            }
        }
        out.push(ch);
    }
    out
}

fn humanize_style(raw: &str) -> String {
    // BoldItalic → Bold Italic
    let mut out = String::new();
    let chars: Vec<char> = raw.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && ch.is_uppercase() {
            let prev = chars[i - 1];
            if prev.is_lowercase()
                || prev.is_ascii_digit()
                || chars.get(i + 1).is_some_and(|c| c.is_lowercase())
            {
                out.push(' ');
            }
        }
        out.push(ch);
    }
    if out.is_empty() {
        "Regular".into()
    } else {
        out
    }
}

fn chinese_aliases(label: &str) -> Vec<String> {
    match label {
        "Noto Sans SC" => vec!["思源".into(), "黑体".into()],
        "JetBrains Mono" | "JetBrains Mono NL" => vec!["等宽".into(), "jetbrains".into()],
        _ => Vec::new(),
    }
}

/// 样式排序：(字重, 是否斜体)。
pub fn style_sort_key(style: &str) -> (u8, u8) {
    let lower = style.to_ascii_lowercase();
    let italic = lower.contains("italic");
    let base = lower
        .replace(" italic", "")
        .replace("italic", "")
        .replace(' ', "");
    let w = match base.as_str() {
        "thin" => 0,
        "extralight" => 1,
        "light" => 2,
        "demilight" => 3,
        "regular" | "" => 4,
        "medium" | "book" => 5,
        "semibold" | "demibold" => 6,
        "bold" => 7,
        "extrabold" => 8,
        "black" | "heavy" => 9,
        _ => 40,
    };
    (w, u8::from(italic))
}

/// 规范化 prefs 里的样式名。
pub fn normalize_style_name(s: &str) -> String {
    match s.trim().to_ascii_lowercase().as_str() {
        "" | "regular" => "Regular".into(),
        "bold" => "Bold".into(),
        "light" => "Light".into(),
        "thin" => "Thin".into(),
        "demilight" | "demi light" => "DemiLight".into(),
        "medium" => "Medium".into(),
        "semibold" | "semi bold" => "SemiBold".into(),
        "extrabold" | "extra bold" => "ExtraBold".into(),
        "extralight" | "extra light" => "ExtraLight".into(),
        "black" | "heavy" => "Black".into(),
        other => {
            // 保留已是 Title Case 的
            if s.chars().any(|c| c.is_uppercase()) {
                s.to_string()
            } else {
                humanize_style(other)
            }
        }
    }
}
