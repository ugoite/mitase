// FEAT-VSCODE-001
// REQ-CORE-022

const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');

const {
  loadSpecModel,
  lookupTrace,
  normalizeRelativePath,
  openTargetsForSpecId,
  resolveIssueTarget
} = require('../src/model');

function fixtureRoot(name) {
  return path.resolve(__dirname, '../../../tests/fixtures/workspaces', name);
}

test('loadSpecModel indexes spec documents without syu yaml', async () => {
  const model = await loadSpecModel(fixtureRoot('passing'));

  assert.equal(model.byKind.get('philosophy').length, 1);
  assert.equal(model.byKind.get('policy').length, 2);
  assert.equal(model.byKind.get('requirement').length, 4);
  assert.equal(model.byKind.get('feature').length, 4);
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
