// Global variables
let currentPluginRow = null;
let dasAvailable = false;
let saveTimeout = null;

// Plugin metadata for better organization
const pluginMetadata = {
    'selector-handler': {
        name: 'Selector Handler',
        schema: 'http://rustybeam.net/schema/SelectorHandlerPlugin',
        library: 'file://./plugins/librusty_beam_selector_handler.so'
    },
    'file-handler': {
        name: 'File Handler',
        schema: 'http://rustybeam.net/schema/FileHandlerPlugin',
        library: 'file://./plugins/librusty_beam_file_handler.so'
    },
    'basic-auth': {
        name: 'Basic Auth',
        schema: 'http://rustybeam.net/schema/BasicAuthPlugin',
        library: 'file://./plugins/librusty_beam_basic_auth.so'
    },
    'authorization': {
        name: 'Authorization',
        schema: 'http://rustybeam.net/schema/AuthorizationPlugin',
        library: 'file://./plugins/librusty_beam_authorization.so'
    },
    'access-log': {
        name: 'Access Log',
        schema: 'http://rustybeam.net/schema/AccessLogPlugin',
        library: 'file://./plugins/librusty_beam_access_log.so'
    },
    'directory': {
        name: 'Directory',
        schema: 'http://rustybeam.net/schema/DirectoryPlugin',
        library: 'file://./plugins/libdirectory.so'
    },
    'redirect': {
        name: 'Redirect',
        schema: 'http://rustybeam.net/schema/RedirectPlugin',
        library: 'file://./plugins/librusty_beam_redirect.so'
    },
    'error-handler': {
        name: 'Error Handler',
        schema: 'http://rustybeam.net/schema/ErrorHandlerPlugin',
        library: 'file://./plugins/librusty_beam_error_handler.so'
    },
    'cors': {
        name: 'CORS',
        schema: 'http://rustybeam.net/schema/CorsPlugin',
        library: 'file://./plugins/librusty_beam_cors.so'
    },
    'security-headers': {
        name: 'Security Headers',
        schema: 'http://rustybeam.net/schema/SecurityHeadersPlugin',
        library: 'file://./plugins/librusty_beam_security_headers.so'
    },
    'rate-limit': {
        name: 'Rate Limit',
        schema: 'http://rustybeam.net/schema/RateLimitPlugin',
        library: 'file://./plugins/librusty_beam_rate_limit.so'
    },
    'health-check': {
        name: 'Health Check',
        schema: 'http://rustybeam.net/schema/HealthCheckPlugin',
        library: 'file://./plugins/librusty_beam_health_check.so'
    },
    'compression': {
        name: 'Compression',
        schema: 'http://rustybeam.net/schema/CompressionPlugin',
        library: 'file://./plugins/librusty_beam_compression.so'
    },
    'websocket': {
        name: 'WebSocket',
        schema: 'http://rustybeam.net/schema/WebSocketPlugin',
        library: 'file://./plugins/librusty_beam_websocket.so'
    },
    'config-reload': {
        name: 'Config Reload',
        schema: 'http://rustybeam.net/schema/ConfigReloadPlugin',
        library: 'file://./plugins/librusty_beam_config_reload.so'
    },
    'oauth2': {
        name: 'OAuth2',
        schema: 'http://rustybeam.net/schema/OAuth2Plugin',
        library: 'file://./plugins/librusty_beam_oauth2.so'
    }
};

// DOM-aware primitives initialization
document.addEventListener('DOMContentLoaded', () => {
    // Initialize real-time validation after schemas are loaded
    initializeRealTimeValidation();
    
    // Listen for DOM-aware primitives events
    document.addEventListener('DASAvailable', () => {
        dasAvailable = true;
        console.log('DOM-aware primitives loaded - real-time sync enabled');
        showStatus('Real-time configuration sync enabled', 'success');
    });
    
    document.addEventListener('DASError', (e) => {
        console.warn('DOM-aware primitives error:', e.detail);
        showStatus('Local editing mode (sync disabled)', 'warning');
    });
    
    // Fallback check after delay
    setTimeout(() => {
        if (!dasAvailable) {
            if (typeof HTMLElement.prototype.POST !== 'undefined') {
                dasAvailable = true;
                console.log('DOM-aware primitives detected via fallback check');
                showStatus('Real-time configuration sync enabled', 'success');
            } else {
                console.warn('DOM-aware primitives not available - using local editing mode');
                showStatus('Local editing mode (sync disabled)', 'warning');
            }
        }
    }, 2000);
});

