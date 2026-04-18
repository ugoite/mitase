// FEAT-VSCODE-001

const fs = require('node:fs/promises');
const path = require('node:path');

const vscode = require('vscode');

const {
  loadDiagnostics,
  loadSpecModel,
  lookupTrace,
  openTargetsForSpecId,
  specIdFromText
} = require('./model');

class SyuContextTreeProvider {
  constructor() {
    this._onDidChangeTreeData = new vscode.EventEmitter();
    this.onDidChangeTreeData = this._onDidChangeTreeData.event;
    this.rootItems = [createMessageNode('Open a file in a syu workspace to inspect trace links.')]
  }

  setMessage(message) {
    this.rootItems = [createMessageNode(message)];
    this._onDidChangeTreeData.fire();
  }

  setTraceContext(workspaceRoot, traceContext) {
    const items = [
      createMessageNode(
        traceContext.symbol
          ? `Status: ${traceContext.status} (${traceContext.file} :: ${traceContext.symbol})`
          : `Status: ${traceContext.status} (${traceContext.file})`
      )
    ];

    if (traceContext.matchedOwners.length > 0) {
      items.push(
        createSectionNode(
          'Matched owners',
          traceContext.matchedOwners.map((owner) => createOwnerNode(workspaceRoot, owner))
        )
      );
    }

    if (traceContext.fileOnlyOwners.length > 0) {
      items.push(
        createSectionNode(
          'File-only owners',
          traceContext.fileOnlyOwners.map((owner) => createOwnerNode(workspaceRoot, owner))
        )
      );
    }

    for (const [label, values] of [
      ['Requirements', traceContext.requirements],
      ['Features', traceContext.features],
      ['Policies', traceContext.policies],
      ['Philosophies', traceContext.philosophies]
    ]) {
      if (values.length === 0) {
        continue;
      }

      items.push(
        createSectionNode(
          label,
          values.map((item) => createSpecItemNode(workspaceRoot, item))
        )
      );
    }

    if (
      traceContext.status === 'unowned' &&
      traceContext.matchedOwners.length === 0 &&
      traceContext.fileOnlyOwners.length === 0
    ) {
      items.push(
        createMessageNode(
          'No requirement or feature traces matched this file. Add a trace entry and refresh diagnostics.'
        )
      );
    }

    this.rootItems = items;
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(node) {
    const item = new vscode.TreeItem(
      node.label,
      node.children?.length
        ? vscode.TreeItemCollapsibleState.Expanded
        : vscode.TreeItemCollapsibleState.None
    );

    if (node.description) {
      item.description = node.description;
    }
    if (node.tooltip) {
      item.tooltip = node.tooltip;
    }
    if (node.command) {
      item.command = node.command;
    }

    return item;
  }

  getChildren(node) {
    return node?.children || this.rootItems;
  }
}

function createMessageNode(label) {
  return { label };
}

function createSectionNode(label, children) {
  return { label, children };
}

function createOwnerNode(workspaceRoot, owner) {
  const matchedBy =
    owner.matchMode === 'symbol'
      ? `symbol ${owner.matchedSymbol}`
      : owner.matchMode === 'wildcard'
        ? 'wildcard *'
        : 'file';

  return {
    label: `${owner.kind} ${owner.id}`,
    description: `${owner.language} ${owner.traceRole} (${matchedBy})`,
    tooltip: owner.title,
    command: {
      command: 'syu.openResolvedTarget',
      title: 'Open spec item',
      arguments: [
        {
          path: path.join(workspaceRoot, owner.documentPath.split('/').join(path.sep)),
          searchText: `id: ${owner.id}`
        }
      ]
    }
  };
}

function createSpecItemNode(workspaceRoot, item) {
  return {
    label: item.id,
    description: item.title,
    tooltip: item.documentPath,
    command: {
      command: 'syu.openResolvedTarget',
      title: 'Open spec item',
      arguments: [
        {
          path: path.join(workspaceRoot, item.documentPath.split('/').join(path.sep)),
          searchText: `id: ${item.id}`
        }
      ]
    }
  };
}

function fileExists(filePath) {
  return fs.access(filePath).then(
    () => true,
    () => false
  );
}

async function isSyuWorkspace(folder) {
  const workspaceRoot = folder.uri.fsPath;
  return (
    (await fileExists(path.join(workspaceRoot, 'syu.yaml'))) ||
    (await fileExists(path.join(workspaceRoot, 'docs/syu/features/features.yaml')))
  );
}

async function syuWorkspaceFolders() {
  const folders = vscode.workspace.workspaceFolders || [];
  const resolved = await Promise.all(
    folders.map(async (folder) => ((await isSyuWorkspace(folder)) ? folder : null))
  );
  return resolved.filter(Boolean);
}

function getBinaryPath(folder) {
  return vscode.workspace.getConfiguration('syu', folder.uri).get('binaryPath', 'syu');
}

function getAutoRefreshDiagnostics(folder) {
  return vscode.workspace
    .getConfiguration('syu', folder.uri)
    .get('autoRefreshDiagnostics', true);
}

async function ensureModel(folder, modelCache) {
  const workspaceRoot = folder.uri.fsPath;
  const cached = modelCache.get(workspaceRoot);
  if (cached && !cached.dirty) {
    return cached.model;
  }

  const model = await loadSpecModel(workspaceRoot);
  modelCache.set(workspaceRoot, { model, dirty: false });
  return model;
}

function invalidateModel(folder, modelCache) {
  const entry = modelCache.get(folder.uri.fsPath);
  if (entry) {
    entry.dirty = true;
  }
}

async function refreshDiagnostics(modelCache, collection) {
  const folders = await syuWorkspaceFolders();
  const diagnosticMap = new Map();

  for (const folder of folders) {
    try {
      const model = await ensureModel(folder, modelCache);
      const records = await loadDiagnostics({
        workspaceRoot: folder.uri.fsPath,
        binaryPath: getBinaryPath(folder),
        model
      });

      for (const record of records) {
        const uri = vscode.Uri.file(record.path);
        const existing = diagnosticMap.get(uri.toString()) || { uri, diagnostics: [] };
        existing.diagnostics.push(toDiagnostic(record));
        diagnosticMap.set(uri.toString(), existing);
      }
    } catch (error) {
      void vscode.window.showErrorMessage(String(error.message || error));
    }
  }

  collection.clear();
  for (const { uri, diagnostics } of diagnosticMap.values()) {
    collection.set(uri, diagnostics);
  }
}

function toDiagnostic(record) {
  const line = Math.max(record.range.line, 0);
  const startCharacter = Math.max(record.range.startCharacter, 0);
  const endCharacter = Math.max(record.range.endCharacter, startCharacter + 1);
  const diagnostic = new vscode.Diagnostic(
    new vscode.Range(line, startCharacter, line, endCharacter),
    record.message,
    record.severity === 'warning'
      ? vscode.DiagnosticSeverity.Warning
      : vscode.DiagnosticSeverity.Error
  );
  diagnostic.code = record.code;
  diagnostic.source = 'syu';
  return diagnostic;
}

function activeSpecId(editor) {
  if (!editor) {
    return null;
  }

  const selected = editor.document.getText(editor.selection);
  if (selected) {
    return specIdFromText(selected);
  }

  const range = editor.document.getWordRangeAtPosition(
    editor.selection.active,
    /(?:PHIL|POL|REQ|FEAT)-[A-Z0-9-]+/
  );
  if (!range) {
    return null;
  }

  return specIdFromText(editor.document.getText(range));
}

function activeSymbol(editor) {
  if (!editor) {
    return null;
  }

  const selected = editor.document.getText(editor.selection).trim();
  if (selected && /^[A-Za-z_][A-Za-z0-9_]*$/u.test(selected)) {
    return selected;
  }

  return null;
}

async function pickSpecItem(model, preferredId) {
  if (preferredId && model.byId.has(preferredId)) {
    return model.byId.get(preferredId);
  }

  const choices = [...model.byId.values()]
    .sort((left, right) => left.id.localeCompare(right.id))
    .map((item) => ({
      label: item.id,
      description: `${item.kind} — ${item.title}`,
      item
    }));

  const picked = await vscode.window.showQuickPick(choices, {
    placeHolder: 'Pick a philosophy, policy, requirement, or feature'
  });
  return picked?.item || null;
}

async function revealSearchText(editor, searchText) {
  if (!searchText) {
    return;
  }

  const contents = editor.document.getText();
  const offset = contents.indexOf(searchText);
  if (offset === -1) {
    return;
  }

  const start = editor.document.positionAt(offset);
  const end = editor.document.positionAt(offset + searchText.length);
  editor.selection = new vscode.Selection(start, end);
  editor.revealRange(new vscode.Range(start, end), vscode.TextEditorRevealType.InCenter);
}

async function openResolvedTarget(target) {
  const document = await vscode.workspace.openTextDocument(vscode.Uri.file(target.path));
  const editor = await vscode.window.showTextDocument(document, { preview: false });
  await revealSearchText(editor, target.searchText);
}

async function refreshContextForActiveEditor(modelCache, treeProvider) {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.isUntitled) {
    treeProvider.setMessage('Open a saved file inside a syu workspace to inspect trace links.');
    return;
  }

