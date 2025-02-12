export default () => ({
  isActive: false,

  init() {
    if (!this.$el.href) return;

    this.isActive =
      this.$el.href.split("?")[0].split("#")[0] ===
      this.$store.global.currentLocation;
    if (this.isActive) {
      if (this.acc_id) this.expanded = true;
      this.$el.scrollIntoView({ block: "center" });
    }
  },
});
