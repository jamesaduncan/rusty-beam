// Global variables
let dasAvailable = false;
let saveTimeout = null;

// Plugin metadata for better organization
// TODO: This should come from a Plugin Metadata API (see PROGRESS.md Phase 2)
const pluginMetadata = {
    'librusty_beam_access_log.so': { 
        name: 'Access Log', 
        type: 'utility', 
        category: 'logging',
        schema: 'http://rustybeam.net/AccessLogPlugin'
    },
    'librusty_beam_authorization.so': { 
        name: 'Authorization', 
        type: 'auth', 
        category: 'security',
        schema: 'http://rustybeam.net/AuthorizationPlugin'
    },
    'librusty_beam_basic_auth.so': { 
        name: 'Basic Auth', 
        type: 'auth', 
        category: 'security',
        schema: 'http://rustybeam.net/BasicAuthPlugin'
    },
    'librusty_beam_compression.so': { 
        name: 'Compression', 
        type: 'utility', 
        category: 'performance',
        schema: 'http://rustybeam.net/CompressionPlugin'
    },
    'librusty_beam_cors.so': { 
        name: 'CORS', 
        type: 'utility', 
        category: 'security',
        schema: 'http://rustybeam.net/CorsPlugin'
    },
    'librusty_beam_error_handler.so': { 
        name: 'Error Handler', 
        type: 'handler', 
        category: 'core',
        schema: 'http://rustybeam.net/ErrorHandlerPlugin'
    },
    'librusty_beam_file_handler.so': { 
        name: 'File Handler', 
        type: 'handler', 
        category: 'core',
        schema: 'http://rustybeam.net/FileHandlerPlugin'
    },
    'librusty_beam_google_oauth2.so': { 
        name: 'Google OAuth2', 
        type: 'oauth', 
        category: 'security',
        schema: 'http://rustybeam.net/GoogleOAuth2Plugin'
    },
    'librusty_beam_health_check.so': { 
        name: 'Health Check', 
        type: 'utility', 
        category: 'monitoring',
        schema: 'http://rustybeam.net/HealthCheckPlugin'
    },
    'librusty_beam_rate_limit.so': { 
        name: 'Rate Limit', 
        type: 'utility', 
        category: 'security',
        schema: 'http://rustybeam.net/RateLimitPlugin'
    },
    'librusty_beam_redirect.so': { 
        name: 'Redirect', 
        type: 'handler', 
        category: 'core',
        schema: 'http://rustybeam.net/RedirectPlugin'
    },
    'librusty_beam_security_headers.so': { 
        name: 'Security Headers', 
        type: 'utility', 
        category: 'security',
        schema: 'http://rustybeam.net/SecurityHeadersPlugin'
    },
    'librusty_beam_selector_handler.so': { 
        name: 'Selector Handler', 
        type: 'handler', 
        category: 'core',
        schema: 'http://rustybeam.net/SelectorHandlerPlugin'
    },
    'librusty_beam_websocket.so': { 
        name: 'WebSocket', 
        type: 'handler', 
        category: 'core',
        schema: 'http://rustybeam.net/WebSocketPlugin'
    }
};

// Auto-save functionality
async function autoSave(element) {
    // Clear any pending save
    if (saveTimeout) {
        clearTimeout(saveTimeout);
    }
    
    // Show saving indicator
    showStatus('Saving configuration...', 'info');
    
    // Debounce saves by 500ms
    saveTimeout = setTimeout(async () => {
        try {
            if (!dasAvailable) {
                showStatus('Changes saved locally (DOM-aware server not available)', 'warning');
                return;
            }
            
            // Get the parent element (tr or the server config table)
            const parent = element.closest('tr') || element.closest('table');
            if (!parent) {
                throw new Error('Could not find parent element');
            }
            
            // Use PUT to update the element with current content
            const response = await parent.PUT(parent.outerHTML);
            
            if (response.ok) {
                showStatus('Configuration saved successfully!', 'success');
                updateConfigStatus();
            } else {
                throw new Error(`Failed to save: ${response.status} ${response.statusText}`);
            }
        } catch (error) {
            console.error('Auto-save failed:', error);
            showStatus(`Failed to save: ${error.message}`, 'error');
        }
    }, 500);
}

// Initialize auto-save on all editable elements
function initializeAutoSave() {
    // Add blur event listeners to all editable elements
    document.querySelectorAll('.editable[contenteditable]').forEach(element => {
        element.addEventListener('blur', () => autoSave(element));
        
        // Also save on Enter key (but keep editing)
        element.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault(); // Prevent new line in plaintext-only
                element.blur(); // Trigger save
                // Refocus after a short delay
                setTimeout(() => element.focus(), 100);
            }
        });
    });
}

