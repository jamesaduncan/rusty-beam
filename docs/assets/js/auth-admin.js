// Global variables
let dasAvailable = false;
let saveTimeout = null;

// Auto-save functionality
async function autoSave(element) {
    // Clear any pending save
    if (saveTimeout) {
        clearTimeout(saveTimeout);
    }
    
    // Show saving indicator
    showStatus('Saving...', 'info');
    
    // Debounce saves by 500ms
    saveTimeout = setTimeout(async () => {
        try {
            if (!dasAvailable) {
                showStatus('Changes saved locally (DOM-aware server not available)', 'warning');
                return;
            }
            
            // Get the parent row (tr element)
            const row = element.closest('tr');
            if (!row) {
                throw new Error('Could not find parent row');
            }
            
            // Use PUT to update the row with current content
            const response = await row.PUT(row.outerHTML);
            
            if (response.ok) {
                showStatus('Changes saved successfully!', 'success');
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
    
    // Add change listeners to select elements
    document.querySelectorAll('.action-select').forEach(select => {
        select.addEventListener('change', () => autoSave(select));
    });
}

// Wait for DOM-aware primitives to load
document.addEventListener('DOMContentLoaded', () => {
    // Initialize auto-save on existing elements
    initializeAutoSave();
    
    // Check if DOM-aware primitives are available
    setTimeout(() => {
        if (typeof HTMLElement.prototype.POST !== 'undefined') {
            dasAvailable = true;
            console.log('DOM-aware primitives loaded successfully');
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

// User Management Functions
async function addUser() {
    const emailInput = document.getElementById('newUserEmail');
    const roleSelect = document.getElementById('newUserRole');
    
    const email = emailInput.value.trim();
    const selectedRoles = Array.from(roleSelect.selectedOptions).map(opt => opt.value);
    
    if (!email || selectedRoles.length === 0) {
        showStatus('Please enter an email and select at least one role', 'error');
        return;
    }
    
    if (!isValidEmail(email)) {
        showStatus('Please enter a valid email address', 'error');
        return;
    }
    
    const userElement = createUserRowFromTemplate(email, selectedRoles);
    
    try {
        if (dasAvailable) {
            const usersTableBody = document.querySelector('#users tbody');
            await usersTableBody.POST(userElement.firstElementChild.outerHTML);
        } else {
            // Fallback: add directly to DOM
            const usersTableBody = document.querySelector('#users tbody');
            usersTableBody.appendChild(userElement);
        }
        
        // Clear form
        emailInput.value = '';
        roleSelect.selectedIndex = -1;
        
        // Initialize auto-save on new elements
        initializeAutoSave();
        
        showStatus(`User ${email} added successfully!`);
    } catch (error) {
        console.error('Failed to add user:', error);
        showStatus('Failed to add user', 'error');
    }
}

function createUserRowFromTemplate(email, roles) {
    const template = document.getElementById('userRowTemplate');
    const clone = template.content.cloneNode(true);
    
    // Set the username
    const usernameSpan = clone.querySelector('[itemprop="username"]');
    usernameSpan.textContent = email;
    
    // Add role tags
    const rolesList = clone.querySelector('.roles-list');
    roles.forEach(role => {
        const roleElement = createRoleTagFromTemplate(role);
        rolesList.appendChild(roleElement);
    });
    
    return clone;
}

function createRoleTagFromTemplate(role) {
    const template = document.getElementById('roleTagTemplate');
    const clone = template.content.cloneNode(true);
    
    const roleSpan = clone.querySelector('[itemprop="role"]');
    roleSpan.textContent = role;
    
    return clone;
}

async function deleteUser(button) {
    const row = button.closest('tr');
    const username = row.querySelector('[itemprop="username"]').textContent;
    
    if (!confirm(`Delete user ${username}?`)) {
        return;
    }
    
    try {
        if (dasAvailable) {
            await row.DELETE();
        } else {
            row.remove();
        }
        showStatus(`User ${username} deleted successfully!`);
    } catch (error) {
        console.error('Failed to delete user:', error);
        showStatus('Failed to delete user', 'error');
    }
}

function addRoleToUser(button) {
    const role = prompt('Enter role name:', 'user');
    if (!role) return;
    
    const rolesList = button.parentNode.querySelector('.roles-list');
    const roleElement = createRoleTagFromTemplate(role);
    rolesList.appendChild(roleElement);
}

async function removeRole(button) {
    const roleItem = button.closest('li');
    try {
        if (dasAvailable) {
            await roleItem.DELETE();
        } else {
            roleItem.remove();
        }
    } catch (error) {
        console.error('Failed to remove role:', error);
        roleItem.remove(); // Fallback
    }
}

// Authorization Rules Functions
async function addRule() {
    const userInput = document.getElementById('newRuleUser');
    const pathInput = document.getElementById('newRulePath');
    const selectorInput = document.getElementById('newRuleSelector');
    const methodSelect = document.getElementById('newRuleMethod');
    const actionSelect = document.getElementById('newRuleAction');
    const descriptionInput = document.getElementById('newRuleDescription');
    
    const user = userInput.value.trim();
    const path = pathInput.value.trim();
    const selector = selectorInput.value.trim();
    const selectedMethods = Array.from(methodSelect.selectedOptions).map(opt => opt.value);
    const action = actionSelect.value;
    const description = descriptionInput.value.trim();
    
    if (!user || !path || selectedMethods.length === 0 || !action || !description) {
        showStatus('Please fill in all required fields', 'error');
        return;
    }
    
    const ruleElement = createRuleRowFromTemplate(user, path, selector, selectedMethods, action, description);
    
    try {
        if (dasAvailable) {
            const rulesTableBody = document.querySelector('#authorization tbody');
            await rulesTableBody.POST(ruleElement.firstElementChild.outerHTML);
        } else {
            const rulesTableBody = document.querySelector('#authorization tbody');
            rulesTableBody.appendChild(ruleElement);
        }
        
        // Clear form
        userInput.value = '';
        pathInput.value = '';
        selectorInput.value = '';
        methodSelect.selectedIndex = -1;
        actionSelect.selectedIndex = 0;
        descriptionInput.value = '';
        
        // Initialize auto-save on new elements
        initializeAutoSave();
        
        showStatus('Authorization rule added successfully!');
    } catch (error) {
        console.error('Failed to add rule:', error);
        showStatus('Failed to add rule', 'error');
    }
}

function createRuleRowFromTemplate(user, path, selector, methods, action, description) {
    const template = document.getElementById('ruleRowTemplate');
    const clone = template.content.cloneNode(true);
    
    // Set user/role with appropriate itemprop
    const userProp = user.includes('@') ? 'username' : 'role';
    const userSpan = clone.querySelector('td:first-child span');
    userSpan.setAttribute('itemprop', userProp);
    userSpan.textContent = user;
    
    // Set path
    const pathSpan = clone.querySelector('[itemprop="path"]');
    pathSpan.textContent = path;
    
    // Set selector
    const selectorSpan = clone.querySelector('[itemprop="selector"]');
    selectorSpan.textContent = selector;
    
    // Add method tags
    const methodsList = clone.querySelector('.methods-list');
    methods.forEach(method => {
        const methodElement = createMethodTagFromTemplate(method);
        methodsList.appendChild(methodElement);
    });
    
    // Set action
    const actionSelect = clone.querySelector('[itemprop="action"]');
    actionSelect.value = action;
    
    // Set description
    const descriptionSpan = clone.querySelector('td:nth-last-child(2) span');
    descriptionSpan.textContent = description;
    
    return clone;
}

function createMethodTagFromTemplate(method) {
    const template = document.getElementById('methodTagTemplate');
    const clone = template.content.cloneNode(true);
    
    const methodSpan = clone.querySelector('[itemprop="method"]');
    methodSpan.textContent = method;
    
    return clone;
}

async function deleteRule(button) {
    const row = button.closest('tr');
    const description = row.querySelector('td:nth-last-child(2) .editable').textContent;
    
    if (!confirm(`Delete rule: ${description}?`)) {
        return;
    }
    
    try {
        if (dasAvailable) {
            await row.DELETE();
        } else {
            row.remove();
        }
        showStatus('Authorization rule deleted successfully!');
    } catch (error) {
        console.error('Failed to delete rule:', error);
        showStatus('Failed to delete rule', 'error');
    }
}

function addMethodToRule(button) {
    const method = prompt('Enter HTTP method:', 'GET');
    if (!method) return;
    
    const methodsList = button.parentNode.querySelector('.methods-list');
    const methodElement = createMethodTagFromTemplate(method.toUpperCase());
    methodsList.appendChild(methodElement);
}

async function removeMethod(button) {
    const methodItem = button.closest('li');
    try {
        if (dasAvailable) {
            await methodItem.DELETE();
        } else {
            methodItem.remove();
        }
    } catch (error) {
        console.error('Failed to remove method:', error);
        methodItem.remove(); // Fallback
    }
}

function updateRule(select) {
    // The change is automatically reflected in the itemprop attribute
    showStatus('Rule updated successfully!');
}

// Utility functions
function isValidEmail(email) {
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
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

// Make functions available globally
window.addUser = addUser;
window.deleteUser = deleteUser;
window.addRoleToUser = addRoleToUser;
window.removeRole = removeRole;
window.addRule = addRule;
window.deleteRule = deleteRule;
window.addMethodToRule = addMethodToRule;
window.removeMethod = removeMethod;
window.updateRule = updateRule;