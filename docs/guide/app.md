# syu app browser UI guide

<!-- FEAT-APP-001, FEAT-DOCS-001 -->

`syu app .` serves a local browser UI for exploring your workspace specification interactively.

```bash
syu app .
# → Serving on http://127.0.0.1:3000  (Ctrl-C to stop)
```

Open the printed URL in any browser.

---

## Layout overview

The UI is divided into three areas:

```
┌─────────────────────────────────────────────────────────────┐
│  Header — section tabs (Philosophy | Policies | …)          │
├─────────────────┬───────────────────────────────────────────┤
│  Left sidebar   │  Main content area                        │
│  • Workspace    │  • Item detail (title, links, traces)     │
│    summary      │                                           │
│  • Section      │                                           │
│    overview     ├───────────────────────────────────────────┤
│  • Document     │  Validation panel                         │
│    list         │  • Issues, rule codes, severities         │
└─────────────────┴───────────────────────────────────────────┘
```

---

## Header — section tabs

Four tabs correspond to the four spec layers:

| Tab | What it contains |
|-----|-----------------|
| **philosophy** | Core values and design principles — the *why* |
| **policies** | Repository-wide rules that operationalise philosophy — the *how* |
| **requirements** | Specific obligations with test traces |
| **features** | Implemented capabilities with implementation traces |

Click a tab to switch the content area and the document list in the sidebar.

---

## Left sidebar

### Workspace summary panel

The top card shows the workspace root path, the spec root path, and three metrics:

- **issues** — total open validation issues
- **requirement traces** — `validated / declared` (how many declared test traces were confirmed on disk)
- **feature traces** — `validated / declared` (same for implementation traces)

### Sections panel

The four layers are shown as cards with bar charts indicating relative item counts. Click any card to jump to that section.

### Document list

Below the sections panel, the documents for the active section are listed. Click a document name to load its items in the main content area.

---

## Main content area

### Item detail

Selecting an item shows:

- **ID and title** — the stable identifier and human-readable name
- **Status badge** — `planned`, `implemented`, or `deprecated`
- **Summary / description** — the prose from the YAML
- **Links panel** — the upstream and downstream relationships (e.g. which requirements a feature satisfies; which policies a requirement enforces)
- **Traces panel** — the declared test and implementation traces, with file path and symbol name

### Clicking links

Every linked ID in the Links panel is a button. Clicking it jumps directly to that item, even if it is in a different section or document.

---

## Validation panel

The validation panel appears at the bottom of the page and lists all issues found during the last `syu validate` run that was used to generate the app data.

Each row shows:

| Column | Meaning |
|--------|---------|
| **Code** | The `SYU-[genre]-[content]-[NNN]` rule code |
| **Severity** | `error` or `warning` |
| **Subject** | The spec item ID the issue refers to (if any) |
| **Message** | A short human-readable description |

### Filtering issues

The panel header includes a severity filter. Select **errors only** to hide warnings, or **all** to see everything.

### Jumping to the affected item

Click the subject ID in any validation issue row to jump directly to that spec item in the main content area.

---

## Starting options

### Default address and port

By default `syu app` binds to `127.0.0.1:3000`. Override on the command line:

```bash
syu app . --port 8080
syu app . --bind 0.0.0.0 --port 8080
```

### Persistent config

Set defaults in `syu.yaml` so you do not have to pass flags every time:

```yaml
app:
  bind: 127.0.0.1
  port: 3000
```

CLI flags always override the config values. See the [configuration guide](./configuration.md#appbind) for the full reference.

---

## Refreshing the data

The browser UI is a snapshot of the workspace at the time `syu app` was started. To see changes you have made to spec files:

1. Stop the server (`Ctrl-C`)
2. Re-run `syu app .`

Live reload is not supported yet — the server must be restarted to pick up changes.

---

## What's next?

- Read the [getting-started guide](./getting-started.md) to build the spec files that this UI displays
- Follow the [tutorial](./tutorial.md) for a complete worked example
- Run `syu validate .` from the CLI for the same validation results in a terminal-friendly format
