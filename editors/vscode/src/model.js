// FEAT-VSCODE-001

const fs = require('node:fs/promises');
const path = require('node:path');
const { execFile } = require('node:child_process');

const YAML = require('yaml');

const MAX_BUFFER_BYTES = 10 * 1024 * 1024
const SOURCE_LOCATION_LANGUAGES = new Set([
  'rust',
  'python',
  'typescript',
  'javascript',
  'shell',
  'yaml',
  'json',
  'markdown',
  'gitignore'
])
const SPEC_KINDS = ['philosophy', 'policy', 'requirement', 'feature']
const DOC_KIND_TO_ITEM_KIND = {
  philosophies: 'philosophy',
  policies: 'policy',
  requirements: 'requirement',
  features: 'feature'
}

function toPosixPath(value) {
  return String(value).replace(/\\/g, '/')
}

function toSystemPath(value) {
  return String(value).split('/').join(path.sep)
}

function normalizeRelativePath(value) {
  if (!value) {
    return ''
  }

  const normalized = path.posix.normalize(toPosixPath(value))
  if (normalized === '.') {
    return ''
  }

  return normalized.replace(/^\.\//, '')
}

async function pathExists(filePath) {
  try {
    await fs.access(filePath)
    return true
  } catch {
    return false
  }
}

async function looksLikeSpecRoot(root) {
  if (!(await pathExists(root))) {
    return false
  }

  const yamlFiles = await walkYamlFiles(root)
  return yamlFiles.length > 0
}

async function readWorkspaceConfig(workspaceRoot) {
  const configPath = path.join(workspaceRoot, 'syu.yaml')
  if (!(await pathExists(configPath))) {
    return null
  }

  try {
    const parsed = YAML.parse(await fs.readFile(configPath, 'utf8')) || {}
    const v1Roots = Array.isArray(parsed?.workspace?.spec_roots)
      ? parsed.workspace.spec_roots.filter((value) => typeof value === 'string' && value.trim())
      : []
    if (!v1Roots[0]) {
      return null
    }
    return { specRoot: v1Roots[0] }
  } catch {
    return null
  }
}

async function resolveWorkspaceContext(startPath) {
  const resolved = path.resolve(startPath)
  const searchRoot = resolved
  let current = searchRoot

  while (true) {
    const configPath = path.join(current, 'syu.yaml')
    if (await pathExists(configPath)) {
      const workspaceConfig = await readWorkspaceConfig(current)
      if (!workspaceConfig) {
        const parent = path.dirname(current)
        if (parent === current) {
          break
        }
        current = parent
        continue
      }
      const { specRoot } = workspaceConfig
      const absoluteSpecRoot = path.resolve(current, specRoot)
      if (
        (searchRoot === current || isWithinPath(searchRoot, absoluteSpecRoot)) &&
        (await looksLikeSpecRoot(absoluteSpecRoot))
      ) {
        return { workspaceRoot: current, specRoot: absoluteSpecRoot }
      }
    }

    const parent = path.dirname(current)
    if (parent === current) {
      break
    }
    current = parent
  }

  if (await looksLikeSpecRoot(searchRoot)) {
    return { workspaceRoot: searchRoot, specRoot: searchRoot }
  }

  return null
}

function isWithinPath(candidate, parent) {
  const relative = path.relative(parent, candidate)
  return relative === '' || (!relative.startsWith('..') && !path.isAbsolute(relative))
}

async function walkYamlFiles(root) {
  if (!(await pathExists(root))) {
    return []
  }

  const entries = await fs.readdir(root, { withFileTypes: true })
  const files = []

  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    const fullPath = path.join(root, entry.name)
    if (entry.isDirectory()) {
      files.push(...(await walkYamlFiles(fullPath)))
      continue
    }

    if (entry.isFile() && /\.ya?ml$/i.test(entry.name)) {
      files.push(fullPath)
    }
  }

  return files
}

function anchorItemId(value) {
  if (typeof value !== 'string') {
    return null
  }

  const [itemId] = value.split('#')
  return /\b[A-Z][A-Z0-9-]*\b/u.test(itemId || '') ? itemId : null
}

function selectorSymbols(selector) {
  if (!selector || typeof selector !== 'object' || Array.isArray(selector)) {
    return []
  }

  if (selector.kind === 'symbol' && Array.isArray(selector.names)) {
    return selector.names.filter((value) => typeof value === 'string' && value.trim())
  }

  return []
}

