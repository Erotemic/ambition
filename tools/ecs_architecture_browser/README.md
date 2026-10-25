# ECS architecture browser

**Status:** active, read-only development tool.

The browser lets you inspect the generated Bevy ECS inventory in one HTML page.
It consumes `.agent/ecs_inventory/` and `.agent/index/`. It does not create a
second ECS index or change the game. The build step adds links, rankings, and
source excerpts to the generated data.

## Open the browser

From the Ambition repository root:

```bash
python3 scripts/ecs_browser.py --open
```

Or build the file without opening a browser:

```bash
python3 scripts/ecs_browser.py
```

Open `.agent/ecs_browser/index.html`. The page has all data, CSS, and JavaScript
inside it. It works without a web server, internet access, Node.js, or a running
game. You can also copy the HTML file to another computer. In that case, the
local source-file links will not work, but source excerpts and source locations
stay available.

The generated file is not a runtime asset. It is under `.agent/` and is ignored
by Git. Commit the generator, UI source, tests, and tool documentation instead.

## If the index is missing or old

The default build stops when the generated commit does not match the checkout.
This prevents reports about an old code tree from looking current.

Regenerate the normal agent index:

```bash
python3 scripts/generate_agent_index.py
python3 scripts/ecs_browser.py --open
```

Index generation uses the existing ECS inventory generator and its documented
tree-sitter environment. See
[`docs/tools/ecs-inventory-tool.md`](../../docs/tools/ecs-inventory-tool.md)
for those requirements.

To inspect an older generated snapshot on purpose:

```bash
python3 scripts/ecs_browser.py --allow-stale --open
```

The page shows a warning when the checkout and index commits differ.
Use `--output <file.html>` to write another HTML file. Local source links assume
the default path in `.agent/ecs_browser/`.

## What to inspect

- **Overview:** large crate/system counts, query-heavy systems, component-use
  frequency, and resource access. Each bar opens the matching record.
- **All records:** search the indexed record types together. Press `/` to focus
  the search field.
- **Systems:** query parameters, known component reads/writes and filters,
  resource and message access, registration evidence, and source snippets.
- **Components and resources:** reverse links to functions that mention them.
- **Messages and channels:** derive records and message-bus read/write evidence.
- **Spawn sites:** source expressions, name labels, and inferred component sets.
- **Crates:** local ECS counts and manifest-declared internal dependencies.
- **Plugins and registrations:** registration contexts, schedules, sets, and
  function-name matches.

The tables support crate filters, sorting, paging, and CSV export. Select a row
or a chart bar to inspect its details. Select linked records to move through the
source evidence. Copy `path:line` from the detail panel to find the source in an
editor.

## Meaning and limits

All metrics are **static**:

- A system-like function is a source candidate. Not all such functions are
  scheduled. A registration link uses a name match; no link does not prove
  that a system is unused.
- A query count is the number of indexed Bevy `Query` parameters. It is not the
  number of entities matched, work done, or execution time.
- Component-use rankings count distinct indexed functions that mention a known
  `Component` type. Parsed query references are hints. Duplicate names, aliases,
  indirect wrappers, and external Bevy types can remain unresolved.
- A spawn site is source evidence. It is not a live Bevy entity, a complete
  archetype, or a count of active instances.
- Declared Cargo dependencies can be optional. They are not proof that a given
  app build links a crate.
- Runtime frame timings, ECS schedule order, live entity populations, and
  rollback/session state need explicit runtime instrumentation.

For runtime inspection later, produce a separately versioned **read-only ECS
snapshot** from a running or headless app. Include session/tick identity and
stable semantic IDs where available. Keep it distinct from this source index.
Do not treat raw Bevy entity handles as durable identity.

## Tests

```bash
python3 -m unittest scripts.tests.test_ecs_browser -v
node --check tools/ecs_architecture_browser/web/app.js
python3 scripts/ecs_browser.py
```

Node is needed only for the optional JavaScript syntax check, not for normal
use. The Python tests use the standard library and small synthetic indexes.
