import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { TableController, type TableView } from "../core/controllers/TableController";

interface Row {
  name: string;
  size: number;
}

const ROWS: Row[] = [
  { name: "airhorn", size: 38 },
  { name: "applause", size: 142 },
  { name: "creeper-hiss", size: 61 },
  { name: "drumroll", size: 96 },
  { name: "ender-portal", size: 188 },
  { name: "fanfare", size: 120 },
  { name: "rimshot", size: 22 },
];

function build(pageSize?: number) {
  let view!: TableView<Row>;
  const table = new TableController<Row>({
    rows: ROWS,
    id: (r) => r.name,
    search: (r) => r.name,
    pageSize,
    columns: [
      { key: "name", sortBy: (r) => r.name },
      { key: "size", sortBy: (r) => r.size },
    ],
    onRender: (v) => (view = v),
  });
  table.render();
  return { table, view: () => view };
}

/**
 * Search, sort, paging and selection interact, and wiring them separately is how a
 * table ends up acting on rows the user cannot see.
 */
describe("TableController", () => {
  it("puts everything on one page when no size is given", () => {
    const { table, view } = build();
    assert.equal(view().page.length, ROWS.length);
    assert.equal(view().pageCount, 1);
    table.goToPage(5);
    assert.equal(view().pageIndex, 0);
  });

  it("pages, and clamps a page past the end", () => {
    const { table, view } = build(3);
    assert.equal(view().pageCount, 3);
    assert.equal(view().page.length, 3);
    table.goToPage(99);
    assert.equal(view().pageIndex, 2);
    assert.equal(view().page.length, 1);
  });

  it("returns to the first page when the query changes", () => {
    // Otherwise a search from page three shows an empty page and reads as no results.
    const { table, view } = build(3);
    table.goToPage(2);
    table.search("a");
    assert.equal(view().pageIndex, 0);
  });

  it("returns to the first page when the sort changes", () => {
    const { table, view } = build(3);
    table.goToPage(2);
    table.sort("size");
    assert.equal(view().pageIndex, 0);
  });

  it("reverses when the same column is sorted twice", () => {
    const { table, view } = build();
    table.sort("size");
    assert.equal(view().sortAscending, true);
    assert.equal(view().page[0].name, "rimshot");
    table.sort("size");
    assert.equal(view().sortAscending, false);
    assert.equal(view().page[0].name, "ender-portal");
  });

  it("drops the sort when a manual order takes over", () => {
    const { table, view } = build();
    table.sort("size");
    table.clearSort();
    assert.equal(view().sortKey, null);
    assert.equal(view().page[0].name, ROWS[0].name);
  });

  it("selects only what the current query matches", () => {
    // "Select all" has to mean the rows on screen. Selecting hidden rows and then
    // deleting is the failure this guards.
    const { table, view } = build();
    table.search("a");
    const matching = view().matching;
    table.toggleAll();
    assert.equal(table.selected.size, matching);
    assert.ok(matching < ROWS.length);
    assert.ok(!table.selected.has("rimshot"));
  });

  it("clears the selection when everything matching is already selected", () => {
    const { table } = build();
    table.toggleAll();
    assert.equal(table.selected.size, ROWS.length);
    table.toggleAll();
    assert.equal(table.selected.size, 0);
  });

  it("keeps a selection made on another page", () => {
    const { table, view } = build(3);
    table.toggleSelected("airhorn");
    table.goToPage(2);
    assert.equal(view().selected.has("airhorn"), true);
    assert.deepEqual(
      table.selectedRows().map((r) => r.name),
      ["airhorn"],
    );
  });

  it("forgets ids that no longer exist after a refresh", () => {
    const { table } = build();
    table.toggleSelected("airhorn");
    table.toggleSelected("rimshot");
    table.setRows(ROWS.filter((r) => r.name !== "airhorn"));
    assert.deepEqual([...table.selected], ["rimshot"]);
  });

  it("reports allSelected against the query, not the whole set", () => {
    const { table, view } = build();
    table.search("rimshot");
    table.toggleAll();
    assert.equal(view().allSelected, true);
    table.search("");
    assert.equal(view().allSelected, false);
  });

  it("does not claim everything is selected when nothing matches", () => {
    const { table, view } = build();
    table.search("nothing matches this");
    assert.equal(view().matching, 0);
    assert.equal(view().allSelected, false);
  });
});
