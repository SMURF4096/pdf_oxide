/**
 * Managed streaming-table adapter (v0.3.39, issue #393).
 *
 * The real O(cols)-memory streaming-table FFI is pending. Until it
 * lands, this adapter buffers rows on the JS heap and flushes them
 * through the buffered-table FFI when {@link StreamingTable.finish}
 * is called. The public shape deliberately matches the future
 * streaming-FFI surface so code written today does not need to
 * migrate when it ships — `pushRow`/`finish`/`pushAll` stay, only the
 * underlying transport changes.
 *
 * @example
 * ```typescript
 * const t = page.streamingTable({
 *   columns: [
 *     { header: 'SKU',  width: 72 },
 *     { header: 'Item', width: 200 },
 *     { header: 'Qty',  width: 48, align: Align.Right },
 *   ],
 *   repeatHeader: true,
 * });
 * for await (const row of readRowsFromDb()) {
 *   t.pushRow([row.sku, row.item, String(row.qty)]);
 * }
 * await t.finish();
 * ```
 */

import type { PageBuilder } from './document-builder.js';
import type { Column, StreamingTableConfig } from '../types/common.js';

export class StreamingTable {
  private _page: PageBuilder;
  private _columns: Column[];
  private _repeatHeader: boolean;
  private _rows: Array<Array<string | null>> = [];
  private _finished = false;

  /** @internal — constructed via `PageBuilder.streamingTable(...)`. */
  constructor(page: PageBuilder, config: StreamingTableConfig) {
    if (!config || !Array.isArray(config.columns) || config.columns.length === 0) {
      throw new Error('StreamingTable requires at least one column');
    }
    this._page = page;
    this._columns = config.columns;
    this._repeatHeader = config.repeatHeader !== false;
  }

  /** Push a single row. Throws if `cells.length !== columns.length`. */
  pushRow(cells: Array<string | null | undefined>): this {
    if (this._finished) {
      throw new Error('StreamingTable already finished');
    }
    if (cells.length !== this._columns.length) {
      throw new Error(
        `row width ${cells.length} does not match column count ${this._columns.length}`
      );
    }
    this._rows.push(cells.map((c) => (c == null ? null : String(c))));
    return this;
  }

  /**
   * Convenience: consume a sync or async iterable and push each row.
   * Useful for DB cursors, streams, generators.
   */
  async pushAll(
    rows: Iterable<Array<string | null | undefined>> | AsyncIterable<Array<string | null | undefined>>
  ): Promise<this> {
    if (this._finished) {
      throw new Error('StreamingTable already finished');
    }
    // Handle both sync and async iterables uniformly. Direct
    // `for await` works on both Iterable and AsyncIterable under
    // modern JS semantics, so we dispatch that way.
    const anyRows = rows as
      | (Iterable<Array<string | null | undefined>> &
          Partial<AsyncIterable<Array<string | null | undefined>>>);
    if (typeof anyRows[Symbol.asyncIterator] === 'function') {
      for await (const row of rows as AsyncIterable<Array<string | null | undefined>>) {
        this.pushRow(row);
      }
    } else {
      for (const row of rows as Iterable<Array<string | null | undefined>>) {
        this.pushRow(row);
      }
    }
    return this;
  }

  /**
   * Flush all buffered rows to the buffered-table FFI and return the
   * parent PageBuilder for chaining. The adapter becomes invalid
   * after `finish()`.
   */
  async finish(): Promise<PageBuilder> {
    if (this._finished) {
      throw new Error('StreamingTable already finished');
    }
    this._finished = true;
    this._page.table({
      columns: this._columns,
      rows: this._rows,
      hasHeader: this._repeatHeader,
    });
    // Release buffered-row memory early.
    this._rows = [];
    return this._page;
  }

  /**
   * Number of body rows buffered so far (does not include the header).
   */
  get rowCount(): number {
    return this._rows.length;
  }
}