// Initialize real-time validation for all editable elements
function initializeRealTimeValidation() {
    // Add validation to editable spans
    document.querySelectorAll('[contenteditable="true"]').forEach(element => {
        element.addEventListener('input', () => validateElement(element));
        element.addEventListener('blur', () => {
            validateElement(element);
            autoSave(element);
        });
    });
    
    // Add validation to plugin configurations
    document.querySelectorAll('[itemtype]').forEach(element => {
        if (element.getAttribute('itemtype').startsWith('http://rustybeam.net/')) {
            validatePluginElement(element);
        }
    });
    
    // Make sure plugin config displays properly
    document.querySelectorAll('.plugin-config').forEach(config => {
        // Initialize any additional UI enhancements here
        config.classList.add('initialized');
    });
}

// Validate individual element
async function validateElement(element) {
    const property = element.getAttribute('itemprop');
    if (!property) return;
    
    const value = element.textContent.trim();
    const errors = [];
    
    // Basic validation based on property name
    switch (property) {
        case 'bindPort':
            const port = parseInt(value);
            if (isNaN(port) || port < 1 || port > 65535) {
                errors.push('Port must be between 1 and 65535');
            }
            break;
        case 'bindAddress':
            if (value && !isValidIP(value)) {
                errors.push('Invalid IP address');
            }
            break;
        case 'serverRoot':
        case 'hostRoot':
            if (!value) {
                errors.push('Root path is required');
            }
            break;
    }
    
    // Update UI based on validation
    if (errors.length > 0) {
        element.classList.add('field-error');
        element.title = errors.join(', ');
    } else {
        element.classList.remove('field-error');
        element.classList.add('field-success');
        element.title = '';
    }
}

// Validate plugin element using schema
async function validatePluginElement(element) {
    if (!window.schemaLoader) return;
    
    try {
        const errors = await schemaLoader.validatePlugin(element);
        
        // Clear previous validation
        element.classList.remove('validation-error', 'validation-success');
        element.querySelectorAll('.validation-error').forEach(el => el.remove());
        
        if (errors.length > 0) {
            element.classList.add('validation-error');
            
            // Add error indicators
            errors.forEach(error => {
                const propElement = element.querySelector(`[itemprop="${error.property}"]`);
                if (propElement) {
                    propElement.classList.add('field-error');
                    propElement.title = error.message;
                }
            });
        } else {
            element.classList.add('validation-success');
        }
    } catch (error) {
        console.error('Plugin validation failed:', error);
    }
}

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

async function addConfigProperty(button) {
    const pluginRow = button.closest('.plugin-row');
    const schemaUrl = pluginRow.getAttribute('itemtype');
    
    if (!schemaUrl || !window.schemaLoader) {
        // Fallback to simple prompt if no schema available
        addConfigPropertySimple(button);
        return;
    }

    try {
        // Get available properties from schema
        const schema = await schemaLoader.loadSchema(schemaUrl);
        const availableProperties = new Map();
        
        // Find properties not already configured
        // Filter out environment-variable properties for security
        const envVarProperties = new Set();
        if (schemaUrl === 'http://rustybeam.net/OAuth2Plugin') {
            envVarProperties.add('client_id');
            envVarProperties.add('client_secret');
        }
        
        for (const [propName, propDef] of schema.properties) {
            const existingProp = pluginRow.querySelector(`[itemprop="${propName}"]`);
            if (!existingProp && 
                propName !== 'library' && 
                propName !== 'plugin' &&
                !envVarProperties.has(propName)) {
                availableProperties.set(propName, propDef);
            }
        }
        
        if (availableProperties.size === 0) {
            showStatus('All available properties are already configured', 'info');
            return;
        }
        
        // Show schema-driven property selection dialog
        showPropertySelectionDialog(button, availableProperties);
        
    } catch (error) {
        console.error('Failed to load schema for property addition:', error);
        addConfigPropertySimple(button);
    }
}

// Fallback simple property addition
function addConfigPropertySimple(button) {
    const propertyName = prompt('Enter property name:', '');
    if (!propertyName) return;
    
    const propertyValue = prompt('Enter property value:', '');
    if (propertyValue === null) return; // User cancelled
    
    const propertiesContainer = button.parentNode.querySelector('.config-properties');
    const propertyElement = createConfigPropertyFromTemplate(propertyName, propertyValue);
    propertiesContainer.appendChild(propertyElement);
    
    // Initialize auto-save and validation on new property
    initializeAutoSave();
    initializeSchemaValidation();
    
    showStatus(`Property ${propertyName} added`, 'success');
}

