---
title: "Workbench implementation / Navigation"
description: "Generated reference for docs/mitase/features/workbench/navigation.yaml"
---

> Generated from `docs/mitase/features/workbench/navigation.yaml`.

## Parsed content

### Schema

- mitase/spec/v1

### Kind

- features

### Namespace

- workbench

### Category

- Workbench implementation

### Features

- **id**: FEAT-WORKBENCH-NAVIGATION-001
  - **title**: Workbench navigation
  - **summary**: Keep HTML navigation, module routes, keyboard handling, and accessibility attributes aligned.
  - **status**: implemented
  - **bindings**:
    - **id**: navigation
      - **role**: implementation
      - **facet**: tooling
      - **responsibility**: Implement browser navigation and accessibility behavior as typed server DTO consumers.
      - **targets**:
        - **id**: html-navigation
          - **adapter**: html
          - **path**: crates/mitase-app-ui/assets/workbench.html
          - **selector**:
            - **kind**: marker
            - **value**: data-page="work"
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
        - **id**: workbench-css
          - **adapter**: declared
          - **path**: crates/mitase-app-ui/assets/workbench.css
          - **selector**:
            - **kind**: file
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
        - **id**: javascript-navigation
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/router.js
          - **selector**:
            - **kind**: symbol
            - **name**: bindRouter
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-015#criterion.detail-deep-link
        - **id**: keyboard-navigation
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/router.js
          - **selector**:
            - **kind**: symbol
            - **name**: bindKeyboardTabs
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
        - **id**: accessibility-attributes
          - **adapter**: html
          - **path**: crates/mitase-app-ui/assets/workbench.html
          - **selector**:
            - **kind**: marker
            - **value**: data-i18n-aria="a11y.main_pages"
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-006#criterion.accessible-navigation
        - **id**: projection-bootstrap
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/main.js
          - **selector**:
            - **kind**: symbol
            - **name**: startWorkbench
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
        - **id**: active-page-render
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/main.js
          - **selector**:
            - **kind**: symbol
            - **name**: render
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
        - **id**: local-specification-filter
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/pages/specifications.js
          - **selector**:
            - **kind**: symbol
            - **name**: candidatesFor
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
        - **id**: busy-status
          - **adapter**: html
          - **path**: crates/mitase-app-ui/assets/workbench.html
          - **selector**:
            - **kind**: marker
            - **value**: data-workbench-status="busy"
          - **claims**:
            - **kind**: satisfies
              - **criterion**: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
      - **owns**:
        - **id**: assets-api-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/api.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-action-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/components/action.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-diagnostic-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/components/diagnostic.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-diff-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/components/diff.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-editor-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/components/editor.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-readiness-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/components/readiness.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-target-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/components/target.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-js-i18n-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/i18n.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-i18n-script
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/i18n.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-main-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/main.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-router-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/router.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-state-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/state.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-diagnostics-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/pages/diagnostics.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-readiness-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/pages/readiness.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-scope-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/pages/scope.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-settings-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/pages/settings.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-specifications-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/pages/specifications.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: assets-page-work-module
          - **adapter**: javascript
          - **path**: crates/mitase-app-ui/assets/js/pages/work.js
          - **selector**:
            - **kind**: module
            - **name**: *
        - **id**: html-file
          - **adapter**: html
          - **path**: crates/mitase-app-ui/assets/workbench.html
          - **selector**:
            - **kind**: file
        - **id**: declared-html-file
          - **adapter**: declared
          - **path**: crates/mitase-app-ui/assets/workbench.html
          - **selector**:
            - **kind**: file
        - **id**: locale-en-file
          - **adapter**: declared
          - **path**: crates/mitase-app-ui/assets/locales/en.json
          - **selector**:
            - **kind**: file
        - **id**: locale-ja-file
          - **adapter**: declared
          - **path**: crates/mitase-app-ui/assets/locales/ja.json
          - **selector**:
            - **kind**: file

## Source YAML

