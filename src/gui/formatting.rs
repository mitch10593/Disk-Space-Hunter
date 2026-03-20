pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let exp = (bytes as f64).log(1024.0).floor() as usize;
    let exp = exp.min(UNITS.len() - 1);
    let value = bytes as f64 / 1024_f64.powi(exp as i32);
    if exp == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", value, UNITS[exp])
    }
}

pub fn format_count(n: u32) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── format_size ──────────────────────────────────────────

    #[test]
    fn given_zero_bytes_when_format_size_then_returns_zero_b() {
        // GIVEN
        let bytes = 0;
        // WHEN
        let result = format_size(bytes);
        // THEN
        assert_eq!(result, "0 B");
    }

    #[test]
    fn given_one_byte_when_format_size_then_returns_one_b() {
        // GIVEN
        let bytes = 1;
        // WHEN
        let result = format_size(bytes);
        // THEN
        assert_eq!(result, "1 B");
    }

    #[test]
    fn given_1023_bytes_when_format_size_then_returns_bytes_unit() {
        // GIVEN
        let bytes = 1023;
        // WHEN
        let result = format_size(bytes);
        // THEN
        assert_eq!(result, "1023 B");
    }

    #[test]
    fn given_1024_bytes_when_format_size_then_returns_kb() {
        // GIVEN
        let bytes = 1024;
        // WHEN
        let result = format_size(bytes);
        // THEN
        assert_eq!(result, "1.0 KB");
    }

    #[test]
    fn given_fractional_kb_when_format_size_then_returns_one_decimal() {
        // GIVEN
        let bytes = 1536;
        // WHEN
        let result = format_size(bytes);
        // THEN
        assert_eq!(result, "1.5 KB");
    }

    #[test]
    fn given_one_mb_when_format_size_then_returns_mb() {
        // GIVEN
        let bytes = 1024 * 1024;
        // WHEN
        let result = format_size(bytes);
        // THEN
        assert_eq!(result, "1.0 MB");
    }

    #[test]
    fn given_one_gb_when_format_size_then_returns_gb() {
        // GIVEN
        let bytes = 1024 * 1024 * 1024;
        // WHEN
        let result = format_size(bytes);
        // THEN
        assert_eq!(result, "1.0 GB");
    }

    #[test]
    fn given_one_tb_when_format_size_then_returns_tb() {
        // GIVEN
        let bytes = 1024_u64 * 1024 * 1024 * 1024;
        // WHEN
        let result = format_size(bytes);
        // THEN
        assert_eq!(result, "1.0 TB");
    }

    #[test]
    fn given_max_unit_exceeded_when_format_size_then_clamps_to_tb() {
        // GIVEN — 1024 TB = 1 PB, but TB is the largest unit
        let bytes = 1024_u64 * 1024 * 1024 * 1024 * 1024;
        // WHEN
        let result = format_size(bytes);
        // THEN
        assert_eq!(result, "1024.0 TB");
    }

    // ── format_count ─────────────────────────────────────────

    #[test]
    fn given_zero_when_format_count_then_returns_zero() {
        // GIVEN
        let n = 0;
        // WHEN
        let result = format_count(n);
        // THEN
        assert_eq!(result, "0");
    }

    #[test]
    fn given_three_digits_when_format_count_then_no_comma() {
        // GIVEN
        let n = 999;
        // WHEN
        let result = format_count(n);
        // THEN
        assert_eq!(result, "999");
    }

    #[test]
    fn given_four_digits_when_format_count_then_one_comma() {
        // GIVEN
        let n = 1000;
        // WHEN
        let result = format_count(n);
        // THEN
        assert_eq!(result, "1,000");
    }

    #[test]
    fn given_seven_digits_when_format_count_then_two_commas() {
        // GIVEN
        let n = 1_234_567;
        // WHEN
        let result = format_count(n);
        // THEN
        assert_eq!(result, "1,234,567");
    }

    #[test]
    fn given_max_u32_when_format_count_then_correct_commas() {
        // GIVEN
        let n = u32::MAX;
        // WHEN
        let result = format_count(n);
        // THEN
        assert_eq!(result, "4,294,967,295");
    }
}
