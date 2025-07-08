#[test]
fn test_plugins_built() {
    // Just verify that the required plugins exist
    let plugins = vec![
        "./plugins/selector-handler.so",
        "./plugins/file-handler.so",
        "./plugins/file-handler-v2.so",
    ];

    for plugin in plugins {
        assert!(
            std::path::Path::new(plugin).exists(),
            "Plugin {} not found. Run ./build-plugins.sh first",
            plugin
        );
    }

    println!("✅ All required plugins are built");
}

#[test]
fn test_config_exists() {
    assert!(
        std::path::Path::new("tests/config/test-config.html").exists(),
        "Test config file not found"
    );
    println!("✅ Test config file exists");
}

#[test]
fn test_setup_scripts_exist() {
    let scripts = vec![
        "./tests/integration/setup-tests.sh",
        "./tests/integration/teardown-tests.sh",
        "./run_hurl_tests.sh",
    ];

    for script in scripts {
        assert!(
            std::path::Path::new(script).exists(),
            "Script {} not found",
            script
        );
    }

    println!("✅ All test scripts exist");
}
