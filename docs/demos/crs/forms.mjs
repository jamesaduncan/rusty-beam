import { SelectorSubscriber } from "https://jamesaduncan.github.io/selector-subscriber/index.mjs";

let saveTimeout = 0;
SelectorSubscriber.subscribe('[contenteditable]', async(element) => {
    element.addEventListener('blur', async (event) => {
        // Clear any pending save
        if (saveTimeout) {
            clearTimeout(saveTimeout);
        }
        
        element.dispatchEvent(new CustomEvent('save-start', {
            detail: { element }
        }));
        
        // Debounce saves by 500ms
        saveTimeout = setTimeout(async () => {
            try {
                const response = await element.PUT();
                if (response.ok) {
                    element.dispatchEvent(new CustomEvent('save-success', {
                        detail: { element },
                        bubbles: true
                    }));
                } else {
                    element.dispatchEvent(new CustomEvent('save-failure', {
                        detail: { element, error: response.statusText },
                        bubbles: true
                    }));
                }
            } catch (error) {
                element.dispatchEvent(new CustomEvent('save-failure', {
                    detail: { element, error },
                    bubbles: true
                }));
            }
        }, 500);

    });
});

SelectorSubscriber.subscribe('button[commandfor]', (button) => {
    const target = document.getElementById( button.getAttribute('commandfor') );
    target.addEventListener('command', async ( event ) => {
        const template = target.querySelector('template');
        const clone = template.content.cloneNode(true);
        const selector = `#(selector=${target.selector})`;
        if ( await window.server.can("POST", selector) ) {
            target.POST( clone );
        }
    });
});

SelectorSubscriber.subscribe('button[command="--remove"][closest]:not([commandfor])', (button) => {
    const target = button.closest( button.getAttribute('closest') );
    button.commandForElement = target;
    target.addEventListener('command', async ( event ) => {
        const selector = `#(selector=${target.selector})`;
        console.log(`testing delete on ${selector}`);
        if ( await window.server.can("DELETE", selector) ) {
            target.DELETE();
        } else {
            console.warn(`Cannot delete ${selector}`);
        }
    });
});