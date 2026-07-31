import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const catalogs = {
  en: JSON.parse(fs.readFileSync(path.join(root, 'crates/syu-app-ui/assets/locales/en.json'))),
  ja: JSON.parse(fs.readFileSync(path.join(root, 'crates/syu-app-ui/assets/locales/ja.json'))),
};
let locale = 'en';
const required = key => {
  const value = catalogs[locale][key];
  if (value === undefined) throw new Error(`Missing ${locale} translation: ${key}`);
  return value;
};

globalThis.window = {
  SyuPreferences: {
    t: required,
    lookup: key => catalogs[locale][key],
  },
};

const source = fs.readFileSync(path.join(root, 'crates/syu-app-ui/assets/js/i18n.js'));
const module = await import(`data:text/javascript;base64,${source.toString('base64')}`);
const { localizeEnum, localizeSpecificationTitle, translate } = module;

assert.throws(() => translate('specification.title.REQ-AUTH-001'));
assert.equal(
  localizeSpecificationTitle({ id: 'REQ-AUTH-001', title: 'Authentication behavior' }),
  'Authentication behavior',
);

locale = 'ja';
assert.equal(
  localizeSpecificationTitle({
    id: 'REQ-CAPABILITY-001',
    title: 'Canonical capability behavior',
    presentation_title_key: 'specification.title.REQ-CAPABILITY-001',
  }),
  '正式な能力の振る舞い',
);
assert.equal(
  localizeSpecificationTitle({
    id: 'REQ-CAPABILITY-001',
    title: 'ユーザーが編集した能力の説明',
  }),
  'ユーザーが編集した能力の説明',
);
assert.equal(localizeEnum('target.access', 'run-only'), '実行のみ');
assert.equal(localizeEnum('target.transition', 'run-only'), '実行のみ');
assert.equal(localizeEnum('operation', 'new-future-operation'), 'new-future-operation');

locale = 'en';
assert.equal(localizeEnum('operation', 'modify'), 'Modify');
locale = 'ja';
assert.equal(localizeEnum('operation', 'modify'), '変更');

console.log('Workbench semantic i18n runtime contract passed');
