// FEAT-VSCODE-001
// REQ-CORE-022

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');

const {
  collectInlineNavigationTargets,
  loadSpecModel,
  lookupTrace,
  normalizeRelativePath,
  openTargetsForSpecId,
  resolveWorkspaceContext,
  resolveIssueTarget
} = require('../src/model');

function v1FixtureRoot(name) {
  return path.resolve(__dirname, '../../../fixtures/v1', name);
}

async function createCustomSpecRootWorkspace() {
  const workspaceRoot = await fs.mkdtemp(path.join(os.tmpdir(), 'syu-vscode-spec-root-'));
  const specRoot = path.join(workspaceRoot, 'spec', 'contracts');

  await fs.mkdir(specRoot, { recursive: true });
  await fs.writeFile(
    path.join(workspaceRoot, 'syu.yaml'),
    [
      'schema: syu/config/v1',
      'workspace:',
      '  spec_roots: [spec/contracts]',
      '  artifact_roots: [src, tests]',
      '  excludes: []',
      'profiles: { active: [], custom: {} }',
      'validation:',
      '  preset: standard',
      '  deny_warnings: false',
      '  rules: {}',
      '  changed: { require_owned_changes: false }',
      'work:',
      '  slicing:',
      '    max_editable_files: 2',
      '    max_editable_symbols: 2',
      '    max_verification_targets: 2',
      '    max_readonly_targets: 2',
      '    max_total_bytes: 4096',
      '  context:',
      '    include_parent_principles: false',
      '    include_parent_rules: false',
      'adapters: { enabled: [rust] }',
      ''
    ].join('\n')
  );
  await fs.writeFile(
    path.join(specRoot, 'foundation.yaml'),
    [
      'schema: syu/spec/v1',
      'kind: philosophies',
      'namespace: custom',
      'category: Custom',
      'philosophies:',
      '  - id: PHIL-CUSTOM-001',
      '    title: Custom root',
      '    summary: Custom summary.',
      '    principles:',
      '      - { id: governed, statement: Keep it governed., applies_to: [product] }',
      '    bindings: []',
      ''
    ].join('\n')
  );
  await fs.writeFile(
    path.join(specRoot, 'policy.yaml'),
    [
      'schema: syu/spec/v1',
      'kind: policies',
      'namespace: custom',
      'category: Custom',
      'policies:',
      '  - id: POL-CUSTOM-001',
      '    title: Custom policy',
      '    summary: Custom policy summary.',
      '    description: Custom policy description.',
      '    rules:',
      '      - id: governed',
      '        level: must',
      '        statement: Keep it governed.',
      '        governed_by: [PHIL-CUSTOM-001#principle.governed]',
      '    bindings: []',
      ''
    ].join('\n')
  );
  await fs.writeFile(
    path.join(specRoot, 'requirement.yaml'),
    [
      'schema: syu/spec/v1',
      'kind: requirements',
      'namespace: custom',
      'category: Custom',
      'requirements:',
      '  - id: REQ-CUSTOM-001',
      '    title: Custom requirement',
      '    description: Custom requirement.',
      '    priority: high',
      '    status: implemented',
      '    criteria:',
      '      - id: check',
      '        kind: behavior',
      '        statement: Verify the custom behavior.',
      '        governed_by: [POL-CUSTOM-001#rule.governed]',
      '    bindings:',
      '      - id: verify',
      '        role: verification',
      '        facet: verification',
      '        responsibility: Verify the custom behavior.',
      '        targets:',
      '          - { id: case, adapter: rust, path: tests/check.rs, selector: { kind: symbol, names: [check_behavior] } }',
      '        verifies: [REQ-CUSTOM-001#criterion.check]',
      ''
    ].join('\n')
  );
  await fs.writeFile(
    path.join(specRoot, 'feature.yaml'),
    [
      'schema: syu/spec/v1',
      'kind: features',
      'namespace: custom',
      'category: Custom',
      'features:',
      '  - id: FEAT-CUSTOM-001',
      '    title: Custom feature',
      '    summary: Custom feature summary.',
      '    status: implemented',
      '    bindings:',
      '      - id: app',
      '        role: implementation',
      '        facet: backend',
      '        responsibility: Implement the custom behavior.',
      '        targets:',
      '          - { id: code, adapter: rust, path: src/lib.rs, selector: { kind: symbol, names: [run] } }',
      '        satisfies: [REQ-CUSTOM-001#criterion.check]',
      ''
    ].join('\n')
  );

  return { workspaceRoot, specRoot };
}

test('loadSpecModel indexes v1 spec documents and derives relationships', async () => {
  const model = await loadSpecModel(v1FixtureRoot('valid-web-app'));

  assert.equal(model.byKind.get('philosophy').length, 1);
  assert.equal(model.byKind.get('policy').length, 1);
  assert.equal(model.byKind.get('requirement').length, 1);
  assert.equal(model.byKind.get('feature').length, 1);
  assert.equal(model.byId.get('REQ-AUTH-001').documentPath, 'spec/requirement.yaml');
  assert.deepEqual(model.byId.get('REQ-AUTH-001').linkedPolicies, ['POL-AUTH-001']);
  assert.deepEqual(model.byId.get('REQ-AUTH-001').linkedFeatures, ['FEAT-AUTH-001']);
  assert.deepEqual(model.byId.get('FEAT-AUTH-001').linkedRequirements, ['REQ-AUTH-001']);
});