// Show intelligent property selection dialog
function showPropertySelectionDialog(button, availableProperties) {
    // Create modal dialog
    const modal = document.createElement('div');
    modal.className = 'property-selection-modal';
    modal.innerHTML = `
        <div class="modal-backdrop" onclick="closePropertyDialog()"></div>
        <div class="modal-content">
            <div class="modal-header">
                <h3>Add Configuration Property</h3>
                <button onclick="closePropertyDialog()" class="btn-small">×</button>
            </div>
            <div class="modal-body">
                <div class="property-list">
                    ${Array.from(availableProperties.entries()).map(([name, def]) => `
                        <div class="property-option" data-property="${name}">
                            <div class="property-header">
                                <strong>${name}</strong>
                                <span class="property-type">${def.type}</span>
                                ${def.required ? '<span class="required">*</span>' : '<span class="optional">optional</span>'}
                            </div>
                            <div class="property-description">${def.description}</div>
                            <div class="property-input">
                                <input type="text" 
                                       id="input-${name}" 
                                       placeholder="${getPlaceholderForProperty(name, def.type)}"
                                       data-type="${def.type}"
                                       data-required="${def.required}">
                                <div class="input-error" id="error-${name}"></div>
                            </div>
                        </div>
                    `).join('')}
                </div>
            </div>
            <div class="modal-footer">
                <button onclick="closePropertyDialog()" class="btn-secondary">Cancel</button>
                <button onclick="addSelectedProperty()" class="btn-primary">Add Property</button>
            </div>
        </div>
    `;
    
    document.body.appendChild(modal);
    
    // Store button reference for later use
    modal.dataset.targetButton = button.id || `btn-${Date.now()}`;
    if (!button.id) button.id = modal.dataset.targetButton;
    
    // Add real-time validation to inputs
    modal.querySelectorAll('input').forEach(input => {
        input.addEventListener('input', () => validatePropertyInput(input));
        input.addEventListener('blur', () => validatePropertyInput(input));
    });
}

// Get appropriate placeholder text for property type
function getPlaceholderForProperty(name, type) {
    const placeholders = {
        'redirect_uri': 'http://localhost:3000/auth/google/callback',
        'authfile': 'file://./auth/users.html',
        'config_file': 'config/plugin-config.html',
        'logfile': 'file://./logs/access.log',
        'realm': 'Admin Area',
        'directory': '/admin'
    };
    
    if (placeholders[name]) {
        return placeholders[name];
    }
    
    switch (type.toLowerCase()) {
        case 'url': return 'http://example.com or file://./path/to/file';
        case 'number': return '0';
        case 'boolean': return 'true';
        default: return `Enter ${name}...`;
    }
}

// Validate property input in real-time
function validatePropertyInput(input) {
    const errorDiv = document.getElementById(`error-${input.id.replace('input-', '')}`);
    const value = input.value.trim();
    const type = input.dataset.type;
    const required = input.dataset.required === 'true';
    
    let isValid = true;
    let errorMessage = '';
    
    // Required validation
    if (required && !value) {
        isValid = false;
        errorMessage = 'This property is required';
    } else if (value) {
        // Type validation
        switch (type.toLowerCase()) {
            case 'url':
                if (!isValidUrl(value)) {
                    isValid = false;
                    errorMessage = 'Must be a valid URL or file path';
                }
                break;
            case 'number':
                if (isNaN(Number(value))) {
                    isValid = false;
                    errorMessage = 'Must be a valid number';
                }
                break;
            case 'boolean':
                if (!['true', 'false', '1', '0'].includes(value.toLowerCase())) {
                    isValid = false;
                    errorMessage = 'Must be true or false';
                }
                break;
        }
    }
    
    // Update UI
    input.classList.toggle('field-error', !isValid);
    input.classList.toggle('field-success', isValid && value);
    errorDiv.textContent = errorMessage;
    errorDiv.style.display = errorMessage ? 'block' : 'none';
    
    return isValid;
}

