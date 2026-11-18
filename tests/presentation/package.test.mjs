import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFile, mkdtemp, mkdir, writeFile, cp, rm } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
function run(command, args, options = {}) {
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', ...options });
  assert.equal(result.status, 0, `${command} ${args.join(' ')}\n${result.stderr}\n${result.stdout}`);
  return result.stdout;
}
const cargo = ['+1.98.0', 'run', '--quiet', '--locked', '--offline', '--example', 'policy_wire', '--features', 'policy-source'];

test('Rust wire values survive a JavaScript roundtrip without integer rounding', async () => {
  const packet = JSON.parse(run('cargo', cargo));
  assert.equal(packet.values[0].Int, '-170141183460469231731687303715884105728');
  assert.equal(packet.values[1].Int, '170141183460469231731687303715884105727');
  assert.equal(packet.values[6].List[0].Int, '9007199254740993');
  assert.equal(packet.values[5].Addr, '0xabc123');
  const wire = JSON.stringify(packet.values);
  assert.deepEqual(JSON.parse(run('cargo', [...cargo, '--', 'roundtrip'], { input: wire })), packet.values);
  const { canonicalFunctionMetadataHash } = await import('../../typescript/core.js');
  assert.equal(packet.metadataHash, canonicalFunctionMetadataHash);
});

test('the packed public entry points have usable types and contain no guest source', async () => {
  const scratch = await mkdtemp(join(tmpdir(), 'allowit-core-package-'));
  try {
    run('pnpm', ['--config.verify-deps-before-run=false', 'check']);
    const packed = JSON.parse(run('pnpm', ['pack', '--json', '--pack-destination', scratch]));
    const packageInfo = Array.isArray(packed) ? packed[0] : packed;
    const tarball = resolve(scratch, packageInfo.filename ?? packageInfo.name);
    const contents = run('tar', ['-tzf', tarball]).trim().split('\n').sort();
    assert.deepEqual(contents, [
      'package/LICENSE', 'package/README.md', 'package/package.json',
      'package/typescript/canonical-functions.json', 'package/typescript/core.d.ts', 'package/typescript/core.js',
      'package/typescript/error-catalogue.json',
    ].sort());
    const packageDir = join(scratch, 'node_modules', '@allowit', 'core');
    await mkdir(packageDir, { recursive: true });
    run('tar', ['-xzf', tarball, '--strip-components=1', '-C', packageDir]);
    await writeFile(join(scratch, 'package.json'), JSON.stringify({ private: true, type: 'module' }));
    await cp(join(root, 'tests/presentation/consumer.ts'), join(scratch, 'consumer.ts'));
    const tsc = process.env.ALLOWIT_TEST_TSC ?? join(root, 'node_modules/typescript/bin/tsc');
    run(process.execPath, [tsc, '--noEmit', '--strict', '--target', 'ES2022', '--module', 'NodeNext', '--moduleResolution', 'NodeNext', join(scratch, 'consumer.ts')], { cwd: scratch });
    await writeFile(join(scratch, 'consumer.mjs'), `import assert from 'node:assert/strict';\nimport { canonicalFunctions, canonicalFunctionsSource, canonicalFunctionMetadataHash } from '@allowit/core';\nimport { canonicalFunctions as alias } from '@allowit/core/core';\nassert.strictEqual(canonicalFunctions, alias);\nconsole.log(JSON.stringify({ names: canonicalFunctions.map(item => item.name), source: canonicalFunctionsSource, hash: canonicalFunctionMetadataHash }));\n`);
    const actual = JSON.parse(run(process.execPath, [join(scratch, 'consumer.mjs')], { cwd: scratch }));
    const source = await readFile(join(root, 'typescript/canonical-functions.json'), 'utf8');
    assert.equal(actual.source, source);
    assert.equal(actual.hash, createHash('sha256').update(source).digest('hex'));
    assert.equal(actual.names.length, 9);
    await writeFile(join(scratch, 'errors.mjs'), `import { errorCatalogue, errorCatalogueSource, errorEntry, errorHelpPath } from '@allowit/core';\nconsole.log(JSON.stringify({ codes: errorCatalogue.entries.map(entry => entry.code), source: errorCatalogueSource, denied: errorEntry('policy_denied'), path: errorHelpPath('policy_denied'), frozen: Object.isFrozen(errorCatalogue.entries[0]) }));\n`);
    const errors = JSON.parse(run(process.execPath, [join(scratch, 'errors.mjs')], { cwd: scratch }));
    const errorSource = await readFile(join(root, 'typescript/error-catalogue.json'), 'utf8');
    assert.equal(errors.source, errorSource);
    assert.equal(errors.source, await readFile(join(root, 'crates/allowit-core/resources/error-catalogue.json'), 'utf8'));
    assert.deepEqual(errors.codes, JSON.parse(errorSource).entries.map(entry => entry.code));
    assert.equal(new Set(errors.codes).size, errors.codes.length);
    assert.equal(errors.denied.for, 'agent');
    assert.equal(errors.path, '/help/policy_denied');
    assert.equal(errors.frozen, true);
    const { canonicalFunctionRegistry } = await import('../../typescript/core.js');
    assert(Object.isFrozen(canonicalFunctionRegistry));
    assert(Object.isFrozen(canonicalFunctionRegistry.functions[0].parameter));
    assert.throws(() => { canonicalFunctionRegistry.functions[0].description = 'changed'; }, TypeError);
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }
});

