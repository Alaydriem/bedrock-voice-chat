export interface TableColumn<Row> {
  key: string;
  /** Value used for sorting. Omit for a column that cannot be sorted. */
  sortBy?: (row: Row) => string | number;
}

export interface TableOptions<Row> {
  rows: readonly Row[];
  columns: readonly TableColumn<Row>[];
  /** Fields a search query is matched against. */
  search?: (row: Row) => string;
  /** Stable identity, used for selection. */
  id: (row: Row) => string;
  pageSize?: number;
  /** Called whenever the visible set, sort, page or selection changes. */
  onRender: (view: TableView<Row>) => void;
}

export interface TableView<Row> {
  /** Rows on the current page. */
  page: readonly Row[];
  /** Rows matching the query, across all pages. */
  matching: number;
  pageIndex: number;
  pageCount: number;
  sortKey: string | null;
  sortAscending: boolean;
  selected: ReadonlySet<string>;
  /** True when every matching row is selected. */
  allSelected: boolean;
}

/**
 * Search, sort, paginate and select, as one unit.
 *
 * These four arrive together — the moment a list is long enough to need one it needs
 * the rest — and they interact: searching resets the page, sorting resets the page,
 * and "select all" means every row matching the current query rather than every row
 * that exists. Wiring them separately is how a table ends up selecting rows the user
 * cannot see.
 *
 * Holds no DOM. It computes a view and hands it back; rendering is the caller's.
 */
export class TableController<Row> {
  #options: TableOptions<Row>;
  #rows: Row[];
  #query = "";
  #sortKey: string | null = null;
  #ascending = true;
  #pageIndex = 0;
  #selected = new Set<string>();

  constructor(options: TableOptions<Row>) {
    this.#options = options;
    this.#rows = [...options.rows];
  }

  /** Rows per page. Without an explicit size the whole set is one page. */
  get pageSize(): number {
    return this.#sizeFor(this.#rows.length);
  }

  get selected(): ReadonlySet<string> {
    return this.#selected;
  }

  setRows(rows: readonly Row[]): void {
    this.#rows = [...rows];
    // Selection is by id, so it survives a refresh — but ids that no longer exist
    // would otherwise linger and inflate the count.
    const live = new Set(this.#rows.map(this.#options.id));
    for (const id of [...this.#selected]) if (!live.has(id)) this.#selected.delete(id);
    this.#clampPage();
    this.render();
  }

  search(query: string): void {
    this.#query = query.trim().toLowerCase();
    this.#pageIndex = 0;
    this.render();
  }

  /** Sort by a column, or reverse it when it is already the sort column. */
  sort(key: string): void {
    if (this.#sortKey === key) this.#ascending = !this.#ascending;
    else {
      this.#sortKey = key;
      this.#ascending = true;
    }
    this.#pageIndex = 0;
    this.render();
  }

  /** Drop the sort. Call this when a manual drag order takes over. */
  clearSort(): void {
    this.#sortKey = null;
    this.#ascending = true;
    this.render();
  }

  goToPage(index: number): void {
    this.#pageIndex = index;
    this.#clampPage();
    this.render();
  }

  toggleSelected(id: string): void {
    if (this.#selected.has(id)) this.#selected.delete(id);
    else this.#selected.add(id);
    this.render();
  }

  /** Select every matching row, or clear if they are all already selected. */
  toggleAll(): void {
    const matching = this.#matching();
    const ids = matching.map(this.#options.id);
    const everySelected = ids.length > 0 && ids.every((id) => this.#selected.has(id));
    if (everySelected) for (const id of ids) this.#selected.delete(id);
    else for (const id of ids) this.#selected.add(id);
    this.render();
  }

  clearSelection(): void {
    this.#selected.clear();
    this.render();
  }

  /** The rows currently selected, in the current order. */
  selectedRows(): Row[] {
    return this.#rows.filter((row) => this.#selected.has(this.#options.id(row)));
  }

  render(): void {
    const matching = this.#matching();
    const size = this.#sizeFor(matching.length);
    const pageCount = Math.max(1, Math.ceil(matching.length / size));
    if (this.#pageIndex >= pageCount) this.#pageIndex = pageCount - 1;
    const ids = matching.map(this.#options.id);
    this.#options.onRender({
      page: matching.slice(this.#pageIndex * size, this.#pageIndex * size + size),
      matching: matching.length,
      pageIndex: this.#pageIndex,
      pageCount,
      sortKey: this.#sortKey,
      sortAscending: this.#ascending,
      selected: this.#selected,
      allSelected: ids.length > 0 && ids.every((id) => this.#selected.has(id)),
    });
  }

  #matching(): Row[] {
    const { search, columns } = this.#options;
    let rows = this.#rows;
    if (this.#query && search) {
      rows = rows.filter((row) => search(row).toLowerCase().includes(this.#query));
    }
    const column = columns.find((c) => c.key === this.#sortKey);
    if (column?.sortBy) {
      const by = column.sortBy;
      const direction = this.#ascending ? 1 : -1;
      rows = [...rows].sort((a, b) => {
        const av = by(a);
        const bv = by(b);
        return (av > bv ? 1 : av < bv ? -1 : 0) * direction;
      });
    }
    return rows;
  }

  #clampPage(): void {
    const matching = this.#matching().length;
    const pageCount = Math.max(1, Math.ceil(matching / this.#sizeFor(matching)));
    this.#pageIndex = Math.max(0, Math.min(this.#pageIndex, pageCount - 1));
  }

  /** Never zero: an unpaged table is one page of everything, and 0 would divide badly. */
  #sizeFor(total: number): number {
    return this.#options.pageSize ?? Math.max(1, total);
  }
}