function targetTraceReference(target) {
  if (!target || typeof target !== 'object' || Array.isArray(target)) {
    return null
  }

  if (typeof target.path !== 'string' || !target.path.trim()) {
    return null
  }

  return {
    file: normalizeRelativePath(target.path),
    symbols: selectorSymbols(target.selector),
    docContains: []
  }
}

function appendGroupedReference(groups, language, reference) {
  if (!reference) {
    return
  }

  const key = typeof language === 'string' && language.trim() ? language.trim() : 'unknown'
  if (!groups[key]) {
    groups[key] = []
  }
  groups[key].push(reference)
}

function createV1IndexEntry(kind, item, documentPath) {
  return {
    kind,
    id: item.id,
    title: item.title,
    documentPath,
    linkedPhilosophies: [],
    linkedPolicies: [],
    linkedRequirements: [],
    linkedFeatures: [],
    verificationTargets: {},
    implementationTargets: {},
    _policyIds: new Set(),
    _requirementIds: new Set()
  }
}

function collectV1DocumentEntries(kind, items, documentPath, byId, byKind) {
  if (!Array.isArray(items)) {
    return
  }

  for (const item of items) {
    if (!item || typeof item.id !== 'string' || typeof item.title !== 'string') {
      continue
    }

    const entry = createV1IndexEntry(kind, item, documentPath)

    if (kind === 'policy') {
      for (const rule of Array.isArray(item.rules) ? item.rules : []) {
        for (const reference of Array.isArray(rule?.governed_by) ? rule.governed_by : []) {
          const philosophyId = anchorItemId(reference)
          if (philosophyId) {
            entry.linkedPhilosophies.push(philosophyId)
          }
        }
      }
    }

    if (kind === 'requirement') {
      for (const criterion of Array.isArray(item.criteria) ? item.criteria : []) {
        for (const reference of Array.isArray(criterion?.governed_by) ? criterion.governed_by : []) {
          const policyId = anchorItemId(reference)
          if (policyId) {
            entry._policyIds.add(policyId)
          }
        }
      }

      for (const binding of Array.isArray(item.bindings) ? item.bindings : []) {
        if (binding?.role !== 'verification') {
          continue
        }

        for (const target of Array.isArray(binding.targets) ? binding.targets : []) {
          appendGroupedReference(
            entry.verificationTargets,
            target?.adapter,
            targetTraceReference(target)
          )
        }
      }
    }

    if (kind === 'feature') {
      for (const binding of Array.isArray(item.bindings) ? item.bindings : []) {
        if (binding?.role === 'implementation') {
          for (const reference of Array.isArray(binding?.satisfies) ? binding.satisfies : []) {
            const requirementId = anchorItemId(reference)
            if (requirementId) {
              entry._requirementIds.add(requirementId)
            }
          }

          for (const target of Array.isArray(binding.targets) ? binding.targets : []) {
            appendGroupedReference(
              entry.implementationTargets,
              target?.adapter,
              targetTraceReference(target)
            )
          }
        }
      }
    }

    entry.linkedPhilosophies = [...new Set(entry.linkedPhilosophies)].sort()
    byId.set(entry.id, entry)
    byKind.get(kind).push(entry)
  }
}

function finalizeV1Relationships(byKind) {
  const requirementsById = new Map(byKind.get('requirement').map((entry) => [entry.id, entry]))

  for (const feature of byKind.get('feature')) {
    feature.linkedRequirements = [...feature._requirementIds].sort()
    for (const requirementId of feature.linkedRequirements) {
      const requirement = requirementsById.get(requirementId)
      if (requirement) {
        requirement.linkedFeatures.push(feature.id)
      }
    }
  }

  for (const requirement of byKind.get('requirement')) {
    requirement.linkedPolicies = [...requirement._policyIds].sort()
    requirement.linkedFeatures = [...new Set(requirement.linkedFeatures)].sort()
  }
}