// Initialize schema validation on plugin elements
function initializeSchemaValidation() {
    // Add real-time validation to plugin rows
    document.querySelectorAll('.plugin-row').forEach(row => {
        const editableElements = row.querySelectorAll('.editable[contenteditable]');
        editableElements.forEach(element => {
            element.addEventListener('input', () => validatePluginRow(row));
            element.addEventListener('blur', () => validatePluginRow(row));
        });
    });
}

// Validate a plugin row against its schema
async function validatePluginRow(pluginRow) {
    if (!window.schemaLoader) {
        console.warn('Schema loader not available');
        return;
    }

    try {
        // Clear previous validation errors
        clearValidationErrors(pluginRow);

        const errors = await schemaLoader.validatePlugin(pluginRow);
        
        if (errors.length > 0) {
            displayValidationErrors(pluginRow, errors);
        } else {
            // Show success state
            pluginRow.classList.remove('validation-error');
            pluginRow.classList.add('validation-success');
        }
    } catch (error) {
        console.error('Validation error:', error);
        showStatus(`Validation failed: ${error.message}`, 'error');
    }
}

// Clear validation error styling and messages
function clearValidationErrors(pluginRow) {
    pluginRow.classList.remove('validation-error', 'validation-success');
    pluginRow.querySelectorAll('.validation-error-message').forEach(el => el.remove());
    pluginRow.querySelectorAll('.editable').forEach(el => {
        el.classList.remove('field-error', 'field-success');
    });
}

// Display validation errors in the UI
function displayValidationErrors(pluginRow, errors) {
    pluginRow.classList.add('validation-error');
    
    errors.forEach(error => {
        const propertyElement = pluginRow.querySelector(`[itemprop="${error.property}"]`);
        if (propertyElement) {
            propertyElement.classList.add('field-error');
            
            // Add error message tooltip
            const errorMsg = document.createElement('div');
            errorMsg.className = 'validation-error-message';
            errorMsg.textContent = error.message;
            propertyElement.parentNode.appendChild(errorMsg);
        }
    });
}

// Wait for DOM-aware primitives to load
document.addEventListener('DOMContentLoaded', () => {
    // Initialize auto-save on existing elements
    initializeAutoSave();
    
    // Initialize schema validation
    initializeSchemaValidation();
    
    // Check if DOM-aware primitives are available
    setTimeout(() => {
        if (typeof HTMLElement.prototype.POST !== 'undefined') {
            dasAvailable = true;
            console.log('DOM-aware primitives loaded successfully');
            updateConfigStatus();
        } else {
            console.warn('DOM-aware primitives not available');
        }
    }, 1000);
});

// Show status message
function showStatus(message, type = 'success') {
    const status = document.getElementById('status') || createStatusElement();
    status.textContent = message;
    status.className = `status-message ${type}`;
    status.style.display = 'block';
    
    // Don't auto-hide info messages (like "Saving...")
    if (type !== 'info') {
        setTimeout(() => {
            status.style.display = 'none';
        }, 3000);
    }
}

function createStatusElement() {
    const status = document.createElement('div');
    status.id = 'status';
    status.className = 'status-message';
    document.body.insertBefore(status, document.body.firstChild);
    return status;
}

// Plugin Management Functions
async function addPlugin() {
    const pluginSelect = document.getElementById('newPluginType');
    const pluginLibrary = pluginSelect.value;
    
    if (!pluginLibrary) {
        showStatus('Please select a plugin type', 'error');
        return;
    }
    
    const metadata = pluginMetadata[pluginLibrary];
    if (!metadata) {
        showStatus('Unknown plugin type', 'error');
        return;
    }
    
    const pluginElement = createPluginRowFromTemplate(metadata.name, `file://./plugins/${pluginLibrary}`, metadata.type, metadata.schema);
    
    try {
        if (dasAvailable) {
            const pluginsTableBody = document.querySelector('#plugins tbody');
            await pluginsTableBody.POST(pluginElement.firstElementChild.outerHTML);
        } else {
            // Fallback: add directly to DOM
            const pluginsTableBody = document.querySelector('#plugins tbody');
            pluginsTableBody.appendChild(pluginElement);
        }
        
        // Clear form
        pluginSelect.selectedIndex = 0;
        
        // Initialize auto-save and validation on new elements
        initializeAutoSave();
        initializeSchemaValidation();
        
        showStatus(`Plugin ${metadata.name} added successfully!`);
        updatePluginCount();
    } catch (error) {
        console.error('Failed to add plugin:', error);
        showStatus('Failed to add plugin', 'error');
    }
}

