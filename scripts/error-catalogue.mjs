// The rules of the error catalogue. The generator refuses a catalogue that breaks one, before it writes the
// TypeScript or the Rust copy; `crates/allowit-core/src/errors.rs` tests the same rules on the Rust side.

export const AUDIENCES = ['owner', 'agent', 'operator'];
export const FIELDS = ['code', 'for', 'title', 'cause', 'next', 'agent'];
export const CODE = /^[a-z][a-z0-9_]{2,63}$/;
// Controls (C0, DEL, C1), line and paragraph separators, direction marks and overrides, and the byte order mark:
// none of them belongs in one line of text that a terminal or a page shows.
const HIDDEN = /[\u0000-\u001f\u007f-\u009f\u200e\u200f\u2028-\u202e\u2066-\u2069\ufeff]/u;
// A title and a cause name no reader: an owner and an agent read the same entry.
const READER = /\b(you|your|yours|yourself)\b/i;

/** Plain text: well-formed Unicode, trimmed, without hidden characters, of 1 to `limit` characters. */
export function plain(text, limit) {
  return typeof text === 'string' && text.isWellFormed() && text.length > 0 && [...text].length <= limit
    && text === text.trim() && !HIDDEN.test(text);
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
    if (READER.test(entry.title) || READER.test(entry.cause)) throw new Error(`${name}: the title and the cause name no reader ("you")`);
    const unknown = Object.keys(entry).filter(key => !FIELDS.includes(key));
    if (unknown.length) throw new Error(`${name}: unknown field ${unknown[0]}`);
  }
  return errors;
}
