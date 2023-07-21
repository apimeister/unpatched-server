class DtzContextDropdown extends HTMLElement {
    constructor() {
        super();
    }
    changeContext(evt) {
        if (evt) evt.preventDefault();
        let val = evt.target.value;
        console.log('new value ' + val);
        // set value on custom element/ this is bound to the select
        let root = this.parentElement;
        root.value = val;
        root.setAttribute("value",val);
    }
    async connectedCallback() {
        let list = getContextListFromCookie();
        let currentContext = getCurrentContext();
        let str = "";
        for (let contextId of list) {
            let alias = getAliasForContext(contextId);
            str += `<option value="${contextId}" ${currentContext == contextId ? 'selected' : ''}>${alias} (${contextId})</option>`;
        }
        this.innerHTML = `<select class="form-select" aria-label="choose context">
        ${str}
      </select>`;
        this.value = currentContext;
        this.querySelector('select').addEventListener('change', this.changeContext);
    }
}
window.customElements.define('dtz-context-dropdown', DtzContextDropdown);