async function loadSpecModel(workspaceRoot) {
  const context = await resolveWorkspaceContext(workspaceRoot)
  const resolvedWorkspaceRoot = context?.workspaceRoot || workspaceRoot
  if (!context) {
    throw new Error('The selected folder is not inside a syu v1 workspace.')
  }
  const absoluteSpecRoot = context.specRoot
  const yamlFiles = await walkYamlFiles(absoluteSpecRoot)
  const byId = new Map()
  const byKind = new Map(SPEC_KINDS.map((kind) => [kind, []]))

  for (const filePath of yamlFiles) {
    let parsed

    try {
      parsed = YAML.parse(await fs.readFile(filePath, 'utf8'))
    } catch {
      continue
    }

    const documentPath = normalizeRelativePath(path.relative(resolvedWorkspaceRoot, filePath))
    if (!DOC_KIND_TO_ITEM_KIND[parsed?.kind]) {
      continue
    }

    collectV1DocumentEntries(
      DOC_KIND_TO_ITEM_KIND[parsed.kind],
      parsed?.[parsed.kind],
      documentPath,
      byId,
      byKind
    )
  }

  finalizeV1Relationships(byKind)

  return {
    workspaceRoot: resolvedWorkspaceRoot,
    specRoot: absoluteSpecRoot,
    byId,
    byKind
  }
}

function summarizeEntry(entry) {
  return {
    id: entry.id,
    kind: entry.kind,
    title: entry.title,
    documentPath: entry.documentPath
  }
}

function sortedMapValues(map) {
  return [...map.values()].sort((left, right) => left.id.localeCompare(right.id))
}

function createOwnerMatch(owner, traceRole, language, reference, matchMode, symbol) {
  return {
    kind: owner.kind,
    id: owner.id,
    title: owner.title,
    documentPath: owner.documentPath,
    traceRole,
    language,
    file: reference.file,
    declaredSymbols: [...reference.symbols],
    matchedSymbol:
      matchMode === 'symbol' ? symbol : matchMode === 'wildcard' ? '*' : null,
    matchMode
  }
}

function matchTraceReference(reference, symbol) {
  if (!symbol) {
    return 'file'
  }

  if (reference.symbols.includes('*')) {
    return 'wildcard'
  }

  if (reference.symbols.includes(symbol)) {
    return 'symbol'
  }

  return null
}

function dedupeOwnerMatches(matches) {
  const seen = new Set()

  return matches.filter((match) => {
    const key = JSON.stringify([
      match.kind,
      match.id,
      match.traceRole,
      match.language,
      match.file,
      match.matchMode,
      match.matchedSymbol
    ])
    if (seen.has(key)) {
      return false
    }
    seen.add(key)
    return true
  })
}

function collectRelatedItems(model, owners) {
  const requirements = new Map()
  const features = new Map()
  const policies = new Map()
  const philosophies = new Map()

  for (const owner of owners) {
    if (owner.kind === 'requirement') {
      const requirement = model.byId.get(owner.id)
      if (!requirement) {
        continue
      }

      requirements.set(requirement.id, summarizeEntry(requirement))
      for (const featureId of requirement.linkedFeatures) {
        const feature = model.byId.get(featureId)
        if (feature) {
          features.set(feature.id, summarizeEntry(feature))
        }
      }
      collectRequirementContext(model, requirement, policies, philosophies)
      continue
    }

    if (owner.kind === 'feature') {
      const feature = model.byId.get(owner.id)
      if (!feature) {
        continue
      }

      features.set(feature.id, summarizeEntry(feature))
      for (const requirementId of feature.linkedRequirements) {
        const requirement = model.byId.get(requirementId)
        if (!requirement) {
          continue
        }

        requirements.set(requirement.id, summarizeEntry(requirement))
        collectRequirementContext(model, requirement, policies, philosophies)
      }
    }
  }

  return {
    requirements: sortedMapValues(requirements),
    features: sortedMapValues(features),
    policies: sortedMapValues(policies),
    philosophies: sortedMapValues(philosophies)
  }
}

function collectRequirementContext(model, requirement, policies, philosophies) {
  for (const policyId of requirement.linkedPolicies) {
    const policy = model.byId.get(policyId)
    if (!policy) {
      continue
    }

    policies.set(policy.id, summarizeEntry(policy))
    for (const philosophyId of policy.linkedPhilosophies) {
      const philosophy = model.byId.get(philosophyId)
      if (philosophy) {
        philosophies.set(philosophy.id, summarizeEntry(philosophy))
      }
    }
  }
}

function relativePathFromWorkspace(workspaceRoot, filePath) {
  const absolutePath = path.isAbsolute(filePath)
    ? filePath
    : path.join(workspaceRoot, filePath)
  return normalizeRelativePath(path.relative(workspaceRoot, absolutePath))
}

