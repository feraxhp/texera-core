# Texera (`texera-core`)

> **Texera** *(union between `text` + `tessera`)*.

This is an actual port for the loved codemirror6 editor (`@codemirror/state`), moved entirely to Rust, with type-safety and high performance.

The editor state is as close as possible to the [CodeMirror 6](https://codemirror.net/) implementation,
all credits for the base design go to `@Marijn Haverbeke`.

> [!IMPORTANT]
> The work made by `@Marijn Haverbeke` is amazing, so much,
> that I decided to not recreate the wheel, but re-implement it in Rust.

---

## Features & Architecture

- **Document Rope (`Text` & `Line`)**: Persistent $O(\log N)$ text buffer backed by `ropey`, with 1-based line metadata, character-offset slicing, and line-break normalization.
- **Changes & Operational Transform (`ChangeDesc`, `ChangeSet`, `ch!`)**: Position mapping (`MapMode`, `Assoc`), change composition (`Composable`), operational transform (`Mapable`), and undo inversion (`invert`).
- **Multi-Cursor Selections (`Selection`, `SelectionRange`)**: Normalizes overlapping ranges automatically, tracks directional cursors (`anchor` / `head`) and goal columns, and supports `State::change_by_range` and `State::replace_selection`.
- **Type-Safe Extension System (`Extension`, `Facet`, `FacetDef`, `StateField`, `Compartment`, `Priority`)**:
  - Static and dynamic facets with topological dependency sorting (`dep::doc()`, `dep::selection()`, `dep::field::<F>()`, `dep::facet::<F>()`).
  - Persistent `StateField`s that can also feed facets via `StateField::provide`.
  - Dynamic runtime reconfiguration via `Compartment`, `StateEffect::reconfigure`, and `StateEffect::append_config`.
- **Transactional Pipeline (`State`, `Transaction`, `TransactionSpec`, `TransactionResult`, `tr!`)**:
  - Multi-spec `State::update` (concurrent or `sequential: true`), `TransactionFilter`, `ChangeFilter`, `TransactionExtender`, typed `Annotation`s, `UserEvent` hierarchy matching, and `StateEffect`s.
- **Multi-Layer Range Sets (`RangeSet`, `RangeSetBuilder`, `RangeValue`)**: Chunked B-tree-like storage of document decorations/markers with fast diffing (`RangeSet::compare`) and cursor merging.
- **Unicode & Categorization (`CharCategorizer`, `WordChars`, `find_cluster_break`, `count_column`, `find_column`)**: Grapheme-cluster navigation, visual tab-stop columns, and $O(N)$ `State::word_at`.

## Quick Start

```rust
use texera_core::{
    AllowMultipleSelections, Composable, Extension, Selection, State, TabSize, tr,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create an initial state with extensions
    let state = State::from("fn main() {\n    \n}").with_extensions(Extension::from([
        AllowMultipleSelections(true).into(),
        TabSize(4).into(),
    ]));

    // 2. Dispatch a transaction using the `tr!` macro
    let res = state.update(tr!({
        changes: { from: 16, to: 16, insert: "println!(\"Hello, Texera!\");" },
        selection: Selection::cursor(43),
        user_event: "input.type",
        scroll_into_view: true,
    }))?;

    assert!(res.doc_changed());
    assert!(res.is_user_event("input"));
    assert_eq!(
        res.state_ref().doc().to_string(),
        "fn main() {\n    println!(\"Hello, Texera!\");\n}"
    );

    // 3. Invert changes for Undo
    let undo = res.inverted_changes();
    let restored = undo.apply(res.state_ref().doc().clone())?;
    assert_eq!(restored.to_string(), "fn main() {\n    \n}");

    Ok(())
}
```

## Examples

Runnable examples demonstrating common workflows are located in the [`examples/`](examples) directory:

```bash
# 1. Basic editing, line inspection, and undo inversion
cargo run --example 01_basic_editing

# 2. Multi-cursor editing, batch insertion, and change_by_range
cargo run --example 02_multi_cursor

# 3. Custom StateField, multi-value Facet, and Compartment reconfiguration
cargo run --example 03_custom_extension

# 4. RangeSets, document decorations, and O(1) diffing
cargo run --example 04_decorations_and_diffing
```

## Documentation

Build and browse the full API documentation and runnable examples with:

```bash
cargo doc --no-deps --open
```

Run the test suite (including all Rustdoc examples):

```bash
cargo test
```

## License

~~~
// Copyright (C) 2026 feraxhp
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
// 
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
~~~
