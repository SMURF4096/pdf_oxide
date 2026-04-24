# #398 — FFI parser reentrancy root-cause + fix design

**Scope:** read-side FFI (`pdf_document_extract_text`, `pdf_document_get_page_count`, et al.) against a single `PdfDocument` handle shared across OS threads.
**Out of scope:** write-side (`DocumentBuilder`, `DocumentEditor`), cross-document concurrency (already works), rendering pipeline.

---

## TL;DR

The parser is **not** fundamentally single-threaded — there is no per-document seek pointer exposed in the public API and every field on `PdfDocument` is already wrapped in either `Mutex`, `AtomicUsize`, or is immutable after `open()` — **except four fields that are plain `HashMap` / `Option` / `bool` and are mutated on the read path**, and one `Mutex<PdfReader>` whose lock is released between every `seek()` / `read()` pair.

Two distinct root-cause bugs drive #398:

1. **Reader split-lock race (the primary offender).** `load_uncompressed_object_impl` seeks, unlocks, locks, reads — giving a concurrent caller a window to seek the shared `BufReader<File>` to a *different* object before the first caller's read completes. The first caller then parses garbage and returns `Error::ParseError` — observed by the Go binding as `"parse error (code 5)"`. `src/document.rs:2164-2174`.
2. **FFI `&mut *handle` aliasing.** Every read entry point in `src/ffi.rs` turns an `*mut PdfDocument` into `&mut *handle`. Two concurrent callers on the same handle produce *two simultaneous* `&mut` references — immediate language-level UB that LLVM can (and does) miscompile regardless of any `Mutex` inside. `src/ffi.rs:253`, `:277`, `:302` and 40+ more.

The smallest fix: (a) hold the reader lock for the duration of a single object load, (b) migrate the four un-synchronized fields to `Mutex`/`RwLock`, (c) change every read-path method from `&mut self` to `&self`, and (d) FFI calls `&*handle` instead of `&mut *handle`. Estimated 1–2 weeks; API-compatible; v0.3.40-scope.

---

## Surface: what bindings currently do

- **Go** (`go/pdf_oxide.go:414`, `:633`): Both `PdfDocument` and `DocumentEditor` still carry `sync.RWMutex`, but after commit `73e14066` every `RLock`/`RUnlock` was swapped to exclusive `Lock`/`Unlock`. All 90 call sites (66 cgo + 24 purego) serialize, so advertised concurrency is gone. `TestConcurrentReads` (20 goroutines × 5 iterations) passes 25/25 under `-race`; previously failed ~40%.
- **Python** (`src/python.rs`): grep `allow_threads` → 0 hits. Every `#[pymethods]` call holds the GIL for its entire duration. Accidentally safe on CPython 3.13- (GIL == global lock), but becomes racy under free-threaded builds (PEP 703, CPython 3.13+ `--disable-gil`) which ship this year.
- **C#** (`csharp/PdfOxide/Core/PdfDocument.cs`): no `lock(...)` / `SemaphoreSlim` around `NativeMethods.PdfDocument*` calls. Undocumented thread-safety. A caller using `Parallel.ForEach` on a shared `PdfDocument` will hit the same race the Go test caught.
- **Node** (`js/`): N-API is single-threaded per worker thread; a single JS `PdfDocument` instance is effectively serialized by the event loop. `Worker_threads` each get their own handle — not affected.
- **WASM** (`src/wasm.rs`): single-threaded by runtime (no `SharedArrayBuffer` for `WasmPdfDocument`). Not affected.

