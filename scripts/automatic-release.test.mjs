// SPDX-License-Identifier: Apache-2.0
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, sep } from 'node:path';
import test from 'node:test';
import { api, findRelease, git, inspectTags, mayBecomeLatest, notes, publishRelease, releasePlan, REQUIRED_ASSETS, validateRun, verifyPublished, verifyReleaseById } from './automatic-release.mjs';

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'meltemi-release-test-'));
  t.after(() => {
    assert.ok(resolve(root).startsWith(resolve(tmpdir()) + sep + 'meltemi-release-test-'));
    rmSync(root, { recursive: true, force: true });
  });
  git(root, 'init', '--initial-branch=main');
  git(root, 'config', 'user.email', 'fixture@example.invalid');
  git(root, 'config', 'user.name', 'Release fixture');
  let n = 0;
  function commit(version = '0.2.0', subject = `change ${++n}`) {
    writeFileSync(join(root, 'Cargo.toml'), `[workspace.package]\nversion = "${version}"\n`);
    writeFileSync(join(root, 'change.txt'), subject);
    git(root, 'add', '.'); git(root, 'commit', '-m', subject);
    git(root, 'update-ref', 'refs/remotes/origin/main', 'HEAD');
    return git(root, 'rev-parse', 'HEAD');
  }
  const initial = commit('0.2.0', 'historical release');
  git(root, 'tag', 'v0.2.0');
  return { root, initial, commit,
    reserve(plan) { const { reused, ...metadata } = plan; git(root, 'tag', '-a', plan.tag, plan.sourceSha, '-m', JSON.stringify(metadata)); },
    plan(sha, version = '0.2.0') { git(root, 'checkout', '--detach', sha); return releasePlan(root, sha, 100, version); },
  };
}

const workflow = { id: 12, path: '.github/workflows/ci.yml' };
const valid = { id: 100, repository: { id: 1, full_name: 'owner/repo' },
  head_repository: { id: 1, full_name: 'owner/repo' }, workflow_id: 12,
  path: workflow.path, event: 'push', head_branch: 'main', status: 'completed',
  conclusion: 'success', head_sha: 'a'.repeat(40) };
