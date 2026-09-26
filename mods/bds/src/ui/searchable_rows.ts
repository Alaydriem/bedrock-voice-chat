import { ObservableBoolean, ObservableString } from '@minecraft/server-ui';
import type { CustomForm } from '@minecraft/server-ui';

export interface SearchableRow {
  // What the search query matches against.
  key: string;
  label: ObservableString;
  onSelect: () => void;
}

export interface SearchableRowsOptions {
  placeholder: string;
  description: string;
}

/// A search field over a column of one-line button rows.
///
/// A form's rows are fixed when it is built, so every row is added up front and the
/// query flips each row's visibility instead of rebuilding the list.
export class SearchableRows {
  /// Adds the field and the rows to `form`. Returns the unsubscribes the caller
  /// must run when the form closes.
  static attach(
    form: CustomForm,
    rows: SearchableRow[],
    options: SearchableRowsOptions,
  ): Array<() => void> {
    const unsubscribes: Array<() => void> = [];
    const search = new ObservableString('', { clientWritable: true });
    form.textField(options.placeholder, search, {
      description: options.description,
    });
    form.divider();

    const visibility = new Map<string, ObservableBoolean>();
    for (const row of rows) {
      const visible = new ObservableBoolean(true);
      visibility.set(row.key, visible);
      form.button(row.label, row.onSelect, { visible });
    }

    const listener = search.subscribe((query) => {
      const needle = query.trim().toLowerCase();
      for (const [key, visible] of visibility) {
        visible.setData(key.toLowerCase().includes(needle));
      }
    });
    unsubscribes.push(() => {
      search.unsubscribe(listener);
    });
    return unsubscribes;
  }
}
