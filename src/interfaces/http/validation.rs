/// Validation helpers for HTTP handlers.
///
/// All functions return `Ok(())` on success or `Err(String)` with a human-readable
/// description of the validation failure.

/// Validates that a command string is not empty.
pub fn validate_command(cmd: &str) -> Result<(), String> {
    if cmd.is_empty() {
        return Err("command must not be empty".to_string());
    }
    Ok(())
}

/// Validates that `timeout_ms` is within the allowed range (max 300_000 ms = 5 min).
pub fn validate_timeout(ms: u64) -> Result<(), String> {
    const MAX_TIMEOUT_MS: u64 = 300_000;
    if ms > MAX_TIMEOUT_MS {
        return Err(format!(
            "timeout_ms {} exceeds maximum allowed value of {}",
            ms, MAX_TIMEOUT_MS
        ));
    }
    Ok(())
}

/// Validates that a sandbox profile name only contains alphanumeric characters and hyphens.
pub fn validate_profile(profile: &str) -> Result<(), String> {
    if profile.is_empty() {
        return Err("profile must not be empty".to_string());
    }
    let valid = profile
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-');
    if !valid {
        return Err(format!(
            "profile '{}' contains invalid characters; only alphanumeric and '-' are allowed",
            profile
        ));
    }
    Ok(())
}

/// Validates a domain name:
/// - Must not be empty
/// - Length 1-253 characters
/// - Must not contain whitespace
pub fn validate_domain(domain: &str) -> Result<(), String> {
    if domain.is_empty() {
        return Err("domain must not be empty".to_string());
    }
    if domain.len() > 253 {
        return Err(format!(
            "domain length {} exceeds maximum of 253 characters",
            domain.len()
        ));
    }
    if domain.chars().any(|c| c.is_whitespace()) {
        return Err(format!("domain '{}' must not contain whitespace", domain));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_command_empty() {
        assert!(validate_command("").is_err());
    }

    #[test]
    fn test_validate_command_valid() {
        assert!(validate_command("echo hello").is_ok());
    }

    #[test]
    fn test_validate_timeout_max() {
        assert!(validate_timeout(300_000).is_ok());
    }

    #[test]
    fn test_validate_timeout_exceeded() {
        assert!(validate_timeout(300_001).is_err());
        assert!(validate_timeout(999_999).is_err());
    }

    #[test]
    fn test_validate_timeout_zero() {
        assert!(validate_timeout(0).is_ok());
    }

    #[test]
    fn test_validate_profile_valid() {
        assert!(validate_profile("default").is_ok());
        assert!(validate_profile("my-profile").is_ok());
        assert!(validate_profile("profile123").is_ok());
    }

    #[test]
    fn test_validate_profile_invalid() {
        assert!(validate_profile("../../etc").is_err());
        assert!(validate_profile("pro file").is_err());
        assert!(validate_profile("pro/file").is_err());
        assert!(validate_profile("").is_err());
    }

    #[test]
    fn test_validate_domain_empty() {
        assert!(validate_domain("").is_err());
    }

    #[test]
    fn test_validate_domain_valid() {
        assert!(validate_domain("example.com").is_ok());
        assert!(validate_domain("sub.domain.org").is_ok());
    }

    #[test]
    fn test_validate_domain_with_space() {
        assert!(validate_domain("evil .com").is_err());
    }

    #[test]
    fn test_validate_domain_too_long() {
        let long = "a".repeat(254);
        assert!(validate_domain(&long).is_err());
    }
}
