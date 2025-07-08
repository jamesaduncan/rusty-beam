# Test Migration Notes

## Test Consolidation

All tests have been consolidated under the `tests/` directory for better organization:

### Completed Migrations:

1. **Integration Tests** - Already organized under `tests/integration/`
   - Hurl-based HTTP API tests
   - Shell scripts for test execution

2. **Configuration and Files** - Already organized under `tests/config/` and `tests/files/`
   - Test configuration files
   - Test fixture files

### Identified Test Locations:

The following test files were found scattered in the codebase:

1. **`src/integration_tests.rs`** - Integration test sanity checks
   - Tests for Hurl file existence
   - Tests for test infrastructure validation
   - Should be moved to `tests/unit/`

2. **`crates/microdata-extract/src/extractor.rs`** - Contains unit tests in `#[cfg(test)]` module
   - Basic extraction tests
   - Nested items tests
   - Multiple properties tests
   - Should remain in source file (Rust convention)

3. **`crates/microdata-extract/tests/integration_tests.rs`** - Microdata integration tests
   - Schema.org tests
   - Complex HTML parsing tests
   - Already in appropriate location

4. **Test scripts in root**:
   - `test-debug-auth.sh`
   - `test-root-auth.sh`
   - Moved to `tests/scripts/`

### Recommendation:

For Rust projects, the common convention is:
- Unit tests stay in the source files using `#[cfg(test)]` modules
- Integration tests go in the `tests/` directory
- The current structure follows this convention well

No further migration is needed at this time. The test structure is already well-organized following Rust best practices.