function lookupTrace(model, filePath, symbol) {
  const relativeFile = relativePathFromWorkspace(model.workspaceRoot, filePath)
  const matchedOwners = []
  const fileOnlyOwners = []

  for (const requirement of model.byKind.get('requirement')) {
    collectTraceMatches(requirement, 'test', requirement.verificationTargets)
  }

  for (const feature of model.byKind.get('feature')) {
    collectTraceMatches(feature, 'implementation', feature.implementationTargets)
  }

  function collectTraceMatches(owner, traceRole, groups) {
    for (const [language, references] of Object.entries(groups)) {
      for (const reference of references) {
        if (reference.file !== relativeFile) {
          continue
        }

        const matchMode = matchTraceReference(reference, symbol)
        if (matchMode) {
          matchedOwners.push(
            createOwnerMatch(owner, traceRole, language, reference, matchMode, symbol || null)
          )
        } else if (symbol) {
          fileOnlyOwners.push(
            createOwnerMatch(owner, traceRole, language, reference, 'file', null)
          )
        }
      }
    }
  }

  const dedupedMatches = dedupeOwnerMatches(matchedOwners)
  const dedupedFileOnly = dedupeOwnerMatches(fileOnlyOwners)
  const contextOwners = dedupedMatches.length > 0 ? dedupedMatches : dedupedFileOnly
  const related = collectRelatedItems(model, contextOwners)

  return {
    file: relativeFile,
    symbol: symbol || null,
    status:
      dedupedMatches.length > 0
        ? 'owned'
        : dedupedFileOnly.length > 0
          ? 'partial'
          : 'unowned',
    matchedOwners: dedupedMatches,
    fileOnlyOwners: dedupedFileOnly,
    ...related
  }
}

function specIdFromText(value) {
  const match = /\b(?:PHIL|POL|REQ|FEAT)-[A-Z0-9-]+\b/.exec(String(value || ''))
  return match ? match[0] : null
}

function collectInlineNavigationTargets(documentText) {
  const lines = String(documentText || '').split(/\r?\n/u)
  const targets = []
  let activeTraceFile = null
  let traceFileIndent = -1
  let inSymbols = false
  let symbolsIndent = -1

  for (let line = 0; line < lines.length; line += 1) {
    const text = lines[line]
    const trimmed = text.trim()
    const indent = text.match(/^(\s*)/u)?.[1].length ?? 0

    for (const match of text.matchAll(/\b(?:PHIL|POL|REQ|FEAT)-[A-Z0-9-]+\b/gu)) {
      targets.push({
        kind: 'specId',
        id: match[0],
        line,
        startCharacter: match.index,
        endCharacter: match.index + match[0].length
      })
    }

    const fileMatch = /^(\s*)(?:-\s+)?(?:file|path):\s*["']?([^"'#]+?)["']?\s*$/u.exec(text)
    if (fileMatch) {
      const file = normalizeRelativePath(fileMatch[2])
      activeTraceFile = file || null
      traceFileIndent = fileMatch[1].length
      inSymbols = false
      symbolsIndent = -1

      if (activeTraceFile) {
        const startCharacter = text.indexOf(fileMatch[2])
        targets.push({
          kind: 'traceFile',
          file: activeTraceFile,
          line,
          startCharacter,
          endCharacter: startCharacter + fileMatch[2].length
        })
      }
      continue
    }

    if (!trimmed) {
      continue
    }

    if (activeTraceFile && indent <= traceFileIndent) {
      activeTraceFile = null
      traceFileIndent = -1
      inSymbols = false
      symbolsIndent = -1
    }

    if (!activeTraceFile) {
      continue
    }

    if (/^\s*(?:symbols|names):\s*$/u.test(text) && indent > traceFileIndent) {
      inSymbols = true
      symbolsIndent = indent
      continue
    }

    if (!inSymbols) {
      continue
    }

    if (indent <= symbolsIndent) {
      inSymbols = false
      symbolsIndent = -1
      continue
    }

    const symbolValue = parseTraceSymbolEntry(text)
    if (!symbolValue) {
      continue
    }

    const startCharacter = text.indexOf(symbolValue)
    targets.push({
      kind: 'traceSymbol',
      file: activeTraceFile,
      symbol: symbolValue,
      line,
      startCharacter,
      endCharacter: startCharacter + symbolValue.length
    })
  }

  return targets
}

