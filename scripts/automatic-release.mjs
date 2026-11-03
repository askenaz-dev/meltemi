// SPDX-License-Identifier: Apache-2.0
// Release orchestration only: no dependency, token logging or shell evaluation.
import { execFileSync } from 'node:child_process';
import { appendFileSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const SCHEMA = 'meltemi-automatic-release/v1';
export const VERSION = /^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
export function compare(a, b) {
  const aa = a.match(VERSION), bb = b.match(VERSION);
  if (!aa || !bb) throw new Error('Expected a release version x.y.z');
  for (let i = 1; i <= 3; i++) {
    const delta = BigInt(aa[i]) - BigInt(bb[i]);
    if (delta) return delta > 0n ? 1 : -1;
  }
  return 0;
}
export function validateRun(run, workflow, repository, expectedSha) {
  if (run.repository?.full_name !== repository || run.head_repository?.full_name !== repository ||
      !Number.isSafeInteger(run.repository?.id) || run.repository.id < 1 ||
      run.repository?.id !== run.head_repository?.id || run.workflow_id !== workflow.id ||
      workflow.path !== '.github/workflows/ci.yml' || run.path !== workflow.path ||
      run.event !== 'push' || run.head_branch !== 'main' || run.status !== 'completed' ||
      run.conclusion !== 'success' || !/^[0-9a-f]{40}$/.test(run.head_sha) ||
      (expectedSha && run.head_sha !== expectedSha)) {
    throw new Error('Release requires successful completed ci.yml from a push to main in this repository');
  }
  return run.head_sha;
}
export function git(root, ...args) {
  return execFileSync('git', ['-C', root, ...args], { encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] }).trim();
}
export function ancestor(root, older, newer) {
  try { git(root, 'merge-base', '--is-ancestor', older, newer); return true; }
  catch (error) { if (error.status === 1) return false; throw error; }
}
export function inspectTags(root) {
  const names = git(root, 'tag', '--list', 'v*').split('\n').filter(x => VERSION.test(x));
  return names.map(name => {
    const sha = git(root, 'rev-parse', `${name}^{commit}`);
    let metadata = null;
    if (git(root, 'cat-file', '-t', `refs/tags/${name}`) === 'tag') {
      const object = git(root, 'cat-file', '-p', `refs/tags/${name}`);
      const body = object.slice(object.indexOf('\n\n') + 2);
      try { const value = JSON.parse(body); if (value.schema === SCHEMA) metadata = value; } catch {}
    }
    if (metadata && (metadata.sourceSha !== sha || `v${metadata.version}` !== name ||
        metadata.tag !== name || !VERSION.test(metadata.sourceVersion ?? '') ||
        !/^[1-9]\d*$/.test(metadata.ciRunId ?? '') ||
        (metadata.baseTag !== null && !/^v\d+\.\d+\.\d+$/.test(metadata.baseTag ?? '')))) {
      throw new Error(`Reservation metadata disagrees with immutable tag ${name}`);
    }
    return { name, sha, metadata };
  });
}
export function releasePlan(root, sourceSha, ciRunId, sourceVersion, tags = inspectTags(root)) {
  if (git(root, 'rev-parse', 'HEAD') !== sourceSha || !ancestor(root, sourceSha, 'origin/main')) {
    throw new Error('The checkout must be the approved CI SHA and an ancestor of origin/main');
  }
  const existing = tags.filter(tag => tag.sha === sourceSha);
  if (existing.length > 1) throw new Error('Multiple release tags already identify this source SHA');
  if (existing.length) {
    if (!existing[0].metadata) throw new Error('Historical release tag has no automatic reservation; it will not be rebuilt or renumbered');
    return { ...existing[0].metadata, reused: true };
  }
  // First-parent distance gives a commit its number regardless of CI completion
  // order. A descendant's tag must never renumber an older source on recovery.
  const history = git(root, 'rev-list', '--first-parent', sourceSha).split('\n');
  let anchorVersion = sourceVersion, anchorSha = sourceSha, distance = 0;
  const nearest = history.slice(1).map(sha => tags.filter(tag => tag.sha === sha)
    .sort((a, b) => compare(a.name, b.name)).at(-1)).find(Boolean);
  const baseTag = nearest?.name ?? null;
  for (let index = 1; index < history.length; index++) {
    let earlier;
    try { earlier = git(root, 'show', `${history[index]}:Cargo.toml`).match(/\[workspace\.package\][\s\S]*?\nversion = "([^"]+)"/)?.[1]; } catch {}
    if (earlier !== sourceVersion) break;
    anchorSha = history[index]; distance = index;
  }
  const advance = (version, count) => {
    const [major, minor, patch] = version.replace(/^v/, '').split('.');
    return `${major}.${minor}.${BigInt(patch) + BigInt(count)}`;
  };
  let version = advance(anchorVersion, distance);
  if (nearest) {
    const index = history.indexOf(nearest.sha);
    const candidate = advance(nearest.name, index);
    // A historical tag newer than the base introduction is authoritative: old
    // commits before that release must not be counted again. A newly declared
    // base competes with the tag candidate, so a stale patch cannot regress.
    if (index <= distance || compare(candidate, version) > 0) {
      version = candidate; anchorVersion = nearest.name.replace(/^v/, ''); anchorSha = nearest.sha; distance = index;
    }
  }
  if (tags.some(tag => compare(tag.name, version) === 0)) {
    throw new Error(`Version v${version} is already reserved by another source; tags are never moved`);
  }
  return { schema: SCHEMA, sourceSha, sourceVersion, version, tag: `v${version}`,
    ciRunId: String(ciRunId), baseTag, anchorSha, anchorVersion, distance, reused: false };
}
export function notes(root, plan) {
  const range = plan.baseTag ? `${plan.baseTag}..${plan.sourceSha}` : plan.sourceSha;
  const log = git(root, 'log', '--reverse', '--format=- %h %s', range);
  let curated = '';
  try {
    const changelog = readFileSync(resolve(root, 'CHANGELOG.md'), 'utf8');
    const section = changelog.split(/^## /m).slice(1).find(part => {
      const title = part.split('\n', 1)[0].match(/^v?(\d+\.\d+\.\d+)(?:\s|$)/);
      return title?.[1] === plan.version;
    });
    if (section) curated = `\n${section.slice(section.indexOf('\n') + 1).trim()}\n`;
  } catch (error) { if (error.code !== 'ENOENT') throw error; }
  return `# Meltemi ${plan.tag}\n\nSource: ${plan.sourceSha}\nCI run: ${plan.ciRunId}\n${curated}` +
    `\nChanges since ${plan.baseTag ?? 'the beginning of this repository'}:\n\n${log}\n` +
    '\nThe source tag retains its declared version; release-manifest.json records the build version stamp.\n';
}
export function mayBecomeLatest(root, plan, latest) {
  if (!latest) return true;
  if (!VERSION.test(latest.tag_name)) throw new Error('Latest release has an unsupported version tag');
  if (compare(plan.version, latest.tag_name) <= 0) return false;
  const previous = git(root, 'rev-parse', `${latest.tag_name}^{commit}`);
  return ancestor(root, previous, plan.sourceSha);
}
export const REQUIRED_ASSETS = [
  'meltemi-Windows.zip', 'meltemi-macOS.tar.gz', 'meltemi-Linux.tar.gz',
  'meltemi-desktop-Windows.msi', 'meltemi-desktop-macOS.dmg', 'meltemi-desktop-Linux.deb',
  'install.sh', 'install.ps1', 'SHA256SUMS', 'attestation.sigstore.json', 'release-manifest.json',
];
export function verifyPublished(release, manifest, readAsset) {
  if (release.draft || release.tag_name !== manifest.tag) throw new Error('Expected the published reserved release');
  const byName = new Map(release.assets.map(asset => [asset.name, asset]));
  for (const name of REQUIRED_ASSETS) {
    if (!(byName.get(name)?.size > 0)) throw new Error(`Published immutable release is incomplete: ${name}`);
  }
  const actual = JSON.parse(readAsset(byName.get('release-manifest.json')));
  if (JSON.stringify(actual) !== JSON.stringify(manifest)) throw new Error('Published manifest differs from its immutable reservation');
  const sums = new Map([...readAsset(byName.get('SHA256SUMS')).matchAll(/^([0-9a-f]{64}) [ *](.+)$/gm)]
    .map(match => [match[2].trim(), match[1]]));
  for (const [name, asset] of byName) {
    if (name === 'SHA256SUMS' || name === 'attestation.sigstore.json') continue;
    if (!sums.has(name) || (asset.digest && asset.digest !== `sha256:${sums.get(name)}`)) {
      throw new Error(`Published asset checksum disagrees with SHA256SUMS: ${name}`);
    }
  }
}
export function publishRelease(root, plan, remote) {
  const existing = remote.release(plan.tag);
  if (existing && !existing.draft) {
    remote.verifyExisting(existing);
    return { published: true, reused: true };
  }
  const promote = mayBecomeLatest(root, plan, remote.latest());
  const draft = existing ?? remote.create(plan);
  remote.upload(plan.tag);
  remote.verify(draft.id, plan.tag, true);
  remote.finish(draft.id, promote);
  remote.verify(draft.id, plan.tag, false);
  return { published: true, latest: promote, reused: Boolean(existing) };
}
export function findRelease(repository, tag, request = api) {
  // The tag endpoint only returns published releases, never drafts.
  const published = request(repository, 'GET', `releases/tags/${tag}`, null, true);
  if (published) return published;
  let found = null;
  for (let page = 1; ; page++) {
    const releases = request(repository, 'GET', `releases?per_page=100&page=${page}`);
    if (!Array.isArray(releases)) throw new Error('Expected a GitHub release list');
    for (const release of releases.filter(value => value.tag_name === tag)) {
      if (found) throw new Error(`Multiple releases identify ${tag}; refusing to choose a draft`);
      found = release;
    }
    if (releases.length < 100) return found;
  }
}
export function verifyReleaseById(repository, id, tag, draft, assets, request = api) {
  if (!Number.isSafeInteger(id) || id < 1) throw new Error('Invalid release id');
  const release = request(repository, 'GET', `releases/${id}`);
  if (release.id !== id || release.tag_name !== tag || release.draft !== draft ||
      release.assets.length !== assets.length ||
      assets.some(name => !release.assets.some(asset => asset.name === name && asset.size > 0))) {
    throw new Error('Release identity, state or uploaded assets are incomplete');
  }
  return release;
}
export function api(repository, method, endpoint, body, missing = false, execute = execFileSync) {
  try {
    const args = ['api', '--method', method, `repos/${repository}/${endpoint}`];
    if (body) args.push('--input', '-');
    const raw = execute('gh', args, { encoding: 'utf8', input: body ? JSON.stringify(body) : undefined,
      stdio: ['pipe', 'pipe', 'pipe'] });
    return raw.trim() ? JSON.parse(raw) : null;
  } catch (error) {
    const status = String(error.stderr).match(/\(HTTP (\d{3})\)/)?.[1];
    if (missing && status === '404') return null;
    throw new Error(`GitHub API ${method} ${endpoint} failed${status ? ` (HTTP ${status})` : ''} (no credentials are logged)`);
  }
}
function readAsset(repository, asset) {
  if (!Number.isSafeInteger(asset.id) || asset.id < 1) throw new Error('Invalid release asset id');
  try {
    return execFileSync('gh', ['api', `repos/${repository}/releases/assets/${asset.id}`,
      '-H', 'Accept: application/octet-stream'], { encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'] });
  } catch { throw new Error(`Could not verify published asset ${asset.name}`); }
}
function output(values) {
  for (const [key, value] of Object.entries(values)) {
    if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `${key}=${value}\n`);
    console.log(`${key}=${value}`);
  }
}
export async function main(command) {
  if (command === 'stamp') {
    const version = process.env.RELEASE_VERSION;
    if (!VERSION.test(version ?? '')) throw new Error('Expected RELEASE_VERSION');
    const args = ['run', '--quiet', '--locked', '-p', 'meltemi-release', '--'];
    const current = execFileSync('cargo', [...args, 'version'], { encoding: 'utf8' }).trim();
    if (current !== version) execFileSync('cargo', [...args, 'set-version', version], { stdio: 'inherit' });
    return;
  }
  const repository = process.env.GITHUB_REPOSITORY;
  if (!/^[\w.-]+\/[\w.-]+$/.test(repository ?? '')) throw new Error('Expected GITHUB_REPOSITORY');
  const root = process.cwd();
  if (command === 'authorize') {
    if (process.env.GITHUB_REF !== 'refs/heads/main') throw new Error('Recovery and publication run from main only');
    const id = process.env.CI_RUN_ID;
    if (!/^[1-9]\d*$/.test(id ?? '')) throw new Error('Expected a numeric CI run id');
    const run = api(repository, 'GET', `actions/runs/${id}`);
    const workflow = api(repository, 'GET', 'actions/workflows/ci.yml');
    const sha = validateRun(run, workflow, repository, process.env.EXPECTED_SHA);
    output({ sha, ci_run_id: id });
  } else if (command === 'reserve') {
    const sha = process.env.SOURCE_SHA;
    if (!/^[0-9a-f]{40}$/.test(sha ?? '')) throw new Error('Invalid approved source SHA');
    try { git(root, 'cat-file', '-e', `${sha}:scripts/automatic-release.mjs`); }
    catch { throw new Error('This CI run predates automatic releases; use a successful CI run after the automation was merged'); }
    git(root, 'fetch', '--tags', 'origin', 'refs/heads/main:refs/remotes/origin/main');
    const sourceVersion = process.env.SOURCE_VERSION;
    if (!VERSION.test(sourceVersion ?? '')) throw new Error('Expected the checked source version');
    const plan = releasePlan(root, sha, process.env.CI_RUN_ID, sourceVersion);
    const { reused, ...manifest } = plan;
    if (!reused) {
      const tag = api(repository, 'POST', 'git/tags', { tag: plan.tag, message: JSON.stringify(manifest), object: sha, type: 'commit' });
      api(repository, 'POST', 'git/refs', { ref: `refs/tags/${plan.tag}`, sha: tag.sha });
    }
    const release = findRelease(repository, plan.tag);
    if (release && !release.draft) verifyPublished(release, manifest, asset => readAsset(repository, asset));
    writeFileSync('release-manifest.json', JSON.stringify(manifest, null, 2) + '\n');
    writeFileSync('release-notes.md', notes(root, plan));
    output({ release: !release || release.draft, version: plan.version, tag: plan.tag, sha,
      published: Boolean(release && !release.draft), ci_run_id: plan.ciRunId });
  } else if (command === 'publish') {
    const plan = JSON.parse(readFileSync('release-assets/release-manifest.json', 'utf8'));
    if (git(root, 'rev-parse', 'HEAD') !== plan.sourceSha) throw new Error('Publication checkout differs from the reserved source');
    git(root, 'fetch', '--tags', 'origin', 'refs/heads/main:refs/remotes/origin/main');
    if (!ancestor(root, plan.sourceSha, 'origin/main')) throw new Error('Reserved source is no longer an ancestor of main');
    const reservation = inspectTags(root).find(t => t.name === plan.tag);
    if (!reservation?.metadata || reservation.sha !== plan.sourceSha ||
        JSON.stringify(reservation.metadata) !== JSON.stringify(plan)) throw new Error('Published manifest must match the immutable reservation');
    const assets = readdirSync('release-assets').map(name => `release-assets/${name}`);
    output(publishRelease(root, plan, {
      release: tag => findRelease(repository, tag),
      verifyExisting: release => verifyPublished(release, plan, asset => readAsset(repository, asset)),
      latest: () => api(repository, 'GET', 'releases/latest', null, true),
      create: value => api(repository, 'POST', 'releases', { tag_name: value.tag,
        target_commitish: value.sourceSha, name: `Meltemi ${value.tag}`, draft: true,
        body: readFileSync('notes/release-notes.md', 'utf8') }),
      upload: tag => execFileSync('gh', ['release', 'upload', tag, ...assets, '--clobber', '--repo', repository], { stdio: 'inherit' }),
      finish: (id, promote) => api(repository, 'PATCH', `releases/${id}`, { draft: false, make_latest: promote ? 'true' : 'false' }),
      verify: (id, tag, draft) => verifyReleaseById(repository, id, tag, draft,
        assets.map(path => path.split(/[\\/]/).at(-1))),
    }));
  } else throw new Error(`Unknown command: ${command}`);
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main(process.argv[2]).catch(error => { console.error(error.message); process.exitCode = 1; });
}