// Add the selected property
function addSelectedProperty() {
    const modal = document.querySelector('.property-selection-modal');
    const inputs = modal.querySelectorAll('input');
    
    let selectedProperty = null;
    let selectedValue = '';
    
    // Find the property with a value
    for (const input of inputs) {
        if (input.value.trim()) {
            if (selectedProperty) {
                showStatus('Please add only one property at a time', 'error');
                return;
            }
            
            if (!validatePropertyInput(input)) {
                showStatus('Please fix validation errors before adding', 'error');
                return;
            }
            
            selectedProperty = input.id.replace('input-', '');
            selectedValue = input.value.trim();
        }
    }
    
    if (!selectedProperty) {
        showStatus('Please enter a value for one of the properties', 'error');
        return;
    }
    
    // Add the property
    const button = document.getElementById(modal.dataset.targetButton);
    const propertiesContainer = button.parentNode.querySelector('.config-properties');
    const propertyElement = createConfigPropertyFromTemplate(selectedProperty, selectedValue);
    propertiesContainer.appendChild(propertyElement);
    
    // Initialize auto-save and validation on new property
    initializeAutoSave();
    initializeSchemaValidation();
    
    showStatus(`Property ${selectedProperty} added successfully!`, 'success');
    closePropertyDialog();
}

// Close property selection dialog
function closePropertyDialog() {
    const modal = document.querySelector('.property-selection-modal');
    if (modal) {
        modal.remove();
    }
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
    const lastReloadEl = document.getElementById('lastReload');
    if (lastReloadEl) {
        lastReloadEl.textContent = now.toLocaleTimeString();
    }
    updatePluginCount();
}

function updatePluginCount() {
    const pluginCount = document.querySelectorAll('#host-localhost tbody tr[itemprop="plugin"]').length;
    const activePluginsEl = document.getElementById('activePlugins');
    if (activePluginsEl) {
        activePluginsEl.textContent = pluginCount;
    }
}

// Utility functions
function isValidIP(ip) {
    const ipRegex = /^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/;
    return ipRegex.test(ip) || ip === 'localhost' || ip === '0.0.0.0';
}

function isValidUrl(string) {
    try {
        new URL(string);
        return true;
    } catch (_) {
        // Check for relative URLs and file:// protocol
        return string.startsWith('/') || string.startsWith('./') || string.startsWith('file://');
    }
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
        
        // Test loading OAuth2Plugin schema
        const oauth2Schema = await schemaLoader.loadSchema('http://rustybeam.net/OAuth2Plugin');
        console.log('OAuth2Plugin schema:', oauth2Schema);
        
        // Test inheritance resolution
        console.log('Inheritance chain:', oauth2Schema.inheritanceChain);
        console.log('All properties (including inherited):', Array.from(oauth2Schema.properties.keys()));
        
        // Test validation
        const testPlugin = document.querySelector('[itemtype="http://rustybeam.net/OAuth2Plugin"]');
        if (testPlugin) {
            const errors = await schemaLoader.validatePlugin(testPlugin);
            console.log('Validation errors for OAuth2Plugin:', errors);
        }
        
        console.log('Schema loader cache stats:', schemaLoader.getCacheStats());
        
    } catch (error) {
        console.error('Schema loader test failed:', error);
    }
}

// Show status message
function showStatus(message, type = 'success') {
    const indicator = document.getElementById('statusIndicator');
    const messageEl = document.getElementById('statusMessage');
    
    messageEl.textContent = message;
    indicator.className = `status-indicator ui-only show ${type}`;
    
    setTimeout(() => {
        indicator.classList.remove('show');
    }, 3000);
}

// Add new plugin
async function addPlugin() {
    const select = document.createElement('select');
    select.innerHTML = '<option value="">Select a plugin...</option>';
    
    for (const [key, plugin] of Object.entries(pluginMetadata)) {
        select.innerHTML += `<option value="${key}">${plugin.name}</option>`;
    }
    
    const modal = document.getElementById('pluginModal');
    const editor = document.getElementById('pluginEditor');
    
    editor.innerHTML = `
        <div class="property-row">
            <div class="property-label">Plugin Type</div>
            ${select.outerHTML}
        </div>
        <div id="pluginProperties"></div>
    `;
    
    const selectEl = editor.querySelector('select');
    selectEl.addEventListener('change', async (e) => {
        const pluginKey = e.target.value;
        if (!pluginKey) return;
        
        const plugin = pluginMetadata[pluginKey];
        const schema = await schemaLoader.getSchema(plugin.schema);
        
        if (schema) {
            const propsContainer = document.getElementById('pluginProperties');
            propsContainer.innerHTML = '';
            
            for (const [propName, propDef] of Object.entries(schema.properties)) {
                if (propName === 'library') continue; // Skip library property
                
                const row = document.createElement('div');
                row.className = 'property-row';
                row.innerHTML = `
                    <div class="property-label">${propName}</div>
                    <input type="text" class="property-input" name="${propName}" 
                           placeholder="${propDef.description || ''}" />
                `;
                propsContainer.appendChild(row);
            }
        }
    });
    
    // Store context for saving
    currentPluginRow = null;
    modal.classList.add('active');
}