function parseTraceSymbolEntry(text) {
  const match = /^\s*-\s*(.+?)\s*$/u.exec(text)
  if (!match) {
    return null
  }

  const rawValue = match[1].replace(/\s+#.*$/u, '').trim()
  if (!rawValue) {
    return null
  }

  if (
    (rawValue.startsWith('"') && rawValue.endsWith('"')) ||
    (rawValue.startsWith("'") && rawValue.endsWith("'"))
  ) {
    return rawValue.slice(1, -1).trim() || null
  }

  return rawValue
}

function itemFromIssueSubject(issue, model) {
  const id = specIdFromText(issue.subject)
  return id ? model?.byId.get(id) || null : null
}

function parseTraceLocation(location) {
  if (typeof location !== 'string') {
    return null
  }

  const separator = location.indexOf(':')
  if (separator <= 0) {
    return null
  }

  const prefix = location.slice(0, separator)
  if (!SOURCE_LOCATION_LANGUAGES.has(prefix)) {
    return null
  }

  const rawPath = location.slice(separator + 1).trim()
  if (!rawPath) {
    return null
  }

  const systemPath = toSystemPath(rawPath)
  if (path.isAbsolute(systemPath)) {
    return path.normalize(systemPath)
  }

  const relativePath = normalizeRelativePath(rawPath)
  return relativePath || null
}

function looksLikeWorkspaceRelativeFile(value) {
  if (typeof value !== 'string' || !value.trim()) {
    return false
  }

  return (
    value === 'syu.yaml' ||
    value.includes('/') ||
    value.includes('\\') ||
    /\.(?:ya?ml|rs|py|tsx?|jsx?|sh|bash|zsh|json|md)$/i.test(value)
  )
}

function isFieldName(value) {
  return typeof value === 'string' && /^[a-z_][a-z0-9_]*$/i.test(value)
}

function formatDiagnosticMessage(issue) {
  const message = issue?.message || 'syu reported an issue'
  return issue?.suggestion ? `${message}\nSuggestion: ${issue.suggestion}` : message
}

async function resolveIssueTarget(issue, model, workspaceRoot) {
  const traceLocation = parseTraceLocation(issue.location)
  const subjectItem = itemFromIssueSubject(issue, model)
  const locationPath =
    traceLocation ||
    (looksLikeWorkspaceRelativeFile(issue.location)
      ? issue.location
      : null) ||
    subjectItem?.documentPath ||
    'syu.yaml'
  const normalizedLocation = toSystemPath(locationPath)
  const targetPath = path.isAbsolute(normalizedLocation)
    ? path.normalize(normalizedLocation)
    : path.join(workspaceRoot, normalizeRelativePath(normalizedLocation))
  const range = await resolveIssueRange(targetPath, issue, subjectItem)

  return { path: targetPath, range }
}

async function resolveIssueRange(targetPath, issue, subjectItem) {
  try {
    const contents = await fs.readFile(targetPath, 'utf8')
    return findIssueRange(contents, issue, subjectItem)
  } catch {
    return { line: 0, startCharacter: 0, endCharacter: 0 }
  }
}

function findIssueRange(contents, issue, subjectItem) {
  const lines = contents.split(/\r?\n/u)

  if (subjectItem) {
    const subjectRange = findTermRange(lines, `id: ${subjectItem.id}`)
    if (subjectRange) {
      if (isFieldName(issue.location)) {
        const fieldRange = findFieldRangeInItemBlock(lines, issue.location, subjectRange.line)
        if (fieldRange) {
          return fieldRange
        }
      }
      return subjectRange
    }
  }

  const searchTerms = []
  if (
    typeof issue.location === 'string' &&
    issue.location &&
    !parseTraceLocation(issue.location) &&
    !looksLikeWorkspaceRelativeFile(issue.location)
  ) {
    searchTerms.push(issue.location)
  }

  for (const term of searchTerms) {
    const range = findTermRange(lines, term)
    if (range) {
      return range
    }
  }

  return { line: 0, startCharacter: 0, endCharacter: 0 }
}

function findTermRange(lines, term, startLine = 0, endLine = lines.length) {
  for (let line = startLine; line < endLine; line += 1) {
    const startCharacter = lines[line].indexOf(term)
    if (startCharacter === -1) {
      continue
    }

    return {
      line,
      startCharacter,
      endCharacter: startCharacter + term.length
    }
  }

  return null
}

function findFieldRangeInItemBlock(lines, fieldName, itemStartLine) {
  const blockEnd = findItemBlockEnd(lines, itemStartLine)
  const fieldPrefix = `${fieldName}:`

  for (let line = itemStartLine + 1; line < blockEnd; line += 1) {
    const trimmed = lines[line].trimStart()
    if (!trimmed.startsWith(fieldPrefix)) {
      continue
    }

    const startCharacter = lines[line].indexOf(fieldPrefix)
    return {
      line,
      startCharacter,
      endCharacter: startCharacter + fieldPrefix.length
    }
  }

  return null
}

function findItemBlockEnd(lines, itemStartLine) {
  const itemIndent = lines[itemStartLine].match(/^(\s*)-\s+id:\s+/u)?.[1].length ?? 0

  for (let line = itemStartLine + 1; line < lines.length; line += 1) {
    const current = lines[line]
    if (!current.trim()) {
      continue
    }

    const nextItem = current.match(/^(\s*)-\s+id:\s+/u)
    if (nextItem && nextItem[1].length === itemIndent) {
      return line
    }

    const currentIndent = current.match(/^(\s*)/u)?.[1].length ?? 0
    if (currentIndent <= itemIndent) {
      return line
    }
  }

  return lines.length
}

function runSyuJson({ workspaceRoot, binaryPath, args }) {
  return new Promise((resolve, reject) => {
    execFile(
      binaryPath,
      args,
      { cwd: workspaceRoot, maxBuffer: MAX_BUFFER_BYTES },
      (error, stdout, stderr) => {
        const trimmedStdout = stdout.trim()

        if (trimmedStdout) {
          try {
            resolve(JSON.parse(trimmedStdout))
            return
          } catch (parseError) {
            reject(
              new Error(
                [
                  `Failed to parse JSON from \`${binaryPath} ${args.join(' ')}\`.`,
                  parseError.message,
                  stderr.trim() || trimmedStdout
                ]
                  .filter(Boolean)
                  .join('\n')
              )
            )
            return
          }
        }

        if (error?.code === 'ENOENT') {
          reject(
            new Error(
              `Could not execute \`${binaryPath}\`. Set \`syu.binaryPath\` to the installed syu CLI.`
            )
          )
          return
        }

        reject(new Error(stderr.trim() || error?.message || 'syu command failed'))
      }
    )
  })
}

async function loadDiagnostics({ workspaceRoot, binaryPath, model }) {
  const result = await runSyuJson({
    workspaceRoot,
    binaryPath,
    args: ['validate', '.', '--format', 'json']
  })

  const diagnostics = []
  for (const issue of result.issues || []) {
    const target = await resolveIssueTarget(issue, model, workspaceRoot)
    diagnostics.push({
      path: target.path,
      range: target.range,
      severity: issue.severity,
      code: issue.code,
      message: formatDiagnosticMessage(issue)
    })
  }

  return diagnostics
}

function openTargetsForSpecId(model, id) {
  const item = model.byId.get(id)
  if (!item) {
    return []
  }

  const targets = [
    {
      kind: 'document',
      label: `${id} definition`,
      description: item.documentPath,
      path: path.join(model.workspaceRoot, toSystemPath(item.documentPath)),
      searchText: `id: ${id}`
    }
  ]

  const referenceGroups =
    item.kind === 'requirement'
      ? item.verificationTargets
      : item.kind === 'feature'
        ? item.implementationTargets
        : {}

  for (const [language, references] of Object.entries(referenceGroups)) {
    for (const reference of references) {
      targets.push({
        kind: 'trace',
        label: `${language}: ${reference.file}`,
        description: `${item.kind} ${id}`,
        path: path.join(model.workspaceRoot, toSystemPath(reference.file)),
        searchText:
          reference.symbols.find((symbol) => symbol && symbol !== '*') || null
      })
    }
  }

  const seen = new Set()
  return targets.filter((target) => {
    const key = `${target.kind}:${target.path}:${target.searchText || ''}`
    if (seen.has(key)) {
      return false
    }
    seen.add(key)
    return true
  })
}

module.exports = {
  collectInlineNavigationTargets,
  formatDiagnosticMessage,
  loadDiagnostics,
  loadSpecModel,
  lookupTrace,
  normalizeRelativePath,
  openTargetsForSpecId,
  readWorkspaceConfig,
  resolveWorkspaceContext,
  resolveIssueTarget,
  runSyuJson,
  specIdFromText
}
