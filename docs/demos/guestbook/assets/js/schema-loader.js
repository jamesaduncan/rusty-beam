// Schema Loader with Inheritance Resolution
// Loads and resolves schema inheritance chains from HTML documents

class SchemaLoader {
    constructor() {
        this.cache = new Map();
        this.loading = new Map(); // Prevent concurrent loads
        this.baseUrl = window.location.origin;
    }

    /**
     * Load a schema by URL and resolve its inheritance chain
     * @param {string} schemaUrl - The schema URL to load
     * @returns {Promise<Object>} Resolved schema with all inherited properties
     */
    async loadSchema(schemaUrl) {
        if (this.cache.has(schemaUrl)) {
            return this.cache.get(schemaUrl);
        }

        // Prevent concurrent loading of the same schema
        if (this.loading.has(schemaUrl)) {
            return this.loading.get(schemaUrl);
        }

        const loadPromise = this._loadSchemaInternal(schemaUrl);
        this.loading.set(schemaUrl, loadPromise);

        try {
            const schema = await loadPromise;
            this.cache.set(schemaUrl, schema);
            return schema;
        } finally {
            this.loading.delete(schemaUrl);
        }
    }

    /**
     * Internal schema loading implementation
     * @private
     */
    async _loadSchemaInternal(schemaUrl) {
        try {
            // Convert schema URL to documentation path
            const docPath = this._schemaUrlToDocPath(schemaUrl);
            const response = await fetch(docPath);
            
            if (!response.ok) {
                throw new Error(`Failed to load schema: ${response.status} ${response.statusText}`);
            }

            const html = await response.text();
            const schema = this._parseSchemaFromHtml(html, schemaUrl);

            // Resolve inheritance chain
            if (schema.parent) {
                const parentSchema = await this.loadSchema(schema.parent);
                schema.properties = { ...parentSchema.properties, ...schema.properties };
                schema.inheritanceChain = [...(parentSchema.inheritanceChain || []), schema.parent];
            } else {
                schema.inheritanceChain = [];
            }

            return schema;
        } catch (error) {
            console.error(`Error loading schema ${schemaUrl}:`, error);
            throw error;
        }
    }

    /**
     * Convert schema URL to documentation file path
     * @private
     */
    _schemaUrlToDocPath(schemaUrl) {
        // Extract schema name from URL: http://rustybeam.net/OAuth2Plugin -> OAuth2Plugin
        const schemaName = schemaUrl.split('/').pop();
        
        // For local development, always use local schema paths
        // In production, this could check if running on rustybeam.net
        return `/schema/${schemaName}/`;
    }

    /**
     * Parse schema definition from HTML document
     * @private
     */
    _parseSchemaFromHtml(html, schemaUrl) {
        const parser = new DOMParser();
        const doc = parser.parseFromString(html, 'text/html');

        // Find the main schema element (usually on body element)
        let schemaElement = doc.querySelector(`[itemtype="${schemaUrl}"]`);
        
        // Some schemas use the full URL, others use path-relative
        if (!schemaElement && schemaUrl.startsWith('http://rustybeam.net/schema/')) {
            const schemaName = schemaUrl.split('/').pop();
            schemaElement = doc.querySelector(`[itemtype="http://rustybeam.net/schema/${schemaName}"]`);
        }
        
        if (!schemaElement) {
            // Default to body if it has itemscope
            if (doc.body && doc.body.hasAttribute('itemscope')) {
                schemaElement = doc.body;
            } else {
                throw new Error(`Schema element not found for ${schemaUrl}`);
            }
        }

        const schema = {
            url: schemaUrl,
            name: schemaUrl.split('/').pop(),
            properties: new Map(),
            title: doc.querySelector('title')?.textContent || schemaUrl
        };

        // Extract parent schema URL
        const parentElement = schemaElement.querySelector('[itemprop="parent"]');
        if (parentElement) {
            schema.parent = parentElement.textContent.trim();
        }

        // Extract properties from the properties table
        const propertyRows = doc.querySelectorAll('tr[itemtype="http://organised.team/Property"]');
        propertyRows.forEach(row => {
            const property = this._parsePropertyFromRow(row);
            if (property) {
                schema.properties.set(property.name, property);
            }
        });

        return schema;
    }