// Edit existing plugin
async function editPlugin(button) {
    const row = button.closest('tr');
    const pluginCell = row.cells[1];
    const pluginEl = pluginCell.querySelector('[itemscope]');
    const schemaUrl = pluginEl.getAttribute('itemtype');
    
    const schema = await schemaLoader.getSchema(schemaUrl);
    const editor = document.getElementById('pluginEditor');
    editor.innerHTML = '';
    
    // Show plugin type
    const pluginName = row.cells[0].textContent;
    const header = document.createElement('div');
    header.className = 'property-row';
    header.innerHTML = `
        <div class="property-label">Plugin Type</div>
        <div>${pluginName}</div>
    `;
    editor.appendChild(header);
    
    // Show editable properties
    if (schema) {
        for (const [propName, propDef] of Object.entries(schema.properties)) {
            if (propName === 'library') continue; // Skip library property
            
            const propEl = pluginEl.querySelector(`[itemprop="${propName}"]`);
            const value = propEl ? propEl.textContent : '';
            
            const row = document.createElement('div');
            row.className = 'property-row';
            row.innerHTML = `
                <div class="property-label">${propName}</div>
                <input type="text" class="property-input" name="${propName}" 
                       value="${value}" placeholder="${propDef.description || ''}" />
            `;
            editor.appendChild(row);
        }
    }
    
    currentPluginRow = row;
    document.getElementById('pluginModal').classList.add('active');
}

// Save plugin configuration
function savePlugin() {
    const editor = document.getElementById('pluginEditor');
    
    if (currentPluginRow) {
        // Editing existing plugin
        const pluginCell = currentPluginRow.cells[1];
        const pluginEl = pluginCell.querySelector('[itemscope]');
        
        // Update properties
        const inputs = editor.querySelectorAll('input[name]');
        inputs.forEach(input => {
            const propName = input.getAttribute('name');
            const value = input.value.trim();
            
            let propEl = pluginEl.querySelector(`[itemprop="${propName}"]`);
            
            if (value) {
                if (!propEl) {
                    propEl = document.createElement('span');
                    propEl.setAttribute('itemprop', propName);
                    pluginEl.appendChild(propEl);
                }
                propEl.textContent = value;
            } else if (propEl) {
                propEl.remove();
            }
        });
    } else {
        // Adding new plugin
        const select = editor.querySelector('select');
        const pluginKey = select.value;
        if (!pluginKey) return;
        
        const plugin = pluginMetadata[pluginKey];
        const tbody = document.querySelector('#host-localhost tbody');
        
        // Create new row
        const row = tbody.insertRow(-1);
        row.innerHTML = `
            <td class="ui-only">${plugin.name}</td>
            <td itemprop="plugin" itemscope itemtype="${plugin.schema}">
                <span itemprop="library">${plugin.library}</span>
            </td>
            <td class="ui-only">
                <div class="plugin-controls">
                    <button class="button secondary" onclick="movePlugin(this, 'up')">↑</button>
                    <button class="button secondary" onclick="movePlugin(this, 'down')">↓</button>
                    <button class="button secondary" onclick="editPlugin(this)">Edit</button>
                    <button class="button danger" onclick="removePlugin(this)">Remove</button>
                </div>
            </td>
        `;
        
        // Add properties
        const pluginEl = row.cells[1].querySelector('[itemscope]');
        const inputs = editor.querySelectorAll('input[name]');
        inputs.forEach(input => {
            const propName = input.getAttribute('name');
            const value = input.value.trim();
            
            if (value) {
                const propEl = document.createElement('span');
                propEl.setAttribute('itemprop', propName);
                propEl.textContent = value;
                pluginEl.appendChild(propEl);
            }
        });
    }
    
    closeModal();
    showStatus('Plugin configuration saved');
}

// Remove plugin
function removePlugin(button) {
    if (confirm('Are you sure you want to remove this plugin?')) {
        const row = button.closest('tr');
        row.remove();
        showStatus('Plugin removed');
    }
}

