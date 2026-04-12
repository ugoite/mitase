# syu app browser UI guide

<!-- FEAT-APP-001, FEAT-DOCS-001 -->

`syu app .` serves a local browser UI for exploring your workspace specification interactively.

```bash
syu app .
# → syu app listening on http://127.0.0.1:3000
# → syu app ready: http://127.0.0.1:3000
# → Open http://127.0.0.1:3000 in your browser.
# → Press Ctrl-C to stop.
```

Open the printed URL in any browser, then press `Ctrl-C` in the terminal when
you are done.

---

## Layout overview

The UI is divided into three areas:

![Annotated overview of the syu app layout](/img/app-guide-overview.png)

The screenshot above highlights the top tabs, the left sidebar, the main item
detail panel, and the validation summary that stays visible while you browse.

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

When validation passes cleanly, the sidebar gives you a quick confidence check
before you start exploring relationships and trace details:

![Annotated clean workspace state in the syu app](/img/app-guide-passing.png)

### Sections panel

The four layers are shown as cards with bar charts indicating relative item counts. Click any card to jump to that section.

### Document list

Below the sections panel, the documents for the active section are listed. Click a document name to load its items in the main content area.

---

## Main content area

### Item detail

Selecting an item shows:

- **ID and title** — the stable identifier and human-readable name
- **Status badge** — the item's YAML `status:` field (for example `planned` or `implemented`)
- **Summary / description** — the prose from the YAML
- **Links panel** — the upstream and downstream relationships (e.g. which requirements a feature satisfies; which policies a requirement enforces)
- **Traces panel** — the declared test and implementation traces, with file path and symbol name

![Annotated feature detail view with linked requirements and implementation traces](/img/app-guide-detail.png)

### Clicking links

Every linked ID in the Links panel is a button. Clicking it jumps directly to that item, even if it is in a different section or document.

For planned requirements and planned features, the detail panel also shows
placeholder guidance instead of an empty traces section so contributors know what
evidence still needs to be added before the item becomes implemented.

---

## Validation panel

The validation panel appears at the bottom of the page and lists the current
issues from the workspace snapshot that `syu app` loaded itself. You do not
need to run `syu validate` first: the app computes the same validation snapshot
when it starts, refreshes it while the tab stays visible, and catches up again
when you return to the tab after spec changes.

Each row shows:

| Column | Meaning |
|--------|---------|
| **Code** | The `SYU-[genre]-[content]-[NNN]` rule code |
| **Severity** | `error` or `warning` |
| **Subject** | The spec item ID the issue refers to (if any) |
| **Message** | A short human-readable description |

![Annotated validation workflow in the syu app](/img/app-guide-validation.png)

Click any issue row to load the selected issue detail on the right. When the
subject maps to a known item in the workspace, the detail panel shows a
`View <ID>` button that jumps directly to the affected philosophy, policy,
requirement, or feature.

### Jumping to the affected item

Use the validation flow in this order:

1. Click the issue row that best matches what you are investigating.
2. Read the selected issue message, location, suggestion, and rule reference.
3. Click `View <ID>` to open the affected spec item in the main content area.

---

## Common workflows

### I want to see which requirements still need tests

1. Open the **Requirements** tab.
2. Pick the requirement document you are working on from the sidebar.
3. Open a requirement with `status: planned`.
4. In the detail pane, look for the planned placeholder under **Tests**.
5. Add the missing trace entries in YAML, then keep the app tab visible or
   switch back to it so the browser refresh flow loads the updated snapshot.
   Run `syu validate .` separately only if you also want the same validation
   details in a terminal.

### I got a validation error - how do I find the affected item?

1. Scroll to **Current issues**.
2. Click the issue row you want to inspect.
3. Read the selected issue's suggestion and rule reference.
4. Click `View <ID>` when it appears to jump to the affected item.
5. Use the source panel below to compare the checked-in YAML with the issue you
   are fixing.

### I want to see what a feature implements

1. Open the **Features** tab.
2. Choose the relevant feature document from the sidebar.
3. Open the feature item you want to inspect.
4. Read **Linked requirements** to see what the feature promises to satisfy.
5. Read **Implementations** to see the traced files, symbols, and optional
   `doc_contains` evidence that anchor the feature to real repository content.

---

## Starting options

### Default address and port

By default `syu app` binds to `127.0.0.1:3000`. Override on the command line:

```bash
syu app . --port 8080
syu app . --bind 0.0.0.0 --port 8080
```

Keep a loopback bind such as `127.0.0.1` or `::1` for normal local use.
Binding to `0.0.0.0`, `::`, or any other non-loopback address makes the app
reachable from other machines on the network, so only do that deliberately
when remote access is part of your setup.

### Persistent config

Set defaults in `syu.yaml` so you do not have to pass flags every time:

```yaml
app:
  bind: 127.0.0.1
  port: 3000
```

CLI flags always override the config values. See the [configuration guide](./configuration.md#appbind) for the full reference.

---

## Health checks and readiness

Once the app has loaded the workspace successfully, `GET /health` returns:

```json
{"status":"ok","version":"0.0.1-alpha.7"}
```

The startup log also prints a distinct ready line:

```text
syu app ready: http://127.0.0.1:3000
```

Use that line or the `/health` endpoint in CI scripts, container probes, or
process supervisors instead of a fixed `sleep`.

---

## Refreshing the data

The browser UI polls the `syu app` server's `/api/version` endpoint about every
two seconds to detect spec snapshot changes. When a YAML file under the spec
root changes, the browser reloads the workspace data without requiring a server
restart.

The refresh banner appears briefly while the UI swaps in the new snapshot.

Two caveats still matter:

1. The browser keeps your current deep link when possible, but if the selected
   item disappears you may land on the first available item in that section.
2. The polling loop pauses while the tab is hidden, so a background tab may stay
   stale until you focus it again or reload the page.
3. Changes outside the spec snapshot flow, such as app server flags or
   configuration in `syu.yaml`, still require restarting `syu app`.

---

## What's next?

- Read the [getting-started guide](./getting-started.md) to build the spec files that this UI displays
- Follow the [tutorial](./tutorial.md) for a complete worked example
- Run `syu validate .` from the CLI for the same validation results in a terminal-friendly format
