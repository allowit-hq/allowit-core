// The rules of the error catalogue. The generator refuses a catalogue that breaks one, before it writes the
// TypeScript or the Rust copy; `crates/allowit-core/src/errors.rs` tests the same rules on the Rust side.
//
// `typescript/error-catalogue.json` is the source, and every host reads the same text: the generator writes it to the
// Rust crate and to the npm package.
//
// An entry is a registry line: the code, its reader and a title that names the code. A host shows the title, then the
// sender's own message as the cause and the step, then the help link. A cause or a step in the catalogue is a claim
// about every sender of the code, so it comes only for a code on an allow-list, `causes` or `staticSteps`, one code per
// line, after a review against that code's senders: each addition is one line of a review.

export const AUDIENCES = ['owner', 'agent', 'operator'];
export const FIELDS = ['code', 'for', 'title', 'cause', 'next', 'agent'];
export const CODE = /^[a-z][a-z0-9_]{2,63}$/;
// Catalogue text is printable ASCII: letters, digits, the space and ASCII punctuation. Every reader, a page, a terminal
// or a Rust host, shows such text as it is written, and no hidden character, direction mark or combining mark can
// change what it shows.
const PRINTABLE = /^[\x20-\x7e]*$/;

/** The words of a text: runs of ASCII letters and digits, in lowercase. The Rust test splits text the same way. */
export const words = text => text.split(/[^A-Za-z0-9]+/).filter(Boolean).map(word => word.toLowerCase());
// A title and a cause name no reader: an owner and an agent read the same entry.
const READER_WORDS = new Set(['you', 'your', 'yours', 'yourself']);
export const namesReader = text => words(text).some(word => READER_WORDS.has(word));

/** Plain text: printable ASCII, trimmed, of 1 to `limit` characters. */
export function plain(text, limit) {
  return typeof text === 'string' && text.length > 0 && text.length <= limit && text === text.trim() && PRINTABLE.test(text);
}

/** Throws for the first rule that the source catalogue breaks. */
export function checkErrorCatalogue(errors) {
  if (errors?.schemaVersion !== 1 || errors.helpPath !== '/help/' || !Array.isArray(errors.entries) || !errors.entries.length) {
    throw new Error('The error catalogue needs schemaVersion 1, helpPath /help/ and entries');
  }
  const extra = Object.keys(errors).filter(key => !['schemaVersion', 'helpPath', 'causes', 'staticSteps', 'entries'].includes(key));
  if (extra.length) throw new Error(`The error catalogue has no field ${extra[0]}`);
  for (const list of ['causes', 'staticSteps']) {
    const listed = errors[list];
    if (!Array.isArray(listed) || listed.some(code => typeof code !== 'string') || new Set(listed).size !== listed.length) {
      throw new Error(`The error catalogue lists each code once in ${list}`);
    }
  }
  const causes = new Set(errors.causes);
  const steps = new Set(errors.staticSteps);
  const seen = new Set();
  for (const entry of errors.entries) {
    const name = `error catalogue entry ${JSON.stringify(entry?.code)}`;
    if (typeof entry?.code !== 'string' || !CODE.test(entry.code)) throw new Error(`${name}: a code is 3 to 64 lowercase letters, digits and underscores`);
    if (seen.has(entry.code)) throw new Error(`${name}: the code has two entries`);
    seen.add(entry.code);
    if (!AUDIENCES.includes(entry.for)) throw new Error(`${name}: "for" is owner, agent or operator`);
    if (!plain(entry.title, 70) || /[.!]$/.test(entry.title)) throw new Error(`${name}: the title is one plain line of at most 70 characters without a final period`);
    if (causes.has(entry.code)) {
      if (!plain(entry.cause, 260)) throw new Error(`${name}: a listed cause is plain text of at most 260 characters`);
    } else if (entry.cause !== undefined) {
      throw new Error(`${name}: the cause is the sender's unless causes lists the code, so the entry has no "cause"`);
    }
    if (steps.has(entry.code)) {
      // `next` is what the owner does on reading the entry in the app; an entry for the agent states the agent's own step.
      if (!plain(entry.next, 260)) throw new Error(`${name}: a static step is plain text of at most 260 characters`);
      if (entry.agent !== undefined && !plain(entry.agent, 260)) throw new Error(`${name}: the agent's next step is plain text of at most 260 characters`);
      if (entry.for === 'agent' && entry.agent === undefined) throw new Error(`${name}: a static step for the agent states the agent's step in "agent", and the owner's in "next"`);
    } else if (entry.next !== undefined || entry.agent !== undefined) {
      throw new Error(`${name}: the step is the sender's unless staticSteps lists the code, so the entry has no "next" or "agent"`);
    }
    if (namesReader(entry.title) || (entry.cause !== undefined && namesReader(entry.cause))) throw new Error(`${name}: the title and the cause name no reader ("you")`);
    const unknown = Object.keys(entry).filter(key => !FIELDS.includes(key));
    if (unknown.length) throw new Error(`${name}: unknown field ${unknown[0]}`);
  }
  for (const list of ['causes', 'staticSteps']) {
    const unknown = errors[list].filter(code => !seen.has(code));
    if (unknown.length) throw new Error(`${list} lists ${unknown[0]}, which has no entry`);
  }
  return errors;
}

