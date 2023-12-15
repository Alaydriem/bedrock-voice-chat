const ACTIVE_CLASS = "is-active";
const TAB_HEADER_CLASS = "tab";
const TAB_CONTENT_CLASS = "tab-content";

export default class Tab {
  aciveTab = null;

  constructor(wrapper, onChange = () => { }) {
    if (wrapper instanceof HTMLElement) {
      this.wrapper = wrapper;
    } else {
      this.wrapper = document.querySelector(wrapper);
    }

    this.onChange = onChange;

    if (!this.wrapper) {
      throw new TypeError("Error: Tab Wrapper not defined");
    }

    this.tabs = this.wrapper.querySelectorAll(`.${TAB_HEADER_CLASS}`);
    this.tabContents = this.wrapper.querySelectorAll(`.${TAB_CONTENT_CLASS}`);

    if (this.tabs.length === 0) {
      throw new TypeError("Error: Tab items not defined");
    }

    this.aciveTab =
      this.wrapper.dataset.activeTab || this.tabs[0].dataset.target;

    this.showTab(this.aciveTab);

    this.tabs.forEach((node) => {
      const nodeID = node.dataset.target;

      node.addEventListener("click", () => {
        if (nodeID != this.aciveTab) {
          this.showTab(nodeID);
        }
      });
    });
  }

  showTab(id) {
    this.tabs.forEach((node) => {
      const activeClass = node.dataset.activeClass;
      const defaultClass = node.dataset.defaultClass;

      if (node.dataset.target == id) {
        if (defaultClass)
          node.classList.remove(...defaultClass.trim().split(" "));
        if (activeClass)
          node.classList.add(...activeClass.trim().split(" "), ACTIVE_CLASS);
      } else {
        if (defaultClass) node.classList.add(...defaultClass.trim().split(" "));
        if (activeClass)
          node.classList.remove(...activeClass.trim().split(" "), ACTIVE_CLASS);
      }
    });

    if (this.tabContents) {
      this.tabContents.forEach((node) => {
        if (`#${node.id}` == id) node.classList.add(ACTIVE_CLASS);
        else node.classList.remove(ACTIVE_CLASS);
      });
    }

    this.aciveTab = id;

    this.onChange(id)
  }
}