test('lookupTrace links implementation files back to requirements and governance', async () => {
  const model = await loadSpecModel(v1FixtureRoot('valid-web-app'));
  const trace = lookupTrace(model, path.join(v1FixtureRoot('valid-web-app'), 'api/login.rs'));

  assert.equal(trace.status, 'owned');
  assert.deepEqual(trace.matchedOwners.map((item) => item.id), ['FEAT-AUTH-001']);
  assert.deepEqual(trace.requirements.map((item) => item.id), ['REQ-AUTH-001']);
  assert.deepEqual(trace.features.map((item) => item.id), ['FEAT-AUTH-001']);
  assert.deepEqual(trace.policies.map((item) => item.id), ['POL-AUTH-001']);
  assert.deepEqual(trace.philosophies.map((item) => item.id), ['PHIL-AUTH-001']);
});

test('lookupTrace links verification files back to their owning requirement', async () => {
  const model = await loadSpecModel(v1FixtureRoot('valid-web-app'));
  const trace = lookupTrace(model, path.join(v1FixtureRoot('valid-web-app'), 'tests/login.rs'));

  assert.equal(trace.status, 'owned');
  assert.deepEqual(trace.matchedOwners.map((item) => item.id), ['REQ-AUTH-001']);
  assert.deepEqual(trace.requirements.map((item) => item.id), ['REQ-AUTH-001']);
  assert.deepEqual(trace.features.map((item) => item.id), ['FEAT-AUTH-001']);
});

test('openTargetsForSpecId returns the YAML document and v1 binding targets', async () => {
  const model = await loadSpecModel(v1FixtureRoot('valid-web-app'));
  const targets = openTargetsForSpecId(model, 'FEAT-AUTH-001');

  assert.equal(targets[0].kind, 'document');
  assert.ok(targets.some((target) => target.path.endsWith(path.join('api', 'login.rs'))));
  assert.ok(targets.some((target) => target.path.endsWith(path.join('web', 'login.ts'))));
});

test('resolveIssueTarget maps definition issues back to v1 YAML files', async () => {
  const workspaceRoot = v1FixtureRoot('valid-web-app');
  const model = await loadSpecModel(workspaceRoot);
  const target = await resolveIssueTarget(
    {
      subject: 'requirement REQ-AUTH-001',
      location: 'status',
      message: 'status is broken'
    },
    model,
    workspaceRoot
  );

  assert.ok(target.path.endsWith(path.join('spec', 'requirement.yaml')));
  assert.ok(target.range.line >= 0);
});

test('normalizeRelativePath keeps repository relative paths portable', () => {
  assert.equal(normalizeRelativePath('.\\src\\feature.js'), 'src/feature.js');
});

test('resolveWorkspaceContext honors configured v1 workspace roots', async () => {
  const workspace = await createCustomSpecRootWorkspace();
  const context = await resolveWorkspaceContext(workspace.workspaceRoot);

  assert.equal(context.workspaceRoot, workspace.workspaceRoot);
  assert.equal(context.specRoot, workspace.specRoot);
});

test('resolveWorkspaceContext resolves an opened v1 spec root back to the repository root', async () => {
  const workspace = await createCustomSpecRootWorkspace();
  const context = await resolveWorkspaceContext(workspace.specRoot);

  assert.equal(context.workspaceRoot, workspace.workspaceRoot);
  assert.equal(context.specRoot, workspace.specRoot);
});

test('collectInlineNavigationTargets finds v1 spec IDs target paths and selector names', () => {
  const targets = collectInlineNavigationTargets(`
requirements:
  - id: REQ-AUTH-001
    criteria:
      - id: invalid-credentials
        governed_by: [POL-AUTH-001#rule.generic-failure]
features:
  - id: FEAT-AUTH-001
    bindings:
      - id: ui
        targets:
          - path: web/login.ts
            selector:
              kind: symbol
              names:
                - submitLogin
                - helper_value
`);

  assert.deepEqual(
    targets
      .filter((target) => target.kind === 'specId')
      .map((target) => target.id),
    ['REQ-AUTH-001', 'POL-AUTH-001', 'FEAT-AUTH-001']
  );
  assert.deepEqual(
    targets.filter((target) => target.kind === 'traceFile').map((target) => target.file),
    ['web/login.ts']
  );
  assert.deepEqual(
    targets
      .filter((target) => target.kind === 'traceSymbol')
      .map((target) => [target.file, target.symbol]),
    [
      ['web/login.ts', 'submitLogin'],
      ['web/login.ts', 'helper_value']
    ]
  );
});
