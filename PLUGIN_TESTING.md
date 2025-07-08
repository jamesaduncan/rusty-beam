# Plugin Testing Infrastructure

## Overview

The plugin test infrastructure has been redesigned with an isolated approach where each plugin test:
- Has its own configuration file
- Gets its own host directory (copied from a template)
- Runs in complete isolation from other tests
- Has proper setup and teardown

## Structure

```
tests/plugins/
├── template/               # Template files copied for each test
│   ├── index.html
│   └── foo.html
├── configs/               # Individual plugin configurations
│   ├── file-handler-config.html
│   ├── selector-handler-config.html
│   ├── health-check-config.html
│   └── cors-config.html
├── hosts/                 # Runtime directories (created during tests)
│   └── [plugin-name]/     # Isolated environment for each test
└── test-*-simple.hurl     # Simplified test files
```

## Running Tests

```bash
# Run all plugin tests with isolation
./run_plugin_tests_isolated.sh

# Run with debug output
PLUGIN_TEST_DEBUG=1 ./run_plugin_tests_isolated.sh
```

## Adding a New Plugin Test

1. Create a configuration file in `tests/plugins/configs/[plugin-name]-config.html`
2. Create a test file `tests/plugins/test-[plugin-name]-simple.hurl`
3. The test runner will automatically:
   - Copy the template directory
   - Start the server with your config
   - Run your tests
   - Clean up afterwards

## Example Configuration

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <title>[Plugin Name] Test Configuration</title>
</head>
<body>
    <h1>[Plugin Name] Test Configuration</h1>
    
    <table itemref="host-localhost" itemscope itemtype="http://rustybeam.net/ServerConfig">
        <tbody>
            <tr>
                <td>Server Root</td>
                <td itemprop="serverRoot">tests/plugins/hosts/[plugin-name]</td>
            </tr>
            <tr>
                <td>Address</td>
                <td itemprop="bindAddress">127.0.0.1</td>
            </tr>
            <tr>
                <td>Port</td>
                <td itemprop="bindPort">3000</td>
            </tr>
        </tbody>
    </table>
    
    <table id="host-localhost" itemprop="host" itemscope itemtype="http://rustybeam.net/HostConfig">
        <tbody>
            <tr>
                <td>Host Name</td>
                <td itemprop="hostName">localhost</td>
            </tr>
            <tr>
                <td>Host Root</td>
                <td itemprop="hostRoot">tests/plugins/hosts/[plugin-name]</td>
            </tr>
            <tr>
                <td>Plugin Pipeline</td>
                <td>
                    <ol itemprop="plugin" itemscope itemtype="http://rustybeam.net/Plugin">
                        <li itemprop="plugin" itemscope itemtype="http://rustybeam.net/Plugin">
                            <span itemprop="library">file://./plugins/[plugin-name].so</span>
                        </li>
                        <li itemprop="plugin" itemscope itemtype="http://rustybeam.net/Plugin">
                            <span itemprop="library">file://./plugins/file-handler-v2.so</span>
                        </li>
                    </ol>
                </td>
            </tr>
        </tbody>
    </table>
</body>
</html>
```

## Benefits

1. **Isolation**: Each test runs in its own environment
2. **Repeatability**: Clean state for every test run
3. **Maintainability**: Easy to add new tests
4. **Debugging**: Each plugin has its own log file
5. **Flexibility**: Can test any plugin combination

## Current Test Status

- ✅ file-handler
- ✅ selector-handler  
- ✅ health-check
- ✅ cors
- ⏳ compression (config needed)
- ⏳ security-headers (config needed)
- ⏳ rate-limit (config needed)
- ⏳ redirect (config needed)
- ⏳ error-handler (config needed)
- ⏳ access-log (config needed)
- ⏳ basic-auth (config needed)
- ⏳ authorization (config needed)