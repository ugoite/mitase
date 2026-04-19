// FEAT-VSCODE-001
// REQ-CORE-022

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');

const {
  loadSpecModel,
  lookupTrace,
  normalizeRelativePath,
  openTargetsForSpecId,
  resolveWorkspaceContext,
  resolveIssueTarget
} = require('../src/model');

function fixtureRoot(name) {
  return path.resolve(__dirname, '../../../tests/fixtures/workspaces', name);
}

async function createCustomSpecRootWorkspace() {
  const workspaceRoot = await fs.mkdtemp(path.join(os.tmpdir(), 'syu-vscode-spec-root-'));
  const specRoot = path.join(workspaceRoot, 'spec', 'contracts');

  await fs.mkdir(path.join(specRoot, 'philosophy'), { recursive: true });
  await fs.mkdir(path.join(specRoot, 'policies'), { recursive: true });
  await fs.mkdir(path.join(specRoot, 'requirements'), { recursive: true });
  await fs.mkdir(path.join(specRoot, 'features'), { recursive: true });
  await fs.writeFile(
    path.join(workspaceRoot, 'syu.yaml'),
    'version: 0.0.1-alpha.8\nspec:\n  root: spec/contracts\n'
  );
  await fs.writeFile(
    path.join(specRoot, 'philosophy', 'foundation.yaml'),
    'category: Philosophy\nphilosophies:\n  - id: PHIL-CUSTOM-001\n    title: Custom root\n'
  );
  await fs.writeFile(
    path.join(specRoot, 'policies', 'policies.yaml'),
    'category: Policies\npolicies:\n  - id: POL-CUSTOM-001\n    title: Custom policy\n    linked_philosophies:\n      - PHIL-CUSTOM-001\n'
  );
  await fs.writeFile(
    path.join(specRoot, 'requirements', 'core.yaml'),
    'category: Requirements\nrequirements:\n  - id: REQ-CUSTOM-001\n    title: Custom requirement\n    linked_policies:\n      - POL-CUSTOM-001\n'
  );
  await fs.writeFile(path.join(specRoot, 'features', 'features.yaml'), 'version: "1"\nfiles: []\n');
  await fs.writeFile(
    path.join(specRoot, 'features', 'core.yaml'),
    'category: Features\nfeatures:\n  - id: FEAT-CUSTOM-001\n    title: Custom feature\n    linked_requirements:\n      - REQ-CUSTOM-001\n'
  );

  return { workspaceRoot, specRoot };
}

test('loadSpecModel indexes spec documents without syu yaml', async () => {
  const model = await loadSpecModel(fixtureRoot('passing'));

  assert.equal(model.byKind.get('philosophy').length, 1);
  assert.equal(model.byKind.get('policy').length, 2);
  assert.equal(model.byKind.get('requirement').length, 5);
  assert.equal(model.byKind.get('feature').length, 5);
  assert.equal(
    model.byId.get('REQ-TRACE-001').documentPath,
    'docs/syu/requirements/traceability/core.yaml'
  );
});

test('lookupTrace links source files back to requirements features and policies', async () => {
  const model = await loadSpecModel(fixtureRoot('passing'));
  const trace = lookupTrace(model, path.join(fixtureRoot('passing'), 'src/rust_feature.rs'));

  assert.equal(trace.status, 'owned');
  assert.deepEqual(trace.matchedOwners.map((item) => item.id), ['FEAT-TRACE-001']);
  assert.deepEqual(trace.requirements.map((item) => item.id), ['REQ-TRACE-001']);
  assert.deepEqual(trace.features.map((item) => item.id), ['FEAT-TRACE-001']);
  assert.deepEqual(trace.policies.map((item) => item.id), ['POL-TRACE-001', 'POL-TRACE-002']);
  assert.deepEqual(trace.philosophies.map((item) => item.id), ['PHIL-TRACE-001']);
});

test('lookupTrace reports partial symbol ownership when only the file is traced', async () => {
  const model = await loadSpecModel(fixtureRoot('passing'));
  const trace = lookupTrace(
    model,
    path.join(fixtureRoot('passing'), 'src/rust_feature.rs'),
    'missingSymbol'
  );

  assert.equal(trace.status, 'partial');
  assert.equal(trace.matchedOwners.length, 0);
  assert.deepEqual(trace.fileOnlyOwners.map((item) => item.id), ['FEAT-TRACE-001']);
});

test('openTargetsForSpecId returns the YAML document and traced files', async () => {
  const model = await loadSpecModel(fixtureRoot('passing'));
  const targets = openTargetsForSpecId(model, 'FEAT-TRACE-001');

  assert.equal(targets[0].kind, 'document');
  assert.ok(targets.some((target) => target.path.endsWith(path.join('src', 'rust_feature.rs'))));
});

test('resolveIssueTarget maps definition issues back to YAML files', async () => {
  const model = await loadSpecModel(fixtureRoot('passing'));
  const target = await resolveIssueTarget(
    {
      subject: 'requirement REQ-TRACE-001',
      location: 'status',
      message: 'status is broken'
    },
    model,
    fixtureRoot('passing')
  );

  assert.ok(
    target.path.endsWith(path.join('docs', 'syu', 'requirements', 'traceability', 'core.yaml'))
  );
  assert.equal(target.range.line, 8);
  assert.equal(target.range.startCharacter, 4);
});

test('resolveIssueTarget preserves absolute issue locations', async () => {
  const model = await loadSpecModel(fixtureRoot('passing'));
  const absoluteTarget = path.join(fixtureRoot('passing'), 'src', 'rust_feature.rs');
  const target = await resolveIssueTarget(
    {
      subject: 'feature FEAT-TRACE-001',
      location: absoluteTarget,
      message: 'trace is broken'
    },
    model,
    fixtureRoot('passing')
  );

  assert.equal(target.path, absoluteTarget);
});

test('normalizeRelativePath keeps repository relative paths portable', () => {
  assert.equal(normalizeRelativePath('.\\src\\feature.js'), 'src/feature.js');
});

test('resolveWorkspaceContext honors configured workspace roots', async () => {
  const workspace = await createCustomSpecRootWorkspace();
  const context = await resolveWorkspaceContext(workspace.workspaceRoot);

  assert.equal(context.workspaceRoot, workspace.workspaceRoot);
  assert.equal(context.specRoot, workspace.specRoot);
});

test('resolveWorkspaceContext resolves an opened spec root back to the repository root', async () => {
  const workspace = await createCustomSpecRootWorkspace();
  const context = await resolveWorkspaceContext(workspace.specRoot);

  assert.equal(context.workspaceRoot, workspace.workspaceRoot);
  assert.equal(context.specRoot, workspace.specRoot);
});
