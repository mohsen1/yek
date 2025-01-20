use yek::parse_size_input;

#[test]
fn test_parse_size_input_tokens() {
    assert_eq!(parse_size_input("100K", true).unwrap(), 100_000);
    assert_eq!(parse_size_input("100k", true).unwrap(), 100_000);
    assert_eq!(parse_size_input("0K", true).unwrap(), 0);
    assert_eq!(parse_size_input("1K", true).unwrap(), 1_000);
    assert_eq!(parse_size_input("1k", true).unwrap(), 1_000);

    // Plain numbers
    assert_eq!(parse_size_input("100", true).unwrap(), 100);
    assert_eq!(parse_size_input("1000", true).unwrap(), 1000);
    assert_eq!(parse_size_input("0", true).unwrap(), 0);

    // Invalid cases
    assert!(parse_size_input("K", true).is_err());
    assert!(parse_size_input("-1K", true).is_err());
    assert!(parse_size_input("-100", true).is_err());
    assert!(parse_size_input("100KB", true).is_err());
    assert!(parse_size_input("invalid", true).is_err());
    assert!(parse_size_input("", true).is_err());
    assert!(parse_size_input(" ", true).is_err());
    assert!(parse_size_input("100K100", true).is_err());
    assert!(parse_size_input("100.5K", true).is_err());

    // Whitespace handling
    assert_eq!(parse_size_input(" 100K ", true).unwrap(), 100_000);
    assert_eq!(parse_size_input("\t100k\n", true).unwrap(), 100_000);
    assert_eq!(parse_size_input(" 100 ", true).unwrap(), 100);
}

#[test]
fn test_parse_size_input_bytes() {
    // KB
    assert_eq!(parse_size_input("100KB", false).unwrap(), 102_400);
    assert_eq!(parse_size_input("100kb", false).unwrap(), 102_400);
    assert_eq!(parse_size_input("0KB", false).unwrap(), 0);
    assert_eq!(parse_size_input("1KB", false).unwrap(), 1_024);

    // MB
    assert_eq!(parse_size_input("1MB", false).unwrap(), 1_048_576);
    assert_eq!(parse_size_input("1mb", false).unwrap(), 1_048_576);
    assert_eq!(parse_size_input("0MB", false).unwrap(), 0);

    // GB
    assert_eq!(parse_size_input("1GB", false).unwrap(), 1_073_741_824);
    assert_eq!(parse_size_input("1gb", false).unwrap(), 1_073_741_824);
    assert_eq!(parse_size_input("0GB", false).unwrap(), 0);

    // Plain bytes
    assert_eq!(parse_size_input("1024", false).unwrap(), 1024);
    assert_eq!(parse_size_input("0", false).unwrap(), 0);

    // Invalid cases
    assert!(parse_size_input("invalid", false).is_err());
    assert!(parse_size_input("", false).is_err());
    assert!(parse_size_input(" ", false).is_err());
    assert!(parse_size_input("-1KB", false).is_err());
    assert!(parse_size_input("-1024", false).is_err());
    assert!(parse_size_input("1.5KB", false).is_err());
    assert!(parse_size_input("1K", false).is_err()); // Must be KB
    assert!(parse_size_input("1M", false).is_err()); // Must be MB
    assert!(parse_size_input("1G", false).is_err()); // Must be GB

    // Whitespace handling
    assert_eq!(parse_size_input(" 100KB ", false).unwrap(), 102_400);
    assert_eq!(parse_size_input("\t100kb\n", false).unwrap(), 102_400);
    assert_eq!(parse_size_input(" 1024 ", false).unwrap(), 1024);
}