function createPluginRowFromTemplate(name, library, type, schema) {
    const template = document.getElementById('pluginRowTemplate');
    const clone = template.content.cloneNode(true);
    
    // Set the plugin name
    const nameElement = clone.querySelector('.plugin-name');
    nameElement.textContent = name;
    
    // Set the library path
    const librarySpan = clone.querySelector('[itemprop="library"]');
    librarySpan.textContent = library;
    
    // Set plugin type data attribute for styling
    const row = clone.querySelector('.plugin-row');
    row.setAttribute('data-plugin-type', type);
    
    // Set the schema if provided
    if (schema) {
        row.setAttribute('itemtype', schema);
    }
    
    return clone;
}

async function deletePlugin(button) {
    const row = button.closest('tr');
    const pluginName = row.querySelector('.plugin-name').textContent;
    
    if (!confirm(`Delete plugin ${pluginName}?`)) {
        return;
    }
    
    try {
        if (dasAvailable) {
            await row.DELETE();
        } else {
            row.remove();
        }
        showStatus(`Plugin ${pluginName} deleted successfully!`);
        updatePluginCount();
    } catch (error) {
        console.error('Failed to delete plugin:', error);
        showStatus('Failed to delete plugin', 'error');
    }
}

function movePluginUp(button) {
    const row = button.closest('tr');
    const prevRow = row.previousElementSibling;
    if (prevRow) {
        row.parentNode.insertBefore(row, prevRow);
        showStatus('Plugin moved up', 'info');
    }
}

function movePluginDown(button) {
    const row = button.closest('tr');
    const nextRow = row.nextElementSibling;
    if (nextRow) {
        row.parentNode.insertBefore(nextRow, row);
        showStatus('Plugin moved down', 'info');
    }
}

function addConfigProperty(button) {
    const propertyName = prompt('Enter property name:', '');
    if (!propertyName) return;
    
    const propertyValue = prompt('Enter property value:', '');
    if (propertyValue === null) return; // User cancelled
    
    const propertiesContainer = button.parentNode.querySelector('.config-properties');
    const propertyElement = createConfigPropertyFromTemplate(propertyName, propertyValue);
    propertiesContainer.appendChild(propertyElement);
    
    // Initialize auto-save on new property
    initializeAutoSave();
    
    showStatus(`Property ${propertyName} added`, 'success');
}

function createConfigPropertyFromTemplate(name, value) {
    const template = document.getElementById('configPropertyTemplate');
    const clone = template.content.cloneNode(true);
    
    // Set the property label
    const label = clone.querySelector('.property-label');
    label.textContent = name + ':';
    
    // Set the property value and itemprop
    const valueSpan = clone.querySelector('.editable');
    valueSpan.textContent = value;
    valueSpan.setAttribute('itemprop', name);
    
    return clone;
}

function removeConfigProperty(button) {
    const property = button.closest('.config-property');
    const propertyName = property.querySelector('.property-label').textContent.replace(':', '');
    
    if (confirm(`Remove property ${propertyName}?`)) {
        property.remove();
        showStatus(`Property ${propertyName} removed`, 'success');
    }
}

// Configuration Management Functions
async function reloadConfig() {
    try {
        showStatus('Reloading configuration...', 'info');
        
        // Send SIGHUP signal to reload configuration
        // This would typically be done via a special endpoint
        if (dasAvailable) {
            // For now, just simulate a reload
            await new Promise(resolve => setTimeout(resolve, 1000));
        }
        
        showStatus('Configuration reloaded successfully!', 'success');
        updateConfigStatus();
    } catch (error) {
        console.error('Failed to reload config:', error);
        showStatus('Failed to reload configuration', 'error');
    }
}

async function validateConfig() {
    try {
        showStatus('Validating configuration...', 'info');
        
        let hasErrors = false;
        const allErrors = [];

        // Validate server config
        const serverRoot = document.querySelector('[itemprop="serverRoot"]').textContent;
        const bindAddress = document.querySelector('[itemprop="bindAddress"]').textContent;
        const bindPort = document.querySelector('[itemprop="bindPort"]').textContent;
        
        // Basic server validation
        if (!serverRoot || !bindAddress || !bindPort) {
            allErrors.push('Server configuration incomplete');
            hasErrors = true;
        }
        
        if (bindPort && (isNaN(parseInt(bindPort)) || parseInt(bindPort) < 1 || parseInt(bindPort) > 65535)) {
            allErrors.push('Invalid port number');
            hasErrors = true;
        }
        
        // Validate IP address
        if (bindAddress && !isValidIP(bindAddress)) {
            allErrors.push('Invalid IP address');
            hasErrors = true;
        }
        
        // Validate all plugins using schema validation
        const plugins = document.querySelectorAll('#plugins tbody tr');
        if (plugins.length === 0) {
            allErrors.push('No plugins configured');
            hasErrors = true;
        } else {
            for (const plugin of plugins) {
                if (window.schemaLoader) {
                    const pluginErrors = await schemaLoader.validatePlugin(plugin);
                    if (pluginErrors.length > 0) {
                        const pluginName = plugin.querySelector('.plugin-name')?.textContent || 'Unknown';
                        pluginErrors.forEach(error => {
                            allErrors.push(`${pluginName}: ${error.message}`);
                        });
                        hasErrors = true;
                    }
                }
            }
        }
        
        if (hasErrors) {
            showStatus(`Validation failed: ${allErrors.length} error(s) found`, 'error');
            document.getElementById('configValid').textContent = 'Invalid';
            console.log('Validation errors:', allErrors);
        } else {
            showStatus('Configuration is valid!', 'success');
            document.getElementById('configValid').textContent = 'Valid';
        }
    } catch (error) {
        console.error('Validation failed:', error);
        showStatus(`Validation failed: ${error.message}`, 'error');
        document.getElementById('configValid').textContent = 'Invalid';
    }
}