/** One entry as the catalogue file writes it: its fields in order, on one line. */
const line = entry => '{' + FIELDS.filter(key => key in entry).map(key => `${JSON.stringify(key)}: ${JSON.stringify(entry[key])}`).join(', ') + '}';
/** The canonical text of a catalogue: the allow-lists and the entries, one code or entry per line, with the fields in order. */
export function catalogueText(errors) {
  const entries = Array.isArray(errors?.entries) ? errors.entries : [];
  const list = name => {
    const listed = Array.isArray(errors?.[name]) ? errors[name] : [];
    return [`  ${JSON.stringify(name)}: [`, ...listed.map((code, i) => '    ' + JSON.stringify(code) + (i < listed.length - 1 ? ',' : '')), '  ],'];
  };
  return ['{', '  "schemaVersion": ' + JSON.stringify(errors?.schemaVersion) + ',', '  "helpPath": ' + JSON.stringify(errors?.helpPath) + ',',
    ...list('causes'), ...list('staticSteps'),
    '  "entries": [', ...entries.map((entry, i) => '    ' + line(entry) + (i < entries.length - 1 ? ',' : '')), '  ]', '}'].join('\n') + '\n';
}

/**
 * Parses the source catalogue file and checks it. The file must be the canonical text of what it parses to, apart from
 * blank lines between groups of entries: JSON.parse keeps only the last of two equal keys and reads escapes, so a
 * second field or an escaped character could hide text that another reader, such as Rust's serde_json, reads otherwise.
 */
export function checkErrorCatalogueSource(bytes) {
  // The file is ASCII, so its bytes and its text are the same: no decoder can replace a malformed byte.
  const raw = Buffer.from(bytes);
  const outside = raw.findIndex(byte => byte !== 0x0a && (byte < 0x20 || byte > 0x7e));
  if (outside !== -1) throw new Error(`The error catalogue is printable ASCII and line feeds; byte ${outside} is 0x${raw[outside].toString(16)}`);
  const source = raw.toString('latin1');
  const errors = JSON.parse(source);
  const canonical = catalogueText(errors).split('\n').filter(text => text !== '');
  const lines = source.split('\n').filter(text => text !== '');
  const differs = lines.findIndex((text, i) => text !== canonical[i]);
  if (lines.length !== canonical.length || differs !== -1) {
    throw new Error(`The error catalogue is not in its canonical form near ${JSON.stringify((lines[differs] ?? '').slice(0, 60))}: one code and one entry per line, its fields once each and in order, and no escapes other than JSON.stringify writes`);
  }
  return checkErrorCatalogue(errors);
}
