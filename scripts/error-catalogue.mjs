// The rules of the error catalogue. The generator refuses a catalogue that breaks one, before it writes the
// TypeScript or the Rust copy; `crates/allowit-core/src/errors.rs` tests the same rules on the Rust side.
//
// `typescript/error-catalogue.json` is the source: each entry's own words, and the retry kind of an entry whose reader
// can send its request again. The generator composes each step from those words and the template of the retry kind,
// and writes the composed catalogue that every host reads.

export const AUDIENCES = ['owner', 'agent', 'operator'];
export const FIELDS = ['code', 'for', 'title', 'cause', 'next', 'agent', 'retry'];
// What a reader does with the request that the error answered. Each kind has one reviewed template, and only a
// template names a request ID, a payment or a retry:
//   same     the request has no final outcome, or the server answers it from its kept outcome: send it unchanged.
//   owner    the outcome stands until the owner changes the policy; then send it unchanged.
//   final    the request can have taken effect: read it first, and send a new one only once it ended without a payment.
//   settled  the purchase has a payment that settled or is settling: do not pay for it again.
//   none     the request is not sent again.
export const RETRY_KINDS = ['same', 'owner', 'final', 'settled', 'none'];
// A template: the agent's sentence before the entry's own step, the agent's sentence after it, and the owner's sentence.
export const TEMPLATE_FIELDS = ['first', 'then', 'owner'];
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

// The word rules, complete for their word lists: an entry's own words are checked word by word.
// A step (`next`, `agent`) names no ID, no payment and no retry: the template of its retry kind says those.
const ID_WORDS = new Set(['id', 'ids']);
const PAYMENT_WORDS = new Set(['pay', 'pays', 'paying', 'paid', 'unpaid', 'repay', 'repays', 'repaying', 'repaid', 'payment',
  'payments', 'settle', 'settles', 'settled', 'settling', 'settlement', 'settlements', 'unsettled', 'charged', 'refund',
  'refunds', 'refunding', 'refunded']);
const RETRY_WORDS = new Set(['retry', 'retries', 'retried', 'retrying', 'resend', 'resends', 'resending', 'resent']);
// The agent's own step does not repeat anything either: "again" is a template's word there.
const AGENT_WORDS = new Set(['again']);
// A title and a cause state no payment outcome, unless the code means one: an entry of the retry kind `settled`. They may
// say that the agent pays, or name a payment, but not that one was made, settled, charged or refunded.
const OUTCOME_WORDS = new Set(['paid', 'unpaid', 'settle', 'settles', 'settled', 'settling', 'settlement', 'settlements', 'unsettled',
  'charged', 'refund', 'refunds', 'refunding', 'refunded']);
export const RULES = {
  request: 'it names an ID: the request ID of a retry is in the template of the retry kind',
  payment: 'it names a payment or its outcome: payment steps are in the template of the retry kind',
  retry: 'it repeats a request: a retry is in the template of the retry kind',
};
/** The word rule that an entry's own text in `field` breaks, `request`, `payment` or `retry`, or null. */
export function wordRule(field, text, kind) {
  const list = words(text);
  if (field === 'title' || field === 'cause') return kind !== 'settled' && list.some(word => OUTCOME_WORDS.has(word)) ? 'payment' : null;
  if (list.some(word => ID_WORDS.has(word))) return 'request';
  if (list.some(word => PAYMENT_WORDS.has(word))) return 'payment';
  if (list.some(word => RETRY_WORDS.has(word) || (field === 'agent' && AGENT_WORDS.has(word)))) return 'retry';
  return null;
}

/** Plain text: printable ASCII, trimmed, of 1 to `limit` characters. */
export function plain(text, limit) {
  return typeof text === 'string' && text.length > 0 && text.length <= limit && text === text.trim() && PRINTABLE.test(text);
}