```yaml
schema: mitase/spec/v1
kind: features
namespace: workbench
category: Workbench implementation
features:
- id: FEAT-WORKBENCH-NAVIGATION-001
  title: Workbench navigation
  summary: Keep HTML navigation, module routes, keyboard handling, and accessibility attributes aligned.
  status: implemented
  bindings:
  - id: navigation
    role: implementation
    facet: tooling
    responsibility: Implement browser navigation and accessibility behavior as typed server DTO consumers.
    targets:
    - id: html-navigation
      adapter: html
      path: crates/mitase-app-ui/assets/workbench.html
      selector:
        kind: marker
        value: data-page="work"
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
    - id: workbench-css
      adapter: declared
      path: crates/mitase-app-ui/assets/workbench.css
      selector:
        kind: file
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
    - id: javascript-navigation
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/router.js
      selector:
        kind: symbol
        name: bindRouter
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
      - kind: satisfies
        criterion: REQ-WORKBENCH-015#criterion.detail-deep-link
    - id: keyboard-navigation
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/router.js
      selector:
        kind: symbol
        name: bindKeyboardTabs
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
    - id: accessibility-attributes
      adapter: html
      path: crates/mitase-app-ui/assets/workbench.html
      selector:
        kind: marker
        value: data-i18n-aria="a11y.main_pages"
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-006#criterion.accessible-navigation
    - id: projection-bootstrap
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/main.js
      selector:
        kind: symbol
        name: startWorkbench
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
    - id: active-page-render
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/main.js
      selector:
        kind: symbol
        name: render
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
    - id: local-specification-filter
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/pages/specifications.js
      selector:
        kind: symbol
        name: candidatesFor
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
    - id: busy-status
      adapter: html
      path: crates/mitase-app-ui/assets/workbench.html
      selector:
        kind: marker
        value: data-workbench-status="busy"
      claims:
      - kind: satisfies
        criterion: REQ-WORKBENCH-012#criterion.responsive-browser-interactions
    owns:
    - id: assets-api-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/api.js
      selector:
        kind: module
        name: '*'
    - id: assets-action-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/components/action.js
      selector:
        kind: module
        name: '*'
    - id: assets-diagnostic-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/components/diagnostic.js
      selector:
        kind: module
        name: '*'
    - id: assets-diff-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/components/diff.js
      selector:
        kind: module
        name: '*'
    - id: assets-editor-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/components/editor.js
      selector:
        kind: module
        name: '*'
    - id: assets-readiness-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/components/readiness.js
      selector:
        kind: module
        name: '*'
    - id: assets-target-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/components/target.js
      selector:
        kind: module
        name: '*'
    - id: assets-js-i18n-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/i18n.js
      selector:
        kind: module
        name: '*'
    - id: assets-i18n-script
      adapter: javascript
      path: crates/mitase-app-ui/assets/i18n.js
      selector:
        kind: module
        name: '*'
    - id: assets-main-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/main.js
      selector:
        kind: module
        name: '*'
    - id: assets-router-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/router.js
      selector:
        kind: module
        name: '*'
    - id: assets-state-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/state.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-diagnostics-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/pages/diagnostics.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-readiness-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/pages/readiness.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-scope-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/pages/scope.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-settings-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/pages/settings.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-specifications-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/pages/specifications.js
      selector:
        kind: module
        name: '*'
    - id: assets-page-work-module
      adapter: javascript
      path: crates/mitase-app-ui/assets/js/pages/work.js
      selector:
        kind: module
        name: '*'
    - id: html-file
      adapter: html
      path: crates/mitase-app-ui/assets/workbench.html
      selector:
        kind: file
    - id: declared-html-file
      adapter: declared
      path: crates/mitase-app-ui/assets/workbench.html
      selector:
        kind: file
    - id: locale-en-file
      adapter: declared
      path: crates/mitase-app-ui/assets/locales/en.json
      selector:
        kind: file
    - id: locale-ja-file
      adapter: declared
      path: crates/mitase-app-ui/assets/locales/ja.json
      selector:
        kind: file
```
