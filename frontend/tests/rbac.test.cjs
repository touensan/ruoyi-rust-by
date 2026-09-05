const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const ts = require('typescript')
let actor
function source(file, mocks = {}, globals = {}) {
  const code = ts.transpileModule(fs.readFileSync(path.join(__dirname, '../src', file), 'utf8').replaceAll('import.meta.env', '({})'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS }
  }).outputText
  const exports = {}
  vm.runInNewContext(code, { exports, Array, console, ...globals, require(name) {
    if (Object.hasOwn(mocks, name)) return mocks[name]
    assert.equal(name, '@/store/modules/user')
    return { default: () => actor }
  } })
  return exports
}
const auth = source('plugins/auth.ts').default
const utils = source('utils/permission.ts')
const directive = source('directive/permission/hasRole.ts').default
function visible(roles) {
  let removed = false
  directive.mounted({ parentNode: { removeChild() { removed = true } } }, { value: roles })
  return !removed
}
for (const [name, roles, permissions, common, admin, other] of [
  ['ordinary user', ['common'], ['member:read'], true, false, false],
  ['administrator inherits user', ['admin', 'common'], ['member:read', 'system:manage'], true, true, false],
  ['disabled base role', ['admin'], ['system:manage'], false, true, false],
  ['revoked roles', [], [], false, false, false],
  ['superuser', ['admin', 'common'], ['*:*:*'], true, true, true],
  ['superuser without assigned roles', [], ['*:*:*'], true, true, true]
]) {
  test(name, () => {
    actor = { roles, permissions }
    for (const [role, allowed] of [['common', common], ['admin', admin], ['other', other]]) {
      assert.equal(auth.hasRole(role), allowed)
      assert.equal(auth.hasRoleOr(['missing', role]), allowed)
      assert.equal(auth.hasRoleAnd([role]), allowed)
      assert.equal(utils.checkRole([role]), allowed)
      assert.equal(visible([role]), allowed)
    }
    assert.equal(auth.hasPermi('member:read'), permissions.includes('member:read') || permissions.includes('*:*:*'))
    assert.equal(utils.checkPermi(['system:manage']), permissions.includes('system:manage') || permissions.includes('*:*:*'))
    assert.equal(auth.hasRoleOr([]), false)
  })
}

test('refresh clears permissions when all roles are revoked', async () => {
  const store = source('store/modules/user.ts', {
    '@/router': { default: {} },
    '@/plugins/cache': { default: { session: { set() {} } } },
    'element-plus': {},
    '@/api/login': { getInfo: async () => ({ user: {}, roles: [], permissions: [] }) },
    '@/utils/auth': { getToken: () => '' },
    '@/utils/validate': { isHttp: () => false, isEmpty: () => true },
    '@/store/modules/lock': {},
    '@/assets/images/profile.jpg': { default: '' }
  }, { defineStore: (_, options) => options }).default
  const state = { ...store.state(), roles: ['admin'], permissions: ['*:*:*'] }
  await store.actions.getInfo.call(state)
  assert.equal(state.permissions.length, 0)
  assert.equal(state.roles.includes('admin'), false)
})
