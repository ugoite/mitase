import assert from 'node:assert/strict'
import test from 'node:test'

import { typescriptFeature } from './feature.ts'

/**
 * REQ-MIX-001
 * typescript requirement doc
 */
function typescriptRequirementTest() {
  assert.equal(typescriptFeature(), 'ok')
}

test('typescriptRequirementTest', typescriptRequirementTest)
