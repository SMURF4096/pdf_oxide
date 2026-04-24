/**
 * Streaming-table adapter backed by the native row-at-a-time FFI
 * (`pdf_page_builder_streaming_table_begin_v2` / `_push_row` / `_finish`).
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
 *   mode: { kind: 'sample', sampleRows: 30 },
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
  private _opened = false;
  private _finished = false;

  /** @internal — constructed via `PageBuilder.streamingTable(...)`. */
  constructor(page: PageBuilder, config: StreamingTableConfig) {
    if (!config || !Array.isArray(config.columns) || config.columns.length === 0) {
      throw new Error('StreamingTable requires at least one column');
    }
    this._page = page;
    this._columns = config.columns;

    const headers = config.columns.map((c) => c.header ?? '');
    const widths  = config.columns.map((c) => c.width);
    const aligns  = config.columns.map((c) => (c.align ?? 0) as number);
    const repeat  = config.repeatHeader !== false;

    this._page._streamingTableBeginV2(headers, widths, aligns, repeat, config.mode);
    this._opened = true;
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
    this._page._streamingTablePushRow(cells.map((c) => (c == null ? null : String(c))));
    return this;
  }

  /**
   * Convenience: consume a sync or async iterable and push each row.
   */
  async pushAll(
    rows: Iterable<Array<string | null | undefined>> | AsyncIterable<Array<string | null | undefined>>
  ): Promise<this> {
    if (this._finished) {
      throw new Error('StreamingTable already finished');
    }
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
   * Close the streaming table and return the parent PageBuilder for chaining.
   */
  async finish(): Promise<PageBuilder> {
    if (this._finished) {
      throw new Error('StreamingTable already finished');
    }
    this._finished = true;
    if (this._opened) {
      this._page._streamingTableFinish();
    }
    return this._page;
  }

  /** Number of the columns this table was opened with. */
  get columnCount(): number {
    return this._columns.length;
  }
}