  const folder = vscode.workspace.getWorkspaceFolder(editor.document.uri);
  if (!folder || !(await isSyuWorkspace(folder))) {
    treeProvider.setMessage('The active file is not inside a syu workspace.');
    return;
  }

  try {
    const model = await ensureModel(folder, modelCache);
    treeProvider.setTraceContext(
      folder.uri.fsPath,
      lookupTrace(model, editor.document.uri.fsPath, null)
    );
  } catch (error) {
    treeProvider.setMessage(String(error.message || error));
  }
}

function registerCommands(context, modelCache, treeProvider, collection) {
  context.subscriptions.push(
    vscode.commands.registerCommand('syu.openResolvedTarget', openResolvedTarget)
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('syu.refreshDiagnostics', async () => {
      await refreshDiagnostics(modelCache, collection);
      await refreshContextForActiveEditor(modelCache, treeProvider);
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('syu.showTraceForActiveFile', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        await vscode.window.showInformationMessage('Open a file before tracing it with syu.');
        return;
      }

      const folder = vscode.workspace.getWorkspaceFolder(editor.document.uri);
      if (!folder || !(await isSyuWorkspace(folder))) {
        await vscode.window.showInformationMessage('The active file is not inside a syu workspace.');
        return;
      }

      const model = await ensureModel(folder, modelCache);
      const symbol = activeSymbol(editor);
      const traceContext = lookupTrace(model, editor.document.uri.fsPath, symbol);
      treeProvider.setTraceContext(folder.uri.fsPath, traceContext);
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('syu.openSpecItemById', async () => {
      const folder =
        vscode.window.activeTextEditor &&
        vscode.workspace.getWorkspaceFolder(vscode.window.activeTextEditor.document.uri);
      const workspaceFolder =
        folder && (await isSyuWorkspace(folder))
          ? folder
          : (await syuWorkspaceFolders())[0];

      if (!workspaceFolder) {
        await vscode.window.showInformationMessage('No syu workspace is open.');
        return;
      }

      const model = await ensureModel(workspaceFolder, modelCache);
      const item = await pickSpecItem(model, activeSpecId(vscode.window.activeTextEditor));
      if (!item) {
        return;
      }

      await openResolvedTarget({
        path: path.join(workspaceFolder.uri.fsPath, item.documentPath.split('/').join(path.sep)),
        searchText: `id: ${item.id}`
      });
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('syu.showRelatedFilesForSpecId', async () => {
      const folder =
        vscode.window.activeTextEditor &&
        vscode.workspace.getWorkspaceFolder(vscode.window.activeTextEditor.document.uri);
      const workspaceFolder =
        folder && (await isSyuWorkspace(folder))
          ? folder
          : (await syuWorkspaceFolders())[0];

      if (!workspaceFolder) {
        await vscode.window.showInformationMessage('No syu workspace is open.');
        return;
      }

      const model = await ensureModel(workspaceFolder, modelCache);
      const item = await pickSpecItem(model, activeSpecId(vscode.window.activeTextEditor));
      if (!item) {
        return;
      }

      const targets = openTargetsForSpecId(model, item.id).map((target) => ({
        label: target.label,
        description: target.description,
        target
      }));
      const picked = await vscode.window.showQuickPick(targets, {
        placeHolder: `Open the document or traced files for ${item.id}`
      });

      if (picked) {
        await openResolvedTarget(picked.target);
      }
    })
  );
}

function activate(context) {
  const modelCache = new Map();
  const diagnostics = vscode.languages.createDiagnosticCollection('syu');
  const treeProvider = new SyuContextTreeProvider();
  const treeView = vscode.window.createTreeView('syuContext', {
    treeDataProvider: treeProvider
  });

  context.subscriptions.push(diagnostics, treeView);

  registerCommands(context, modelCache, treeProvider, diagnostics);

  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument(async (document) => {
      const folder = vscode.workspace.getWorkspaceFolder(document.uri);
      if (!folder || !(await isSyuWorkspace(folder))) {
        return;
      }

      invalidateModel(folder, modelCache);
      if (getAutoRefreshDiagnostics(folder)) {
        await refreshDiagnostics(modelCache, diagnostics);
      }
      await refreshContextForActiveEditor(modelCache, treeProvider);
    })
  );

  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor(async () => {
      await refreshContextForActiveEditor(modelCache, treeProvider);
    })
  );

  context.subscriptions.push(
    vscode.workspace.onDidChangeWorkspaceFolders(async () => {
      modelCache.clear();
      await refreshDiagnostics(modelCache, diagnostics);
      await refreshContextForActiveEditor(modelCache, treeProvider);
    })
  );

  void refreshDiagnostics(modelCache, diagnostics);
  void refreshContextForActiveEditor(modelCache, treeProvider);
}

function deactivate() {}

module.exports = {
  activate,
  deactivate
}