    /**
     * Parse a property definition from a table row
     * @private
     */
    _parsePropertyFromRow(row) {
        const nameElement = row.querySelector('[itemprop="name"]');
        const typeElement = row.querySelector('[itemprop="type"]');
        const cardinalityElement = row.querySelector('[itemprop="cardinality"]');
        const descriptionElement = row.querySelector('[itemprop="description"]');

        if (!nameElement || !typeElement) {
            return null;
        }

        return {
            name: nameElement.textContent.trim(),
            type: typeElement.textContent.trim(),
            cardinality: cardinalityElement?.textContent.trim() || '0..1',
            description: descriptionElement?.textContent.trim() || '',
            required: this._isRequired(cardinalityElement?.textContent.trim() || '0..1')
        };
    }

    /**
     * Determine if a property is required based on cardinality
     * @private
     */
    _isRequired(cardinality) {
        return cardinality === '1' || cardinality.startsWith('1..');
    }

    /**
     * Validate a value against a property definition
     */
    validateProperty(property, value) {
        const errors = [];

        // Required validation
        if (property.required && (!value || value.trim() === '')) {
            errors.push(`${property.name} is required`);
            return errors;
        }

        // Skip further validation if empty and not required
        if (!value || value.trim() === '') {
            return errors;
        }

        // Type validation
        switch (property.type.toLowerCase()) {
            case 'url':
                if (!this._isValidUrl(value)) {
                    errors.push(`${property.name} must be a valid URL`);
                }
                break;
            case 'number':
                if (isNaN(Number(value))) {
                    errors.push(`${property.name} must be a number`);
                }
                break;
            case 'boolean':
                if (!['true', 'false', '1', '0'].includes(value.toLowerCase())) {
                    errors.push(`${property.name} must be true or false`);
                }
                break;
            case 'text':
                // Text is always valid, but we could add length restrictions here
                break;
        }

        return errors;
    }

    /**
     * Validate URL format
     * @private
     */
    _isValidUrl(string) {
        try {
            new URL(string);
            return true;
        } catch (_) {
            // Check for relative URLs and file:// protocol
            return string.startsWith('/') || string.startsWith('./') || string.startsWith('file://');
        }
    }

    /**
     * Get all properties for a schema (including inherited)
     */
    async getSchemaProperties(schemaUrl) {
        const schema = await this.loadSchema(schemaUrl);
        return schema.properties;
    }

    /**
     * Validate an entire plugin configuration against its schema
     */
    async validatePlugin(pluginElement) {
        const schemaUrl = pluginElement.getAttribute('itemtype');
        if (!schemaUrl) {
            return [{ property: 'itemtype', message: 'No schema specified for plugin' }];
        }

        try {
            const schema = await this.loadSchema(schemaUrl);
            const errors = [];

            // Validate each property
            for (const [propertyName, propertyDef] of schema.properties) {
                const propertyElement = pluginElement.querySelector(`[itemprop="${propertyName}"]`);
                const value = propertyElement?.textContent?.trim() || '';
                
                const propertyErrors = this.validateProperty(propertyDef, value);
                propertyErrors.forEach(message => {
                    errors.push({ property: propertyName, message });
                });
            }

            return errors;
        } catch (error) {
            return [{ property: 'schema', message: `Failed to load schema: ${error.message}` }];
        }
    }

    /**
     * Clear cache (useful for development/testing)
     */
    clearCache() {
        this.cache.clear();
        this.loading.clear();
    }

    /**
     * Get cache statistics
     */
    getCacheStats() {
        return {
            cachedSchemas: this.cache.size,
            loadingSchemas: this.loading.size,
            cacheKeys: Array.from(this.cache.keys())
        };
    }
}

// Create global schema loader instance
window.schemaLoader = new SchemaLoader();

// Export for module use
if (typeof module !== 'undefined' && module.exports) {
    module.exports = SchemaLoader;
}