// Scenario: CI de una PR no publica
// Scenario: Recuperación valida el CI
test('only successful same-repository CI push main is publication authority', () => {
  assert.equal(validateRun(valid, workflow, 'owner/repo', valid.head_sha), valid.head_sha);
  for (const patch of [{ event: 'pull_request' }, { conclusion: 'failure' },
    { status: 'in_progress' }, { head_branch: 'feature' }, { workflow_id: 99 },
    { path: '.github/workflows/other.yml' }, { head_sha: 'main' },
    { head_repository: { id: 2, full_name: 'owner/repo' } },
    { head_repository: { id: 2, full_name: 'attacker/fork' } }]) {
    assert.throws(() => validateRun({ ...valid, ...patch }, workflow, 'owner/repo'));
  }
  assert.throws(() => validateRun(valid, workflow, 'owner/repo', 'b'.repeat(40)));
});
// Scenario: Reintento conserva versión
test('a source SHA reserves one version and a failed attempt reuses it', t => {
  const f = fixture(t); const sha = f.commit(); const plan = f.plan(sha);
  assert.equal(plan.version, '0.2.1'); assert.equal(plan.baseTag, 'v0.2.0');
  f.reserve(plan);
  const retry = f.plan(sha);
  assert.equal(retry.version, plan.version); assert.equal(retry.reused, true);
  assert.equal(inspectTags(f.root).length, 2);
});
// Scenario: Reintento antiguo no retrocede latest
test('out of order CI receives source-ordered numbers, unaffected by descendant tags', t => {
  const f = fixture(t); const older = f.commit(); const newer = f.commit();
  const newPlan = f.plan(newer); assert.equal(newPlan.version, '0.2.2'); f.reserve(newPlan);
  const oldPlan = f.plan(older); assert.equal(oldPlan.version, '0.2.1'); f.reserve(oldPlan);
  assert.equal(f.plan(newer).version, '0.2.2');
  assert.equal(mayBecomeLatest(f.root, oldPlan, { tag_name: newPlan.tag }), false);
});
test('a new declared minor starts at that exact version, then advances with source distance', t => {
  const f = fixture(t); const intro = f.commit('0.3.0'); const next = f.commit('0.3.0');
  assert.equal(f.plan(next, '0.3.0').version, '0.3.1');
  const first = f.plan(intro, '0.3.0'); assert.equal(first.version, '0.3.0'); f.reserve(first);
  assert.equal(f.plan(next, '0.3.0').version, '0.3.1');
});
test('historical tags and collisions cannot silently republish different contents', t => {
  const f = fixture(t); assert.throws(() => f.plan(f.initial), /Historical/);
  const sha = f.commit();
  git(f.root, 'checkout', '--detach', f.initial);
  writeFileSync(join(f.root, 'collision.txt'), 'unrelated source');
  git(f.root, 'add', '.'); git(f.root, 'commit', '-m', 'another lineage owns this number');
  git(f.root, 'tag', 'v0.2.1');
  assert.throws(() => f.plan(sha), /already reserved/);
});
test('a changed declared patch cannot regress below an earlier automatic release', t => {
  const f = fixture(t);
  git(f.root, 'tag', 'v0.2.50');
  f.commit('0.2.0'); const sha = f.commit('0.2.1');
  assert.equal(f.plan(sha, '0.2.1').version, '0.2.52');
});
test('a historical release resets the distance from a much older declared base', t => {
  const f = fixture(t);
  for (let i = 0; i < 4; i++) f.commit();
  git(f.root, 'tag', 'v0.2.1');
  assert.equal(f.plan(f.commit()).version, '0.2.2');
});
test('source must be exact checkout and belong to main', t => {
  const f = fixture(t); const main = f.commit();
  assert.throws(() => releasePlan(f.root, f.initial, 100, '0.2.0'), /checkout/);
  git(f.root, 'checkout', '--detach', f.initial);
  writeFileSync(join(f.root, 'side.txt'), 'side'); git(f.root, 'add', '.'); git(f.root, 'commit', '-m', 'unmerged side');
  const side = git(f.root, 'rev-parse', 'HEAD');
  assert.throws(() => releasePlan(f.root, side, 100, '0.2.0'), /ancestor/);
  assert.equal(git(f.root, 'rev-parse', 'origin/main'), main);
});
// Scenario: Fuente y binario son reproducibles
test('notes contain only the reserved range and preserve its base on retry', t => {
  const f = fixture(t); const sha = f.commit('0.2.0', 'Fix a real bug'); const plan = f.plan(sha);
  f.reserve(plan); const text = notes(f.root, f.plan(sha));
  assert.match(text, /Fix a real bug/); assert.doesNotMatch(text, /historical release/);
  assert.match(text, new RegExp(sha));
});
test('a newer version from a non-descendant cannot become latest', t => {
  const f = fixture(t); const sha = f.commit(); const plan = f.plan(sha);
  git(f.root, 'checkout', '--detach', f.initial);
  writeFileSync(join(f.root, 'other.txt'), 'different lineage'); git(f.root, 'add', '.'); git(f.root, 'commit', '-m', 'side lineage');
  git(f.root, 'tag', 'v0.1.9');
  assert.equal(mayBecomeLatest(f.root, plan, { tag_name: 'v0.1.9' }), false);
  assert.equal(mayBecomeLatest(f.root, plan, { tag_name: 'v0.2.0' }), true);
});
test('upload failure leaves a reusable draft; published retries perform no mutation', t => {
  const f = fixture(t); const plan = f.plan(f.commit());
  let draft = null, creates = 0, uploads = 0, finishes = 0;
  const remote = {
    release: () => draft, latest: () => ({ tag_name: 'v0.2.0' }),
    create: () => { creates++; return (draft = { id: 1, draft: true }); },
    upload: () => { if (++uploads === 1) throw new Error('network failure'); },
    finish: (_id, promote) => { assert.equal(promote, true); finishes++; draft.draft = false; },
    verify: (id, _tag, expectedDraft) => { assert.equal(id, draft.id); assert.equal(draft.draft, expectedDraft); },
    verifyExisting: () => assert.equal(draft.draft, false),
  };
  assert.throws(() => publishRelease(f.root, plan, remote), /network failure/);
  assert.equal(draft.draft, true);
  assert.equal(publishRelease(f.root, plan, remote).published, true);
  assert.equal(publishRelease(f.root, plan, remote).reused, true);
  assert.deepEqual([creates, uploads, finishes], [1, 2, 1]);
});
// Scenario: Borrador no visible por tag
test('draft lookup includes later pages and rejects duplicate identities', () => {
  const unrelated = Array.from({ length: 100 }, (_, i) => ({ id: i + 1, tag_name: `v1.0.${i}` }));
  const draft = { id: 101, tag_name: 'v0.2.1', draft: true };
  const calls = [];
  const request = (_repo, _method, endpoint, _body, missing) => {
    calls.push(endpoint);
    if (endpoint.startsWith('releases/tags/')) { assert.equal(missing, true); return null; }
    return endpoint.endsWith('page=1') ? unrelated : [draft];
  };
  assert.equal(findRelease('owner/repo', draft.tag_name, request), draft);
  assert.deepEqual(calls, ['releases/tags/v0.2.1', 'releases?per_page=100&page=1', 'releases?per_page=100&page=2']);
  unrelated[0] = { ...draft, id: 102 };
  assert.throws(() => findRelease('owner/repo', draft.tag_name, request), /Multiple releases/);
  assert.equal(findRelease('owner/repo', draft.tag_name, () => ({ ...draft, draft: false })).draft, false);
  assert.throws(() => findRelease('owner/repo', draft.tag_name, () => { throw new Error('HTTP 403'); }), /HTTP 403/);
});
// Scenario: Publicación conserva la identidad del borrador
test('the real API draft visibility contract survives interrupted publication', t => {
  const f = fixture(t); const plan = f.plan(f.commit());
  let release = null, creates = 0, uploads = 0, finishes = 0;
  const verifiedStates = [];
  const request = (_repo, _method, endpoint, _body, missing) => {
    if (endpoint === `releases/tags/${plan.tag}`) {
      if (release && !release.draft) return release;
      if (missing) return null;
      throw new Error('HTTP 404: draft is not visible by tag');
    }
    if (endpoint === 'releases?per_page=100&page=1') return release ? [release] : [];
    assert.equal(endpoint, 'releases/45');
    verifiedStates.push(release.draft);
    return release;
  };
  const remote = {
    release: tag => findRelease('owner/repo', tag, request),
    latest: () => ({ tag_name: 'v0.2.0' }),
    create: () => { creates++; return (release = { id: 45, tag_name: plan.tag, draft: true, assets: [] }); },
    upload: () => { uploads++; release.assets = REQUIRED_ASSETS.map(name => ({ name, size: 10 })); },
    verify: (id, tag, draft) => verifyReleaseById('owner/repo', id, tag, draft, REQUIRED_ASSETS, request),
    finish: id => { assert.equal(id, 45); if (++finishes === 1) throw new Error('interrupted before finalization'); release.draft = false; },
    verifyExisting: existing => assert.equal(existing.draft, false),
  };
  assert.throws(() => publishRelease(f.root, plan, remote), /interrupted/);
  assert.equal(publishRelease(f.root, plan, remote).published, true);
  assert.equal(publishRelease(f.root, plan, remote).reused, true);
  assert.deepEqual([creates, uploads, finishes], [1, 2, 2]);
  assert.deepEqual(verifiedStates, [true, true, false]);
});
test('verification by id rejects a different release or incomplete uploads', () => {
  const release = { id: 45, tag_name: 'v0.2.1', draft: true, assets: [{ name: 'file', size: 10 }] };
  const verify = value => verifyReleaseById('owner/repo', 45, 'v0.2.1', true, ['file'], (_repo, _method, endpoint) => {
    assert.equal(endpoint, 'releases/45'); return value;
  });
  assert.equal(verify(release), release);
  for (const patch of [{ id: 46 }, { tag_name: 'v0.2.2' }, { draft: false }, { assets: [] },
    { assets: [{ name: 'file', size: 0 }] }, { assets: [...release.assets, { name: 'unexpected', size: 10 }] }]) {
    assert.throws(() => verify({ ...release, ...patch }), /identity, state or uploaded assets/);
  }
});
test('API failures expose only HTTP status and distinguish missing from denied', () => {
  const fail = status => () => { throw Object.assign(new Error('private payload'), { stderr: `secret-token and signed-url (HTTP ${status})` }); };
  assert.equal(api('owner/repo', 'GET', 'releases/tags/v0.2.1', null, true, fail(404)), null);
  for (const status of [403, 404, 500]) {
    assert.throws(() => api('owner/repo', 'GET', 'releases/45', null, false, fail(status)), error => {
      assert.match(error.message, new RegExp(`HTTP ${status}`));
      assert.doesNotMatch(error.message, /secret-token|signed-url|private payload/);
      return true;
    });
  }
  assert.throws(() => api('owner/repo', 'GET', 'releases/45', null, true, fail(500)), /HTTP 500/);
});
test('a published retry fails closed on missing assets, mismatched manifest or digest', () => {
  const manifest = { tag: 'v0.2.1', sourceSha: 'a'.repeat(40) };
  const assets = REQUIRED_ASSETS.map((name, id) => ({ name, id: id + 1, size: 10, digest: `sha256:${'a'.repeat(64)}` }));
  const release = { tag_name: manifest.tag, draft: false, assets };
  const read = asset => asset.name === 'release-manifest.json' ? JSON.stringify(manifest) :
    assets.filter(a => !['SHA256SUMS', 'attestation.sigstore.json'].includes(a.name))
      .map(a => `${'a'.repeat(64)}  ${a.name}`).join('\n');
  verifyPublished(release, manifest, read);
  assert.throws(() => verifyPublished({ ...release, assets: assets.slice(1) }, manifest, read), /incomplete/);
  assert.throws(() => verifyPublished(release, { ...manifest, sourceSha: 'b'.repeat(40) }, read), /manifest differs/);
  assert.throws(() => verifyPublished({ ...release, assets: assets.map((a, i) => i ? a : { ...a, digest: 'sha256:wrong' }) }, manifest, read), /checksum disagrees/);
});
// Scenario: Pushes rápidos conservados
test('the workflow validates authority before source checkout and never stamps main', () => {
  const wf = readFileSync(new URL('../.github/workflows/release.yml', import.meta.url), 'utf8');
  assert.match(wf, /workflow_run:/); assert.match(wf, /queue: max/);
  assert.match(wf, /ref: \$\{\{ needs\.authorize\.outputs\.sha \}\}/);
  assert.match(wf, /ref: \$\{\{ github\.workflow_sha \}\}/);
  assert.match(wf, /node scripts\/automatic-release\.mjs authorize/);
  assert.doesNotMatch(wf, /git push|cancel-in-progress: true|ref: main/);
  const ci = readFileSync(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8');
  assert.match(ci, /queue: max/); assert.doesNotMatch(ci, /cancel-in-progress: true/);
});
