// jsdom does no layout: every element measures 0×0, so a virtualized list would render no
// rows. Give the scrolling area of the item list a window-like size in tests.
const LIST = "item-list";

for (const [property, size] of [
  ["offsetHeight", 600],
  ["offsetWidth", 1000],
  ["clientHeight", 600],
  ["clientWidth", 1000],
] as const) {
  Object.defineProperty(HTMLElement.prototype, property, {
    configurable: true,
    get(this: HTMLElement) {
      return this.dataset.testid === LIST ? size : 0;
    },
  });
}