test('the generator refuses catalogue text that a Rust host or a terminal cannot show', async () => {
  const { checkErrorCatalogue } = await import('../../scripts/error-catalogue.mjs');
  const catalogue = JSON.parse(await readFile(join(root, 'typescript/error-catalogue.json'), 'utf8'));
  assert.equal(checkErrorCatalogue(catalogue), catalogue);
  const entry = { code: 'some_code', for: 'owner', title: 'The wallet did not sign', cause: 'A cause.', next: 'A step.' };
  const one = change => ({ schemaVersion: 1, helpPath: '/help/', entries: [{ ...entry, ...change }] });
  assert.doesNotThrow(() => checkErrorCatalogue(one({})));
  for (const change of [
    { cause: 'A lone surrogate \ud800 that serde_json refuses.' },
    { cause: 'A C1 control \u0085 here.' },
    { title: 'Two lines' },
    { title: 'Reversed ‮ text' },
    { next: 'A byte order mark ﻿ here.' },
    { cause: 'Two\nlines.' },
    { title: 'Your wallet did not sign' },
    { cause: 'The signature that you sent is invalid.' },
    { title: 'A title.' },
    { title: 'x'.repeat(71) },
    { code: 'Bad-Code' },
    { for: 'someone' },
    { extra: 'field' },
  ]) {
    assert.throws(() => checkErrorCatalogue(one(change)), Error, JSON.stringify(change));
  }
  assert.throws(() => checkErrorCatalogue({ schemaVersion: 1, helpPath: '/help/', entries: [entry, entry] }), /two entries/);
  // An underscore ends a word, as in the Rust test.
  assert.throws(() => checkErrorCatalogue(one({ cause: 'The key that _you_ supplied did not match.' })), /name no reader/);
});

test('the catalogue file is the canonical text of what it parses to', async () => {
  const { checkErrorCatalogueSource } = await import('../../scripts/error-catalogue.mjs');
  const source = await readFile(join(root, 'typescript/error-catalogue.json'), 'utf8');
  assert.doesNotThrow(() => checkErrorCatalogueSource(source));
  const first = source.indexOf('{"code": ');
  const end = source.indexOf('}', first) + 1;
  const entry = source.slice(first, end);
  const variants = [
    // A second title: JSON.parse keeps the valid one, and serde_json refuses the lone surrogate of the first.
    entry.replace('"title": ', '"title": "A \\ud800 title", "title": '),
    // An escape that reads as a letter.
    entry.replace('"title": "', '"title": "\\u0054'),
    // Fields out of order.
    entry.replace(/"for": "(\w+)", /, '').replace('{', '{"for": "owner", '),
  ];
  for (const variant of variants) {
    assert.notEqual(variant, entry);
    assert.throws(() => checkErrorCatalogueSource(source.slice(0, first) + variant + source.slice(end)), /canonical form/, variant);
  }
});

test('each generated copy keeps the line endings of its source on every checkout', () => {
  const eol = path => run('git', ['check-attr', 'eol', '--', path]).trim().split(': ').pop();
  for (const [source, copy] of [
    ['typescript/error-catalogue.json', 'crates/allowit-core/resources/error-catalogue.json'],
    ['typescript/canonical-functions.json', 'crates/allowit-core/resources/canonical-functions.json'],
  ]) {
    assert.equal(eol(source), 'lf', source);
    assert.equal(eol(copy), eol(source), copy);
  }
});
