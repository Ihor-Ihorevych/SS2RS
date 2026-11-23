// Validate .gltf/.glb files with the Khronos glTF-Validator (the reference implementation).
//
//   cd <some scratch folder> && npm install gltf-validator       # once, outside the repository
//   GLTF_VALIDATOR_DIR=<that folder> node validate_gltf.mjs <file-or-folder>... [--quiet]
//
// External images are read from disk next to the glTF file when they exist; a missing one is
// replaced by a 1 x 1 PNG and counted as "missing_images" (the exporter references the PNGs of
// the texture exporter, which may not have been written). Exit status: 0 if no file has an
// error, 1 otherwise. Prints one JSON summary at the end.
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';

const dir = process.env.GLTF_VALIDATOR_DIR;
if (!dir) {
  console.error('set GLTF_VALIDATOR_DIR to the folder where `npm install gltf-validator` was run');
  process.exit(2);
}
const validator = createRequire(path.join(path.resolve(dir), 'x.js'))('gltf-validator');
const quiet = process.argv.includes('--quiet');
const args = process.argv.slice(2).filter((a) => !a.startsWith('--'));

const ONE_PIXEL_PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==',
  'base64',
);

function* walk(p) {
  const st = fs.statSync(p);
  if (st.isDirectory()) {
    for (const n of fs.readdirSync(p).sort()) yield* walk(path.join(p, n));
  } else if (/\.(gltf|glb)$/i.test(p)) {
    yield p;
  }
}

const totals = { files: 0, with_errors: 0, errors: 0, warnings: 0, infos: 0, hints: 0, missing_images: 0 };
const codes = {};
const failures = [];
for (const arg of args) {
  for (const file of walk(arg)) {
    const bytes = new Uint8Array(fs.readFileSync(file));
    let missing = 0;
    const report = await validator.validateBytes(bytes, {
      uri: path.basename(file),
      maxIssues: 0,
      externalResourceFunction: async (uri) => {
        const p = path.join(path.dirname(file), decodeURIComponent(uri));
        if (fs.existsSync(p)) return new Uint8Array(fs.readFileSync(p));
        missing++;
        return new Uint8Array(ONE_PIXEL_PNG);
      },
    });
    const s = report.issues;
    totals.files++;
    totals.errors += s.numErrors;
    totals.warnings += s.numWarnings;
    totals.infos += s.numInfos;
    totals.hints += s.numHints;
    totals.missing_images += missing;
    for (const m of s.messages) codes[`${['error', 'warning', 'info', 'hint'][m.severity]}:${m.code}`] = (codes[`${['error', 'warning', 'info', 'hint'][m.severity]}:${m.code}`] || 0) + 1;
    if (s.numErrors > 0) {
      totals.with_errors++;
      if (failures.length < 20) failures.push({ file, messages: s.messages.filter((m) => m.severity === 0).slice(0, 3) });
    }
    if (!quiet) console.log(`${s.numErrors ? 'ERR ' : 'ok  '} ${file} e=${s.numErrors} w=${s.numWarnings} i=${s.numInfos} h=${s.numHints}`);
  }
}
console.log(JSON.stringify({ validator: validator.version(), totals, codes, failures }, null, 1));
process.exit(totals.with_errors ? 1 : 0);
