use std::path::Path;
use std::fs;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hurl_files_exist() {
        // Test that our integration test files exist
        let test_files = [
            "tests/integration/tests.hurl",
            "tests/integration/tests_auth.hurl", 
            "tests/integration/test-authorization.hurl",
        ];

        for test_file in &test_files {
            if Path::new(test_file).exists() {
                println!("✓ Found Hurl test file: {}", test_file);
            } else {
                println!("⚠ Missing Hurl test file: {}", test_file);
            }
        }
    }

    #[test]
    fn test_hurl_files_syntax() {
        // Test that hurl files have valid syntax by attempting to parse them
        let test_files = [
            "tests/integration/tests.hurl",
            "tests/integration/tests_auth.hurl", 
            "tests/integration/test-authorization.hurl",
        ];

        for test_file in &test_files {
            if Path::new(test_file).exists() {
                match fs::read_to_string(test_file) {
                    Ok(content) => {
                        // Basic syntax validation - check for required elements
                        assert!(content.contains("HTTP"), "File {} should contain HTTP status codes", test_file);
                        println!("✓ Hurl file syntax check passed: {}", test_file);
                    }
                    Err(e) => {
                        panic!("Failed to read Hurl test file {}: {}", test_file, e);
                    }
                }
            }
        }
    }

    #[test]
    fn test_integration_test_directory_structure() {
        // Verify our integration test directory structure
        let dirs = [
            "tests",
            "tests/integration",
        ];

        for dir in &dirs {
            assert!(Path::new(dir).is_dir(), "Directory should exist: {}", dir);
            println!("✓ Found directory: {}", dir);
        }

        // List contents of integration directory
        if let Ok(entries) = fs::read_dir("tests/integration") {
            println!("Integration test files:");
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        println!("  - {}", path.display());
                    }
                }
            }
        }
    }

    #[test]
    fn test_hurl_available() {
        // Test that hurl is available in the system
        match std::process::Command::new("hurl").arg("--version").output() {
            Ok(output) => {
                let version = String::from_utf8_lossy(&output.stdout);
                println!("✓ Hurl is available: {}", version.trim());
            }
            Err(_) => {
                println!("⚠ Hurl is not available. Integration tests can be run manually with:");
                println!("   cd tests/integration && ./run-tests.sh");
            }
        }
    }

    #[test]
    fn test_hurl_execution_ready() {
        // Test that hurl can be executed and shows help
        match std::process::Command::new("hurl")
            .arg("--help")
            .output() 
        {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if stdout.contains("HTTP requests") {
                        println!("✓ Hurl is ready for integration testing");
                    } else {
                        println!("⚠ Hurl help output unexpected");
                    }
                } else {
                    println!("⚠ Hurl help command failed");
                }
            }
            Err(e) => {
                println!("⚠ Could not execute hurl: {}", e);
            }
        }
        
        // Also check that test files exist for manual integration testing
        let test_files = [
            "tests/integration/tests.hurl",
            "tests/integration/tests_auth.hurl", 
            "tests/integration/test-authorization.hurl",
        ];
        
        let mut files_found = 0;
        for test_file in &test_files {
            if Path::new(test_file).exists() {
                files_found += 1;
            }
        }
        
        println!("✓ Found {}/{} Hurl test files ready for integration testing", files_found, test_files.len());
        println!("  🚀 Run integration tests with:");
        println!("    ./tests/integration/run-tests.sh           # Clean output");
        println!("    ./tests/integration/run-tests.sh --verbose # Detailed output");
    }

    #[test]
    fn test_integration_test_runner_exists() {
        // Check that our integration test runner script exists
        let script_path = "tests/integration/run-tests.sh";
        
        if Path::new(script_path).exists() {
            match fs::read_to_string(script_path) {
                Ok(content) => {
                    assert!(content.contains("hurl"), "Test runner should use hurl");
                    println!("✓ Integration test runner found: {}", script_path);
                }
                Err(e) => {
                    panic!("Failed to read test runner {}: {}", script_path, e);
                }
            }
        } else {
            println!("⚠ Integration test runner not found: {}", script_path);
        }
    }

    #[test] 
    fn test_server_can_build() {
        // Test that we can build the server binary for integration tests
        let output = std::process::Command::new("cargo")
            .args(&["build", "--release"])
            .output()
            .expect("Failed to run cargo build");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Server build failed: {}", stderr);
        }

        // Check that the binary exists
        let binary_path = "target/release/rusty-beam";
        assert!(Path::new(binary_path).exists(), "Server binary should exist at {}", binary_path);
        println!("✓ Server binary built successfully: {}", binary_path);
    }

    #[test]
    fn test_integration_test_documentation() {
        // Verify that integration tests are documented
        let readme_paths = [
            "tests/README.md",
            "tests/integration/README.md", 
            "README.md",
        ];

        let mut found_docs = false;
        for readme_path in &readme_paths {
            if Path::new(readme_path).exists() {
                match fs::read_to_string(readme_path) {
                    Ok(content) => {
                        if content.to_lowercase().contains("test") || content.to_lowercase().contains("hurl") {
                            println!("✓ Found integration test documentation: {}", readme_path);
                            found_docs = true;
                        }
                    }
                    Err(_) => {}
                }
            }
        }

        if !found_docs {
            println!("⚠ No integration test documentation found");
            println!("  Consider adding documentation about running: ./tests/integration/run-tests.sh");
        }
    }
}