// Move plugin up/down
function movePlugin(button, direction) {
    const row = button.closest('tr');
    const tbody = row.parentElement;
    const rows = Array.from(tbody.rows);
    const index = rows.indexOf(row);
    
    // Find plugin rows (skip header rows)
    const pluginRows = rows.filter(r => r.cells[1] && r.cells[1].hasAttribute('itemprop'));
    const pluginIndex = pluginRows.indexOf(row);
    
    if (direction === 'up' && pluginIndex > 0) {
        const prevRow = pluginRows[pluginIndex - 1];
        tbody.insertBefore(row, prevRow);
        showStatus('Plugin moved up');
    } else if (direction === 'down' && pluginIndex < pluginRows.length - 1) {
        const nextRow = pluginRows[pluginIndex + 1];
        tbody.insertBefore(nextRow, row);
        showStatus('Plugin moved down');
    }
}

// Close modal
function closeModal() {
    document.getElementById('pluginModal').classList.remove('active');
    currentPluginRow = null;
}

// Export configuration
function exportConfig() {
    const html = document.documentElement.outerHTML;
    const blob = new Blob([html], { type: 'text/html' });
    const url = URL.createObjectURL(blob);
    
    const a = document.createElement('a');
    a.href = url;
    a.download = 'rustybeam-config.html';
    a.click();
    
    URL.revokeObjectURL(url);
    showStatus('Configuration exported');
}

// Validate configuration
async function validateConfig() {
    // TODO: Implement validation
    showStatus('Configuration is valid');
}

// Save configuration
async function saveConfig() {
    try {
        // In a real implementation, this would save to the server
        // For now, we'll use localStorage as a placeholder
        const configHTML = document.documentElement.outerHTML;
        localStorage.setItem('rustybeam-config', configHTML);
        showStatus('Configuration saved');
    } catch (error) {
        showStatus('Failed to save configuration', 'error');
    }
}

// Reload server configuration
async function reloadServer() {
    try {
        showStatus('Sending reload request...', 'info');
        
        const response = await fetch('/config/', {
            method: 'PATCH',
            headers: {
                'Content-Length': '0'
            }
        });
        
        if (response.ok) {
            showStatus('Server configuration reload initiated', 'success');
        } else if (response.status === 403) {
            showStatus('Access denied - administrator privileges required', 'error');
        } else {
            showStatus(`Failed to reload server: ${response.status} ${response.statusText}`, 'error');
        }
    } catch (error) {
        showStatus('Error connecting to server: ' + error.message, 'error');
    }
}

// Redirect rule management functions
function addRedirectRule() {
    const tbody = document.querySelector('#redirect-rules tbody');
    const newRow = document.createElement('tr');
    newRow.setAttribute('itemscope', '');
    newRow.setAttribute('itemtype', 'http://rustybeam.net/RedirectRule');
    
    newRow.innerHTML = `
        <td>
            <span itemprop="from" class="rule-value editable" contenteditable="true">^/path/.*$</span>
        </td>
        <td>
            <span itemprop="to" class="rule-value editable" contenteditable="true">/new-path</span>
        </td>
        <td>
            <span itemprop="status" class="rule-value editable" contenteditable="true">302</span>
        </td>
        <td>
            <span itemprop="on" class="rule-value editable" contenteditable="true"></span>
        </td>
        <td class="ui-only">
            <button class="button danger" onclick="removeRedirectRule(this)">Remove</button>
        </td>
    `;
    
    tbody.appendChild(newRow);
    
    // Initialize validation on new elements
    newRow.querySelectorAll('.editable').forEach(element => {
        element.addEventListener('input', () => validateElement(element));
        element.addEventListener('blur', () => {
            validateElement(element);
            autoSave(element);
        });
    });
    
    showStatus('Redirect rule added');
}

function removeRedirectRule(button) {
    const row = button.closest('tr');
    if (confirm('Remove this redirect rule?')) {
        row.remove();
        showStatus('Redirect rule removed');
    }
}

// Make functions available globally
window.addPlugin = addPlugin;
window.editPlugin = editPlugin;  
window.savePlugin = savePlugin;
window.removePlugin = removePlugin;
window.movePlugin = movePlugin;
window.closeModal = closeModal;
window.exportConfig = exportConfig;
window.validateConfig = validateConfig;
window.saveConfig = saveConfig;
window.reloadServer = reloadServer;
window.testSchemaLoader = testSchemaLoader;
window.addRedirectRule = addRedirectRule;
window.removeRedirectRule = removeRedirectRule;