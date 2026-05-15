/// Input validation helpers for Tauri command handlers.
///
/// All public functions return `Result<(), String>` so they can be used
/// directly with the `?` operator inside `#[tauri::command]` handlers.

/// Validates that a character ID is positive.
pub fn validate_character_id(char_id: i64) -> Result<(), String> {
    if char_id <= 0 {
        return Err(format!("Invalid character_id: {} (must be positive)", char_id));
    }
    Ok(())
}

/// Validates that a string field is not empty or whitespace-only.
pub fn validate_non_empty(value: &str, field_name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{} must not be empty", field_name));
    }
    Ok(())
}

/// Validates that a gold amount is within a reasonable range.
pub fn validate_gold_amount(amount: i64) -> Result<(), String> {
    if amount < -1_000_000 || amount > 10_000_000 {
        return Err(format!(
            "Gold amount {} is out of valid range (-1,000,000 to 10,000,000)",
            amount
        ));
    }
    Ok(())
}

/// Validates that a roster name meets length constraints.
pub fn validate_roster_name(name: &str) -> Result<(), String> {
    validate_non_empty(name, "roster_name")?;
    if name.len() > 64 {
        return Err("roster_name must be at most 64 characters".to_string());
    }
    Ok(())
}

/// Validates a content_id (raid/task identifier).
pub fn validate_content_id(content_id: &str) -> Result<(), String> {
    validate_non_empty(content_id, "content_id")?;
    if content_id.len() > 128 {
        return Err("content_id must be at most 128 characters".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_character_id_positive() {
        assert!(validate_character_id(1).is_ok());
        assert!(validate_character_id(999).is_ok());
    }

    #[test]
    fn test_validate_character_id_zero_or_negative() {
        assert!(validate_character_id(0).is_err());
        assert!(validate_character_id(-1).is_err());
        assert!(validate_character_id(-100).is_err());
    }

    #[test]
    fn test_validate_non_empty() {
        assert!(validate_non_empty("hello", "field").is_ok());
        assert!(validate_non_empty("a", "field").is_ok());
    }

    #[test]
    fn test_validate_non_empty_rejects_blank() {
        assert!(validate_non_empty("", "field").is_err());
        assert!(validate_non_empty("   ", "field").is_err());
        assert!(validate_non_empty("\t", "field").is_err());
    }

    #[test]
    fn test_validate_gold_amount_valid_range() {
        assert!(validate_gold_amount(0).is_ok());
        assert!(validate_gold_amount(1000).is_ok());
        assert!(validate_gold_amount(-500).is_ok());
        assert!(validate_gold_amount(10_000_000).is_ok());
        assert!(validate_gold_amount(-1_000_000).is_ok());
    }

    #[test]
    fn test_validate_gold_amount_out_of_range() {
        assert!(validate_gold_amount(10_000_001).is_err());
        assert!(validate_gold_amount(-1_000_001).is_err());
    }

    #[test]
    fn test_validate_roster_name() {
        assert!(validate_roster_name("Vaanyar").is_ok());
        assert!(validate_roster_name("x").is_ok());
    }

    #[test]
    fn test_validate_roster_name_rejects_empty() {
        assert!(validate_roster_name("").is_err());
        assert!(validate_roster_name("   ").is_err());
    }

    #[test]
    fn test_validate_roster_name_rejects_too_long() {
        let long = "a".repeat(65);
        assert!(validate_roster_name(&long).is_err());
    }

    #[test]
    fn test_validate_content_id() {
        assert!(validate_content_id("act_3_mordum").is_ok());
        assert!(validate_content_id("chaos").is_ok());
    }

    #[test]
    fn test_validate_content_id_rejects_empty() {
        assert!(validate_content_id("").is_err());
    }

    #[test]
    fn test_validate_content_id_rejects_too_long() {
        let long = "x".repeat(129);
        assert!(validate_content_id(&long).is_err());
    }
}
