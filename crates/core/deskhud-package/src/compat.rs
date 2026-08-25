//! 引擎产品版本与包 `engine` 兼容族匹配。
//!
//! 政策见仓库 `docs/versioning.md`。

/// 当前 DeskHud 引擎产品 SemVer（本 crate / workspace）。
pub const ENGINE_PRODUCT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 产品版本 `"0.3.5"` → 族 `"0.3"`；`"1.4.2"` → 族 `"1"`。
pub fn engine_family_of_product(product_version: &str) -> String {
    let ver = product_version.trim();
    let mut parts = ver.split('.');
    let major = parts.next().unwrap_or("").trim();
    let minor = parts.next().unwrap_or("").trim();
    let major_num: u64 = major
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0);
    if major_num == 0 {
        let minor_num: String = minor.chars().take_while(|c| c.is_ascii_digit()).collect();
        format!("{major_num}.{minor_num}")
    } else {
        major_num.to_string()
    }
}

/// 包声明的 `engine` 是否与当前产品版本的兼容族一致。
pub fn pack_engine_matches(pack_engine: &str, product_version: &str) -> bool {
    pack_engine.trim() == engine_family_of_product(product_version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_pre_1() {
        assert_eq!(engine_family_of_product("0.3.5"), "0.3");
        assert_eq!(engine_family_of_product("0.2.0"), "0.2");
        assert_eq!(engine_family_of_product("0.10.1"), "0.10");
    }

    #[test]
    fn family_major_1_plus() {
        assert_eq!(engine_family_of_product("1.4.2"), "1");
        assert_eq!(engine_family_of_product("2.0.0"), "2");
    }

    #[test]
    fn match_pre_1() {
        assert!(pack_engine_matches("0.3", "0.3.5"));
        assert!(!pack_engine_matches("0.2", "0.3.5"));
        assert!(!pack_engine_matches("0.3.5", "0.3.5"));
    }

    #[test]
    fn match_major_1_plus() {
        assert!(pack_engine_matches("1", "1.4.2"));
        assert!(!pack_engine_matches("1.4", "1.4.2"));
        assert!(!pack_engine_matches("2", "1.4.2"));
    }

    #[test]
    fn current_product_family_is_consistent() {
        let family = engine_family_of_product(ENGINE_PRODUCT_VERSION);
        assert!(pack_engine_matches(&family, ENGINE_PRODUCT_VERSION));
    }
}