/** Throws for the first rule that the source catalogue breaks. */
export function checkErrorCatalogue(errors) {
  if (errors?.schemaVersion !== 1 || errors.helpPath !== '/help/' || !Array.isArray(errors.entries) || !errors.entries.length) {
    throw new Error('The error catalogue needs schemaVersion 1, helpPath /help/ and entries');
  }
  const kinds = Object.keys(errors.retry ?? {});
  if (kinds.join() !== RETRY_KINDS.join()) throw new Error(`The error catalogue has the retry templates ${RETRY_KINDS.join(', ')}, in this order`);
  for (const kind of RETRY_KINDS) {
    const template = errors.retry[kind];
    if (Object.keys(template ?? {}).join() !== TEMPLATE_FIELDS.join()) throw new Error(`The retry template ${kind} has the fields ${TEMPLATE_FIELDS.join(', ')}`);
    // A template sentence may be longer than an entry's own step: it carries the whole retry rule of its kind.
    if (!(template.first === '' || plain(template.first, 400)) || !plain(template.then, 400) || !plain(template.owner, 260)) {
      throw new Error(`The retry template ${kind} is plain text: an optional first sentence, then the agent's and the owner's`);
    }
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
    if (entry.retry !== undefined && !RETRY_KINDS.includes(entry.retry)) throw new Error(`${name}: the retry kind is one of ${RETRY_KINDS.join(', ')}`);
    // `next` is what the owner does on reading the entry in the app; an entry for the agent states the agent's own step.
    if (entry.for === 'agent' && entry.agent === undefined && entry.retry === undefined) {
      throw new Error(`${name}: an entry for the agent states the agent's step in "agent" or by its retry kind, and the owner's in "next"`);
    }
    for (const field of ['title', 'cause', 'next', 'agent']) {
      const rule = entry[field] === undefined ? null : wordRule(field, entry[field], entry.retry);
      if (rule) throw new Error(`${name}: the ${field} breaks a rule: ${RULES[rule]}`);
    }
    if (namesReader(entry.title) || namesReader(entry.cause)) throw new Error(`${name}: the title and the cause name no reader ("you")`);
    const unknown = Object.keys(entry).filter(key => !FIELDS.includes(key));
    if (unknown.length) throw new Error(`${name}: unknown field ${unknown[0]}`);
  }
  return errors;
}

/** The longest composed step: the entry's own words and the sentences of its template. */
export const COMPOSED = 600;
/** The sentences of a step, joined by one space. */
const join = (...sentences) => sentences.filter(Boolean).join(' ');
/**
 * The catalogue that hosts read: each step of an entry with a retry kind composed from the entry's own words and the
 * kind's template. The agent's step reads the template's first sentence, the entry's step and the template's last one.
 * The owner's step of an entry for the agent ends with the template's sentence for the owner, which says what the
 * agent does; the owner's step of another entry is the owner's own.
 */
export function composeErrorCatalogue(errors) {
  const entries = errors.entries.map(entry => {
    if (entry.retry === undefined) return entry;
    const template = errors.retry[entry.retry];
    const next = entry.for === 'agent' ? join(entry.next, template.owner) : entry.next;
    const agent = join(template.first, entry.agent, template.then);
    // A composed step is plain text of at most COMPOSED characters, as errors.rs reads it.
    if (!plain(next, COMPOSED) || !plain(agent, COMPOSED)) throw new Error(`error catalogue entry ${JSON.stringify(entry.code)}: a composed step is plain text of at most ${COMPOSED} characters`);
    return { ...entry, next, agent };
  });
  return { schemaVersion: errors.schemaVersion, helpPath: errors.helpPath, retry: errors.retry, entries };
}

/** One entry as the catalogue file writes it: its fields in order, on one line. */
const line = (entry, fields = FIELDS) => '{' + fields.filter(key => key in entry).map(key => `${JSON.stringify(key)}: ${JSON.stringify(entry[key])}`).join(', ') + '}';
/** The canonical text of a catalogue: its templates and its entries one per line, with the fields in order. */
export function catalogueText(errors) {
  const kinds = Object.keys(errors?.retry ?? {});
  const entries = Array.isArray(errors?.entries) ? errors.entries : [];
  return ['{', '  "schemaVersion": ' + JSON.stringify(errors?.schemaVersion) + ',', '  "helpPath": ' + JSON.stringify(errors?.helpPath) + ',',
    '  "retry": {', ...kinds.map((kind, i) => `    ${JSON.stringify(kind)}: ${line(errors.retry[kind] ?? {}, TEMPLATE_FIELDS)}` + (i < kinds.length - 1 ? ',' : '')), '  },',
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
    throw new Error(`The error catalogue is not in its canonical form near ${JSON.stringify((lines[differs] ?? '').slice(0, 60))}: one template or entry per line, its fields once each and in order, and no escapes other than JSON.stringify writes`);
  }
  return checkErrorCatalogue(errors);
}
