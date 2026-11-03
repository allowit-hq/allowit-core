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

test('the packed public subpath has usable types and contains no guest source', async () => {
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
    ].sort());
    const packageDir = join(scratch, 'node_modules', '@allowit', 'sdk');
    await mkdir(packageDir, { recursive: true });
    run('tar', ['-xzf', tarball, '--strip-components=1', '-C', packageDir]);
    await writeFile(join(scratch, 'package.json'), JSON.stringify({ private: true, type: 'module' }));
    await cp(join(root, 'tests/presentation/consumer.ts'), join(scratch, 'consumer.ts'));
    const tsc = process.env.ALLOWIT_TEST_TSC ?? join(root, 'node_modules/typescript/bin/tsc');
    run(process.execPath, [tsc, '--noEmit', '--strict', '--target', 'ES2022', '--module', 'NodeNext', '--moduleResolution', 'NodeNext', join(scratch, 'consumer.ts')], { cwd: scratch });
    await writeFile(join(scratch, 'consumer.mjs'), `import { canonicalFunctions, canonicalFunctionsSource, canonicalFunctionMetadataHash } from '@allowit/sdk/core';\nconsole.log(JSON.stringify({ names: canonicalFunctions.map(item => item.name), source: canonicalFunctionsSource, hash: canonicalFunctionMetadataHash }));\n`);
    const actual = JSON.parse(run(process.execPath, [join(scratch, 'consumer.mjs')], { cwd: scratch }));
    const source = await readFile(join(root, 'typescript/canonical-functions.json'), 'utf8');
    assert.equal(actual.source, source);
    assert.equal(actual.hash, createHash('sha256').update(source).digest('hex'));
    assert.equal(actual.names.length, 9);
    const { canonicalFunctionRegistry } = await import('../../typescript/core.js');
    assert(Object.isFrozen(canonicalFunctionRegistry));
    assert(Object.isFrozen(canonicalFunctionRegistry.functions[0].parameter));
    assert.throws(() => { canonicalFunctionRegistry.functions[0].description = 'changed'; }, TypeError);
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }
});