function exportConfig() {
    try {
        // Get the server configuration
        const serverConfig = document.querySelector('#serverConfig').outerHTML;
        const pluginsConfig = document.querySelector('#plugins').outerHTML;
        
        // Create a complete configuration file
        const configContent = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rusty Beam Server Configuration</title>
    <script type="module" src="https://jamesaduncan.github.io/dom-aware-primitives/index.mjs"></script>
</head>
<body>
    <h1>Rusty Beam Server Configuration</h1>
    
    ${serverConfig}
    
    <div itemscope itemtype="http://rustybeam.net/HostConfig">
        <span itemprop="hostName">localhost</span>
        <span itemprop="hostRoot">./examples/guestbook</span>
        ${pluginsConfig}
    </div>
</body>
</html>`;
        
        // Download the configuration file
        const blob = new Blob([configContent], { type: 'text/html' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `rusty-beam-config-${new Date().toISOString().split('T')[0]}.html`;
        a.click();
        URL.revokeObjectURL(url);
        
        showStatus('Configuration exported successfully!', 'success');
    } catch (error) {
        console.error('Export failed:', error);
        showStatus('Failed to export configuration', 'error');
    }
}

function importConfig() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.html';
    input.onchange = (e) => {
        const file = e.target.files[0];
        if (!file) return;
        
        const reader = new FileReader();
        reader.onload = (e) => {
            try {
                const content = e.target.result;
                // This would typically parse and apply the configuration
                // For now, just show a success message
                showStatus('Configuration import feature coming soon!', 'info');
            } catch (error) {
                console.error('Import failed:', error);
                showStatus('Failed to import configuration', 'error');
            }
        };
        reader.readAsText(file);
    };
    input.click();
}

// Update configuration status display
function updateConfigStatus() {
    const now = new Date();
    document.getElementById('lastReload').textContent = now.toLocaleTimeString();
    updatePluginCount();
}

function updatePluginCount() {
    const pluginCount = document.querySelectorAll('#plugins tbody tr').length;
    document.getElementById('activePlugins').textContent = pluginCount;
}

// Utility functions
function isValidIP(ip) {
    const ipRegex = /^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/;
    return ipRegex.test(ip) || ip === 'localhost' || ip === '0.0.0.0';
}

function escapeHtml(text) {
    const map = {
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;',
        "'": '&#039;'
    };
    return text.replace(/[&<>"']/g, m => map[m]);
}

// Debug function to test schema loading
async function testSchemaLoader() {
    if (!window.schemaLoader) {
        console.error('Schema loader not available');
        return;
    }

    try {
        console.log('Testing schema loader...');
        
        // Test loading GoogleOAuth2Plugin schema
        const oauth2Schema = await schemaLoader.loadSchema('http://rustybeam.net/GoogleOAuth2Plugin');
        console.log('GoogleOAuth2Plugin schema:', oauth2Schema);
        
        // Test inheritance resolution
        console.log('Inheritance chain:', oauth2Schema.inheritanceChain);
        console.log('All properties (including inherited):', Array.from(oauth2Schema.properties.keys()));
        
        // Test validation
        const testPlugin = document.querySelector('[itemtype="http://rustybeam.net/GoogleOAuth2Plugin"]');
        if (testPlugin) {
            const errors = await schemaLoader.validatePlugin(testPlugin);
            console.log('Validation errors for GoogleOAuth2Plugin:', errors);
        }
        
        console.log('Schema loader cache stats:', schemaLoader.getCacheStats());
        
    } catch (error) {
        console.error('Schema loader test failed:', error);
    }
}

// Make functions available globally
window.addPlugin = addPlugin;
window.deletePlugin = deletePlugin;
window.movePluginUp = movePluginUp;
window.movePluginDown = movePluginDown;
window.addConfigProperty = addConfigProperty;
window.removeConfigProperty = removeConfigProperty;
window.reloadConfig = reloadConfig;
window.validateConfig = validateConfig;
window.exportConfig = exportConfig;
window.importConfig = importConfig;
window.testSchemaLoader = testSchemaLoader;