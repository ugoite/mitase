import assert from 'node:assert/strict'
import test from 'node:test'

import { typescriptFeature } from './app.ts'

/** REQ-TS-001 keeps the first TypeScript test trace explicit. */
test('typescriptRequirementTest', () => {
  assert.notEqual(typescriptFeature(), '')
})
