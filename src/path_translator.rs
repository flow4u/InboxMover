/// Cross-platform path translator for known myDRE mounts:
/// - Windows `z:\` <-> Linux `/mnt/data/`
/// - Windows `i:\` <-> Linux `/mnt/inbox/`
pub struct PathTranslator;

impl PathTranslator {
    /// Translates paths between Windows and Linux for known mounts.
    /// Uses compile-time target OS by default.
    pub fn translate(path: &str) -> String {
        Self::translate_for_os(path, cfg!(target_os = "windows"))
    }

    /// Explicit translation targeting Windows (`is_windows = true`) or Linux (`is_windows = false`).
    pub fn translate_for_os(path: &str, is_windows: bool) -> String {
        if path.is_empty() {
            return String::new();
        }

        let p_check = path.replace('\\', "/");

        if is_windows {
            // Translate Linux -> Windows
            if p_check.starts_with("/mnt/data/") {
                let suffix = &path[10..].replace('/', "\\");
                format!("z:\\{suffix}")
            } else if p_check.starts_with("/mnt/inbox/") {
                let suffix = &path[11..].replace('/', "\\");
                format!("i:\\{suffix}")
            } else if p_check == "/mnt/data" {
                "z:\\".to_string()
            } else if p_check == "/mnt/inbox" {
                "i:\\".to_string()
            } else {
                path.to_string()
            }
        } else {
            // Translate Windows -> Linux
            let p_lower = p_check.to_lowercase();
            if p_lower.starts_with("z:/") {
                let suffix = &path[3..].replace('\\', "/");
                format!("/mnt/data/{suffix}")
            } else if p_lower.starts_with("i:/") {
                let suffix = &path[3..].replace('\\', "/");
                format!("/mnt/inbox/{suffix}")
            } else if p_lower == "z:" || p_lower == "z:/" {
                "/mnt/data/".to_string()
            } else if p_lower == "i:" || p_lower == "i:/" {
                "/mnt/inbox/".to_string()
            } else {
                path.to_string()
            }
        }
    }

    /// Checks if a path belongs to an incompatible OS and is not one of the known shared mounts.
    pub fn is_incompatible_path(path: &str, is_windows: bool) -> bool {
        if path.is_empty() {
            return false;
        }

        let p_low = path.to_lowercase().replace('\\', "/");
        if p_low.starts_with("i:") || p_low.starts_with("z:") || p_low.starts_with("/mnt") {
            return false;
        }

        let looks_like_linux = p_low.starts_with('/');
        let looks_like_windows = p_low.len() >= 2
            && p_low.chars().next().map_or(false, |c| c.is_ascii_alphabetic())
            && p_low.chars().nth(1) == Some(':');

        if is_windows && looks_like_linux {
            return true;
        }
        if !is_windows && looks_like_windows {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_linux_to_windows() {
        assert_eq!(
            PathTranslator::translate_for_os("/mnt/data/projects/data.csv", true),
            "z:\\projects\\data.csv"
        );
        assert_eq!(
            PathTranslator::translate_for_os("/mnt/inbox/transfer-123", true),
            "i:\\transfer-123"
        );
        assert_eq!(
            PathTranslator::translate_for_os("/mnt/data", true),
            "z:\\"
        );
        assert_eq!(
            PathTranslator::translate_for_os("/mnt/inbox", true),
            "i:\\"
        );
    }

    #[test]
    fn test_translate_windows_to_linux() {
        assert_eq!(
            PathTranslator::translate_for_os("z:\\projects\\data.csv", false),
            "/mnt/data/projects/data.csv"
        );
        assert_eq!(
            PathTranslator::translate_for_os("i:\\transfer-123", false),
            "/mnt/inbox/transfer-123"
        );
        assert_eq!(
            PathTranslator::translate_for_os("Z:", false),
            "/mnt/data/"
        );
        assert_eq!(
            PathTranslator::translate_for_os("i:/", false),
            "/mnt/inbox/"
        );
    }

    #[test]
    fn test_incompatible_paths() {
        // C:\ on Linux is incompatible
        assert!(PathTranslator::is_incompatible_path("C:\\Users\\Data", false));
        // /home/user on Windows is incompatible
        assert!(PathTranslator::is_incompatible_path("/home/user/data", true));
        // z:\ and /mnt/ are known and cross-platform
        assert!(!PathTranslator::is_incompatible_path("z:\\inbox", false));
        assert!(!PathTranslator::is_incompatible_path("/mnt/data", true));
    }
}