Two of five bindings (Go, Python) accidentally sidestep the bug; two (C#, Node/native) are vulnerable on paper but no one has filed a bug yet; one (WASM) cannot hit it.

---

## Internals: what the parser shares

### `src/document.rs:288-411` — `PdfDocument` struct

The struct advertises `Send + Sync` via a compile-time assertion at `src/document.rs:414-419`:

```rust
const _: () = {
    fn _assert_send_sync<T: Send + Sync>() {}
    fn _check() { _assert_send_sync::<PdfDocument>(); }
};
```

That assertion is **true for `Sync` but only accidentally** — it proves every field is `Sync`, not that concurrent method calls are sound. Most fields do use `Mutex`:

| Field | Line | Sync primitive |
| --- | --- | --- |
| `reader` | `:295` | `Mutex<PdfReader>` |
| `object_cache` | `:307` | `Mutex<BoundedObjectCache>` |
| `resolving_stack` | `:309` | `Mutex<HashSet<ObjectRef>>` |
| `recursion_depth` | `:311` | `Mutex<u32>` |
| `encryption_handler` | `:314` | `Mutex<Option<EncryptionHandler>>` |
| `encrypt_dict_ref` | `:320` | `Mutex<Option<ObjectRef>>` |
| `font_cache` + 4 other font caches | `:330-352` | `Mutex<BoundedEntryCache<…>>` each |
| `scanned_object_offsets` | `:367` | `Mutex<Option<HashMap<…>>>` |
| `objstm_recovery_done` | `:372` | `Mutex<bool>` |
| `image_xobject_cache`, `xobject_text_free_cache`, `xobject_stream_cache` | `:375-383` | `Mutex<…>` each |
| `xobject_stream_cache_bytes` | `:384` | `AtomicUsize` |
| `xobject_spans_cache`, `form_xobject_images_cache` | `:388-394` | `Mutex<BoundedEntryCache>` each |
| `page_content_cache` | `:398` | `Mutex<Option<…>>` |
| `running_artifact_signatures` | `:410` | `Mutex<Option<HashMap>>` |

**But five fields are *not* locked:**

| Field | Line | Type | Mutated on read path? |
| --- | --- | --- | --- |
| `structure_tree_cache` | `:355` | `Option<Option<Arc<StructTreeRoot>>>` | Yes — `document.rs:4014`, `:4017`, `:9195`, `:9198` |
| `structure_content_cache` | `:358` | `Option<HashMap<u32, Vec<OrderedContent>>>` | Yes — `document.rs:4060` |
| `page_cache` | `:361` | `HashMap<usize, Object>` | Yes — `document.rs:3241`, `:3388` |
| `page_cache_populated` | `:364` | `bool` | Yes — `document.rs:3184` |
| `erase_regions` | `:396` | `HashMap<usize, Vec<Rect>>` | Write-only (only `erase_*` methods touch it); read-path reads via `&self.erase_regions.get(...)` at `:6608, :7102, :7413, :7553`. Safe on reads only. |

Those five fields are the reason every public read-path method on `PdfDocument` is declared `fn foo(&mut self, …)`. Once they're `&mut`, the FFI has to fabricate `&mut *handle`, and the dominos fall.

### `src/document.rs:56-100` — `PdfReader` (the seek cursor)

```rust
enum PdfReader {
    File(BufReader<File>),
    Memory(BufReader<Cursor<Vec<u8>>>),
}
```

One `BufReader<File>` per document = **one OS seek pointer**. If the `Mutex<PdfReader>` guard is released between `seek` and `read`, another thread can re-seek the same file descriptor, and the first thread's subsequent `read` returns bytes from the wrong offset.

### `src/cache.rs:16-28` — `MutexExt::lock_or_recover`

```rust
fn lock_or_recover(&self) -> std::sync::MutexGuard<'_, T> {
    self.lock().unwrap_or_else(|poisoned| {
        log::debug!("Mutex was poisoned, recovering");
        poisoned.into_inner()
    })
}
```

Silently discarding poison is safe for bounded caches but is a second-order concern: if one of the races below causes a panic mid-lock, every subsequent caller swallows the poison and the corrupted cache state propagates.

### `src/ffi.rs` — read entry points

All ~40 `pdf_document_*` read-side entries do the same two-line dance:

```rust
let doc = unsafe { &mut *handle };            // ← :253, :277, :302, :327 …
match doc.extract_text(page_index as usize) { /* … */ }
```

A handful use `&*handle` instead — `pdf_document_get_version` at `:224`, `pdf_document_is_encrypted` at `:4004` — but those are the exception. Every entry that calls a method with a `&mut self` receiver (almost all of them) uses `&mut *handle`. Two concurrent callers holding the same `*mut PdfDocument` independently produce two `&mut PdfDocument` references alive at the same time — language-level UB, *regardless* of whether the methods happen to synchronize internally.

---

## The specific races

### Race A — split-lock on `PdfReader` (primary cause of `code 5`)

`load_uncompressed_object_impl`, `src/document.rs:2163-2206`:

```rust
// 2164-2166
self.reader
    .lock_or_recover()
    .seek(SeekFrom::Start(offset))?;               // ← guard dropped here

// 2170-2174
let mut header_bytes = Vec::new();
let bytes_read = self
    .reader
    .lock_or_recover()                             // ← re-acquired here
    .read_until(b'\n', &mut header_bytes)?;
```

The `MutexGuard` returned by the first `.lock_or_recover()` is dropped at the `;` on line 2166. A concurrent thread calling `load_object` for a *different* object can acquire the lock between lines 2166 and 2173, call `seek(SeekFrom::Start(other_offset))`, and drop its guard. Thread 1's `read_until` then reads the bytes at `other_offset`, not `offset`. Downstream parsing fails:

- If the bytes happen to start with a valid `"N G obj"` header for a *different* object, `parse_object_number` returns the wrong id and the subsequent parser sees mismatched content — surfaced as `Error::ParseError { offset, reason: "Expected object header, found: …" }` at `:2244`.
- If the bytes are mid-stream binary garbage, `String::from_utf8_lossy` gives replacement chars, the `obj_pos` search at `:2213` misses, the backwards-search retry at `:2232` may succeed or fail stochastically, and we either return `ParseError` or (worse) silently load the wrong object.

The same split-lock pattern also appears at `src/document.rs:2591-2595` (`find_object_header_backwards` — seek then read) and `:1282-1292` (`recover_from_object_streams` — seek-to-0 then `read_to_end`). The latter is less dangerous because it happens only once per document under the `objstm_recovery_done` guard, but the former fires on every backwards-search retry.

**This is the single race that matches the empirical `code 5 / parse error` signature.** The Go test observed exactly this: two goroutines calling `ExtractText(0)` simultaneously, each doing many `load_object` calls, each interleaving seeks.

### Race B — `page_cache` / `page_cache_populated` torn updates

`src/document.rs:3173-3242`, `get_page`:

```rust
// 3175
if let Some(cached) = self.page_cache.get(&page_index) {
    return Ok(cached.clone());
}
// 3183-3185
if !self.page_cache_populated && cache_misses >= LAZY_THRESHOLD {
    self.page_cache_populated = true;          // ← racy write
    if let Err(e) = self.populate_page_cache() { … }  // ← writes self.page_cache
// …
// 3241
self.page_cache.insert(page_index, page.clone());   // ← racy write
```

`page_cache: HashMap<usize, Object>` is *not* wrapped in a `Mutex` (`src/document.rs:361`), yet `get_page` mutates it from `&mut self`. With two concurrent `&mut *handle` callers:

- **Data race on the `HashMap`**: simultaneous `insert`s will corrupt the hash table (bucket free-list scramble, infinite loop on next lookup, or dropped entries). In practice this panics in `hashbrown` with a probe-length assertion, which `MutexExt` then silently recovers from — with a half-corrupted cache.
- **Torn `bool` read of `page_cache_populated`**: one thread sees `false`, starts `populate_page_cache`, second thread sees `false` too, both race on populating. The `HashMap::insert` inside `populate_page_cache` at `:3388` is concurrent with the `get` at `:3192`.

This race *would* panic or miscompile if detected; in reality LLVM optimizes around it and you get silent cache-entry loss plus the occasional `Error::InvalidPdf("Page tree has no kids")` when the populate loop consumes partial state. Tests see this as flaky `code 5` too (the Go binding maps both `ERR_PARSE=3` and `ERR_INTERNAL=5` cases; the fallback branch at `src/ffi.rs:77` classifies unmatched errors as `ERR_INTERNAL=5`, and the Go side at `go/types.go:109` labels 5 as "parse error").

### Race C — `resolving_stack` false-positive cycle detection

`src/document.rs:1403-1411`:

```rust
if self.resolving_stack.lock_or_recover().contains(&obj_ref) {
    // … logs "Circular reference detected"
    return Err(Error::CircularReference(obj_ref));
}
```

`resolving_stack` is a `Mutex<HashSet<ObjectRef>>` and is **correctly locked**, but the lock scope is a single method call. The set is document-global, not per-thread. Two concurrent threads both calling `load_object(obj_ref_42)`:

- Thread A inserts `obj_ref_42` at `:1540`, descends.
- Thread B enters `load_object(obj_ref_42)`, sees it in the set → returns `Error::CircularReference(obj_ref_42)`. **False positive.**

The same hazard applies to `recursion_depth` at `:1543`: concurrent increments are atomic inside the Mutex, but one thread's deep recursion can push `recursion_depth` over `MAX_RECURSION_DEPTH=100` (`:103`) while another thread is at depth 1, triggering spurious `Error::RecursionLimitExceeded`. Both errors are caught by the Go binding and bubble up as "code 5".

### (Bonus) Race D — `structure_tree_cache` `Option` replacement under `&mut self`

`src/document.rs:4007-4017` in `extract_text_with_options`:

```rust
let cached_tree = match &self.structure_tree_cache {
    Some(tree) => tree.clone(),
    None => {
        let tree_res = self.structure_tree();
        match &tree_res {
            Ok(Some(tree)) => {
                self.structure_tree_cache = Some(Some(Arc::new(tree.clone())));
```

`structure_tree_cache: Option<Option<Arc<StructTreeRoot>>>` — a plain `Option` mutated from `&mut self`. Two concurrent callers: both see `None`, both call `self.structure_tree()` (expensive, re-parses the StructTreeRoot), both assign `self.structure_tree_cache` — the second write drops the first `Arc` immediately. Not a crash race (assignment of `Option<Arc<_>>` is 2×`usize`, not atomic on most 64-bit targets) but **is** a torn-write race; LLVM may reorder the assignment and the guard `is_none()` check in other call sites. Same hazard at `:9195`, `:9198`, and the `structure_content_cache` assignment at `:4060`.

---

## Proposed fixes (ranked by scope)

### Option 1 — Narrow to interior mutability (**recommended**)

Total diff size: ~200 LOC. API-compatible. Preserves every downstream crate consumer.

**Steps:**

1. **Hold the reader lock for one logical read.** Refactor `load_uncompressed_object_impl` to bind the guard once:
   ```rust
   let mut reader = self.reader.lock_or_recover();
   reader.seek(SeekFrom::Start(offset))?;
   reader.read_until(b'\n', &mut header_bytes)?;
   // … all reads in this method use the same `reader` binding
   ```
   Apply the same change to `find_object_header_backwards` (`:2591-2595`) and `recover_from_object_streams` (`:1282-1292`). Kills Race A.

2. **Wrap the four unlocked caches in `Mutex`:**
   - `page_cache: Mutex<HashMap<usize, Object>>`
   - `page_cache_populated: AtomicBool`
   - `structure_tree_cache: Mutex<Option<Option<Arc<StructTreeRoot>>>>`
   - `structure_content_cache: Mutex<Option<HashMap<u32, Vec<OrderedContent>>>>`
   - `erase_regions: Mutex<HashMap<usize, Vec<Rect>>>` (for symmetry; reads are currently safe but writes aren't synchronized with any reader)
   Kills Races B and D.

3. **Flip every read-path method from `&mut self` to `&self`.** After step 2 this is mechanical: the compiler will force every `.caches_field.insert(...)` to go through `.lock_or_recover().insert(...)`. Affected methods at `document.rs:2716, 2752, 2856, 2927, 2956, 3155, 3987, 3998, 4493` and ~60 more. No behavior change.

4. **FFI: change `&mut *handle` → `&*handle`.** In `src/ffi.rs`, ~40 sites: `:253, :277, :302, :327, :351, :843, :877, :1008, :1188, :1357, …`. Kills the language-level UB. One-line mechanical change per site.

5. **Per-call scratch for cycle detection.** Pass `&mut HashSet<ObjectRef>` as a parameter to `load_object`/`load_uncompressed_object_impl` instead of using `self.resolving_stack`. Requires a public API shim (`load_object_with_ctx`) — see Option 3 if we want the full version. For Option 1, alternatively: key the stack by `ThreadId` (`self.resolving_stack: Mutex<HashMap<ThreadId, HashSet<ObjectRef>>>`). Kills Race C but looks hacky.

**Estimated effort:** 1–2 weeks. One engineer. No API break. Ships as v0.3.40.

### Option 2 — Fully immutable `PdfDocument`, `Arc`-shared

Move *every* cache to `OnceLock` / `Mutex` and guarantee the entire public surface is `&self`. Cleaner than Option 1 in the long run. Requires:

- All caches become `OnceLock<T>` where applicable (truly immutable after first compute) or `Mutex<T>`.
- `PdfDocument` becomes `Arc<PdfDocumentInner>` at the API surface; FFI stores `Arc<PdfDocumentInner>` and clones cheaply.
- Bindings can then advertise `Send + Sync + Clone` and drop their managed-side mutex entirely (Go can restore `RWMutex::RLock`, C# can remove any future lock).

**Estimated effort:** 3–4 weeks. Touches `page_cache`'s lazy-populate state machine (the `!self.page_cache_populated && cache_misses >= LAZY_THRESHOLD` logic becomes `OnceLock` + `call_once`). No public API break but changes internal receiver types; a handful of downstream users that hold `&mut PdfDocument` will see the compiler nag.

### Option 3 — Per-read scratch context (`fn foo(&self, ctx: &mut ScratchCtx)`)

Make the parser fully functional: all mutable state (resolving_stack, recursion_depth, temporary buffers) lives on a caller-provided `ScratchCtx` that's cheap to create per call. Caches stay on `PdfDocument` but become read-through.

- Public API break: every `doc.extract_text(0)` becomes `doc.extract_text(0, &mut ScratchCtx::new())` or, via a default wrapper, stays ergonomic but the low-level path is explicit.
- Bindings unchanged (they wrap the ergonomic API).
- Recursion tracking becomes per-call and cycle detection becomes correct-by-construction (no cross-thread pollution).

**Estimated effort:** 2–3 weeks of core refactor, 1 week of binding churn. Semver break → v0.4.0 scope, not 0.3.40.

---

## Recommendation

**Ship Option 1 as v0.3.40.** It is the smallest change that passes `TestConcurrentReads` without the `73e14066` serialization, keeps the public API stable, and lets Python's free-threaded future work.

Keep Option 3 as the v0.4 milestone (it's the right long-term architecture) and treat Option 2 as the incremental path to it: Option 1 → Option 2 → Option 3 composes nicely, each step removing one hack.

**Why not Option 2 directly:** the `page_cache` lazy-populate logic is subtler than a drop-in `OnceLock` replacement (LAZY_THRESHOLD hysteresis, `populate_page_cache` failure fallback at `:3185-3190`). Getting it right at the same time as fixing the reader race multiplies the review surface. Option 1 lands the critical fix; Option 2 is a clean follow-up once the interior-mutability discipline is in place.

---

## Acceptance criteria for the fix

- `TestConcurrentReads` in `go/pdf_oxide_test.go` (20 goroutines × 5 iterations of `PageCount()` + `ExtractText(0)` on one shared `*PdfDocument`) passes 25/25 runs of `go test -race -count=5 -run TestConcurrentReads` **after** reverting the `73e14066` `Lock`/`Unlock` swap back to `RLock`/`RUnlock`.
- Python: a new smoke test spawning 8 threads × 50 `extract_text` calls on a shared `PdfDocument` under a free-threaded build (`python3.13t`) passes without segfault or `code 5`.
- No regressions on the v0.3.39 `cargo test --lib` suite (4810 tests) or the FFI-integration suite (15 tests) — cargo-hack `--each-feature` must still go green.
- `cargo +nightly miri test --lib load_object_concurrent_smoke` passes (new test under miri to catch any remaining UB).
- No observable perf regression on the single-threaded benchmark (`bench/bench_extract_text.rs`): Option 1 adds 4 `Mutex` lock/unlocks per page (fast path: uncontended, ~5 ns each).

---

## Appendix: grep commands to verify the audit yourself

```bash
# Every read-side FFI entry that fabricates &mut *handle
rg -n 'let doc = unsafe \{ &mut \*handle' src/ffi.rs | wc -l    # ~40 hits

# Every &mut self receiver on a pub fn in document.rs
rg -n '^\s*pub fn \w+\(&mut self' src/document.rs | wc -l       # ~60 hits

# Split-lock reader pattern (lock-seek-drop-lock-read)
rg -n 'self\.reader\s*\.\s*lock' src/document.rs                # 5 hits — each must be reviewed

# Fields NOT wrapped in Mutex / Atomic on PdfDocument
sed -n '288,411p' src/document.rs | rg -v 'Mutex|Atomic|//'     # find the bare `HashMap` / `Option` / `bool`

# Every page_cache mutation
rg -n 'page_cache[\.\s]' src/document.rs

# Every structure_tree_cache mutation
rg -n 'structure_tree_cache\s*=' src/document.rs

# Go binding's exclusive-lock workaround (the regression this fix reverses)
git show 73e14066 --stat | head -20
git log --oneline -S 'RLock' -- go/pdf_oxide.go                 # last commit to remove RLock

# Python binding's GIL releases (expect 0 today, expect non-zero after fix)
rg -c 'allow_threads' src/python.rs

# C# binding's unsynchronized native calls (exposure for free-threaded callers)
rg -n 'NativeMethods\.PdfDocument' csharp/PdfOxide/Core/PdfDocument.cs | wc -l
```

Every `path:line` citation in this document was hand-verified against the tree at `release/v0.3.39` (HEAD `1ef072a3` as of this writing).
