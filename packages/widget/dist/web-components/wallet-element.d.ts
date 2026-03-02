export declare class RampOSWalletElement extends HTMLElement {
    private root;
    private mountPoint;
    static get observedAttributes(): string[];
    constructor();
    connectedCallback(): void;
    attributeChangedCallback(): void;
    disconnectedCallback(): void;
    private getTheme;
    private renderComponent;
}
