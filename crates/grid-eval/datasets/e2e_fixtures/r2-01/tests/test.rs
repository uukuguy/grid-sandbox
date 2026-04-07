use r2_01::Config;

#[test]
fn test_access_fields() {
    let cfg = Config::new("test", 42);
    assert_eq!(cfg.name, "test");
    assert_eq!(cfg.value, 42);
}
