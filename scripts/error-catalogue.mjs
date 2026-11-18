// The rules of the error catalogue. The generator refuses a catalogue that breaks one, before it writes the
// TypeScript or the Rust copy; `crates/allowit-core/src/errors.rs` tests the same rules on the Rust side.

export const AUDIENCES = ['owner', 'agent', 'operator'];
export const FIELDS = ['code', 'for', 'title', 'cause', 'next', 'agent'];
export const CODE = /^[a-z][a-z0-9_]{2,63}$/;
// Catalogue text is printable ASCII: letters, digits, the space and ASCII punctuation. Every reader, a page, a terminal
// or a Rust host, shows such text as it is written, and no hidden character, direction mark or combining mark can
// change what it shows.
const PRINTABLE = /^[\x20-\x7e]*$/;
// A title and a cause name no reader: an owner and an agent read the same entry. A word is a run of ASCII letters and
// digits, as in the Rust test.
const READER_WORDS = new Set(['you', 'your', 'yours', 'yourself']);
export const namesReader = text => text.split(/[^A-Za-z0-9]+/).some(word => READER_WORDS.has(word.toLowerCase()));

/** Plain text: printable ASCII, trimmed, of 1 to `limit` characters. */
export function plain(text, limit) {
  return typeof text === 'string' && text.length > 0 && text.length <= limit && text === text.trim() && PRINTABLE.test(text);
}

/** Throws for the first rule that the catalogue document breaks. */
export function checkErrorCatalogue(errors) {
  if (errors?.schemaVersion !== 1 || errors.helpPath !== '/help/' || !Array.isArray(errors.entries) || !errors.entries.length) {
    throw new Error('The error catalogue needs schemaVersion 1, helpPath /help/ and entries');
  }
  const seen = new Set();
  for (const entry of errors.entries) {
    const name = `error catalogue entry ${JSON.stringify(entry?.code)}`;
    if (typeof entry?.code !== 'string' || !CODE.test(entry.code)) throw new Error(`${name}: a code is 3 to 64 lowercase letters, digits and underscores`);
    if (seen.has(entry.code)) throw new Error(`${name}: the code has two entries`);
    seen.add(entry.code);
    if (!AUDIENCES.includes(entry.for)) throw new Error(`${name}: "for" is owner, agent or operator`);
    if (!plain(entry.title, 70) || /[.!]$/.test(entry.title)) throw new Error(`${name}: the title is one plain line of at most 70 characters without a final period`);
    if (!plain(entry.cause, 260) || !plain(entry.next, 260)) throw new Error(`${name}: the cause and the next step are plain text of at most 260 characters`);
    if (entry.agent !== undefined && !plain(entry.agent, 260)) throw new Error(`${name}: the agent's next step is plain text of at most 260 characters`);
    if (namesReader(entry.title) || namesReader(entry.cause)) throw new Error(`${name}: the title and the cause name no reader ("you")`);
    const unknown = Object.keys(entry).filter(key => !FIELDS.includes(key));
    if (unknown.length) throw new Error(`${name}: unknown field ${unknown[0]}`);
  }
  return errors;
}

/** One entry as the catalogue file writes it: its fields in order, on one line. */
const line = entry => '{' + FIELDS.filter(key => key in entry).map(key => `${JSON.stringify(key)}: ${JSON.stringify(entry[key])}`).join(', ') + '}';

/**
 * Parses the catalogue file and checks it. The file must be the canonical text of what it parses to, apart from blank
 * lines between groups of entries: JSON.parse keeps only the last of two equal keys and reads escapes, so a second
 * field or an escaped character could hide text that another reader, such as Rust's serde_json, reads otherwise.
 */
export function checkErrorCatalogueSource(bytes) {
  // The file is ASCII, so its bytes and its text are the same: no decoder can replace a malformed byte.
  const raw = Buffer.from(bytes);
  const outside = raw.findIndex(byte => byte !== 0x0a && (byte < 0x20 || byte > 0x7e));
  if (outside !== -1) throw new Error(`The error catalogue is printable ASCII and line feeds; byte ${outside} is 0x${raw[outside].toString(16)}`);
  const source = raw.toString('latin1');
  const errors = JSON.parse(source);
  const entries = Array.isArray(errors?.entries) ? errors.entries : [];
  const canonical = ['{', '  "schemaVersion": ' + JSON.stringify(errors?.schemaVersion) + ',', '  "helpPath": ' + JSON.stringify(errors?.helpPath) + ',', '  "entries": [',
    ...entries.map((entry, i) => '    ' + line(entry) + (i < entries.length - 1 ? ',' : '')), '  ]', '}'];
  const lines = source.split('\n').filter(text => text !== '');
  const differs = lines.findIndex((text, i) => text !== canonical[i]);
  if (lines.length !== canonical.length || differs !== -1) {
    throw new Error(`The error catalogue is not in its canonical form near ${JSON.stringify((lines[differs] ?? '').slice(0, 60))}: one entry per line, its fields once each and in order, and no escapes other than JSON.stringify writes`);
  }
  return checkErrorCatalogue(errors);
}
