export default (id) => ({
  acc_id: id,
  get expanded() {
    return this.expandedItem === this.acc_id;
  },
  set expanded(val) {
    this.expandedItem = val ? this.acc_id : null;
  }
});
