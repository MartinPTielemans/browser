// Saves the Mac engine's build tree as artifacts of this run, or restores it from another run's
// (engine-mac.yml): a tar stream, compressed with zstd and cut into parts, each an unzipped
// artifact of its own. A part is uploaded while the next is made, and downloaded while the last is
// unpacked, so the disk holds the tree and two parts, never the whole archive. tar keeps the
// files' dates, which is what lets ninja rebuild only what changed. The manifest goes last, so a
// run that has one has every part.
//
//   node tree.mjs save <root> <path>...   saves <path>s, relative to <root>
//   node tree.mjs restore <root> <run>    unpacks run <run>'s tree into <root>
//
// TREE_NAME names the artifacts, mac-tree by default. TREE_RETENTION_DAYS sets how long they stay,
// 90 by default, the most a public repository keeps them. Restoring from another run needs GITHUB_TOKEN with actions: read.
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {createReadStream} from 'node:fs';
import {mkdir, readdir, readFile, rm, stat, writeFile} from 'node:fs/promises';
import {join} from 'node:path';
import {DefaultArtifactClient} from '@actions/artifact';

const NAME = process.env.TREE_NAME ?? 'mac-tree';
const PREFIX = `${NAME}.tar.zst.`;
const MANIFEST = `${NAME}.manifest`;
// 4 GB parts: a 50 GB tree in 13, well under a job's 500 artifacts.
const PART_BYTES = 4 << 30;
const artifacts = new DefaultArtifactClient();
const work = join(process.env.RUNNER_TEMP ?? '/tmp', 'mac-tree');
const seconds = start => Math.round((Date.now() - start) / 1000);

function sha256(file) {
  return new Promise((resolve, reject) => {
    const hash = createHash('sha256');
    createReadStream(file).on('data', d => hash.update(d)).on('end', () => resolve(hash.digest('hex'))).on('error', reject);
  });
}

function run(script, args, stdio) {
  const child = spawn('bash', ['-o', 'pipefail', '-c', script, 'tree', ...args], {stdio});
  const done = new Promise((resolve, reject) =>
    child.on('exit', code => (code === 0 ? resolve() : reject(new Error(`${script.split(' ')[0]}… exited with ${code}`)))));
  return {child, done};
}

async function save(root, paths) {
  const start = Date.now();
  const retentionDays = Number(process.env.TREE_RETENTION_DAYS ?? 90);
  await rm(work, {recursive: true, force: true});
  await mkdir(work, {recursive: true});
  // gsplit names a part only once it's whole, and waits while two wait for upload, which holds
  // tar and zstd back too.
  const filter = `while [ "$(ls "${work}" | grep -c '^${PREFIX}[0-9]*$')" -ge 2 ]; do sleep 1; done; cat > "$FILE.part" && mv "$FILE.part" "$FILE"`;
  const pack = run(
    `tar --no-mac-metadata -C "$1" -cf - "\${@:2}" | zstd -q -T0 -3 | gsplit -a 3 -d -b ${PART_BYTES} --filter='${filter}' - "${work}/${PREFIX}"`,
    [root, ...paths],
    'inherit',
  );
  let packed = false;
  pack.done.then(() => (packed = true), () => (packed = true));
  const parts = [];
  let bytes = 0;
  for (;;) {
    const ready = (await readdir(work)).filter(n => n.startsWith(PREFIX) && /\.\d+$/.test(n)).sort();
    if (ready.length === 0) {
      if (packed) break;
      await new Promise(r => setTimeout(r, 1000));
      continue;
    }
    const name = ready[0];
    const file = join(work, name);
    const size = (await stat(file)).size;
    const hash = await sha256(file);
    await artifacts.uploadArtifact(name, [file], work, {skipArchive: true, retentionDays});
    await rm(file);
    parts.push({name, size, sha256: hash});
    bytes += size;
    console.log(`tree: ${name} up, ${(size / 2 ** 30).toFixed(1)} GB, ${seconds(start)} s in`);
  }
  await pack.done;
  const manifest = join(work, MANIFEST);
  await writeFile(manifest, JSON.stringify({commit: process.env.GITHUB_SHA, root, paths, parts}, null, 1));
  await artifacts.uploadArtifact(MANIFEST, [manifest], work, {skipArchive: true, retentionDays});
  console.log(`tree: saved ${(bytes / 2 ** 30).toFixed(1)} GB in ${parts.length} parts, in ${seconds(start)} s`);
}

async function restore(root, runId) {
  const start = Date.now();
  const findBy = String(runId) === process.env.GITHUB_RUN_ID ? undefined : {
    token: process.env.GITHUB_TOKEN,
    workflowRunId: Number(runId),
    repositoryOwner: process.env.GITHUB_REPOSITORY.split('/')[0],
    repositoryName: process.env.GITHUB_REPOSITORY.split('/')[1],
  };
  const {artifacts: listed} = await artifacts.listArtifacts(findBy ? {findBy} : {});
  const byName = new Map(listed.map(a => [a.name, a]));
  const download = async name => {
    const artifact = byName.get(name);
    if (!artifact) throw new Error(`run ${runId} has no ${name}`);
    const dir = join(work, String(artifact.id));
    await mkdir(dir, {recursive: true});
    await artifacts.downloadArtifact(artifact.id, {path: dir, findBy, expectedHash: artifact.digest});
    return join(dir, name);
  };
  await rm(work, {recursive: true, force: true});
  const {parts} = JSON.parse(await readFile(await download(MANIFEST), 'utf8'));
  await mkdir(root, {recursive: true});
  const unpack = run('zstd -q -d -T0 | tar -C "$1" -xf -', [root], ['pipe', 'inherit', 'inherit']);
  let next = download(parts[0].name);
  for (let i = 0; i < parts.length; i++) {
    const file = await next;
    if (i + 1 < parts.length) next = download(parts[i + 1].name);
    if ((await sha256(file)) !== parts[i].sha256) throw new Error(`${parts[i].name} isn't the part that was saved`);
    await new Promise((resolve, reject) => {
      const stream = createReadStream(file);
      stream.on('error', reject).on('end', resolve);
      stream.pipe(unpack.child.stdin, {end: false});
    });
    await rm(file);
    console.log(`tree: ${parts[i].name} unpacked, ${seconds(start)} s in`);
  }
  unpack.child.stdin.end();
  await unpack.done;
  console.log(`tree: restored ${parts.length} parts in ${seconds(start)} s`);
}

const [mode, root, ...rest] = process.argv.slice(2);
const job = mode === 'save' && rest.length ? save(root, rest) : mode === 'restore' && rest.length === 1 ? restore(root, rest[0]) : null;
if (!job) {
  console.error('usage: tree.mjs save <root> <path>... | restore <root> <run>');
  process.exit(2);
}
job.catch(e => {
  console.error(`tree: ${e.message}`);
  process.exit(1);
});
