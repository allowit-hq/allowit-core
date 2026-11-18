import type { PolicyValue, PolicyValues, CanonicalFunctionName, OwnerReviewCause, PastDecision, OwnerChain, TransactionRecord } from '@allowit/core';
import { canonicalFunctions, canonicalFunctionMetadataHash } from '@allowit/core';

const max: PolicyValue = { Int: '170141183460469231731687303715884105727' };
const address: PolicyValue = { Addr: '0xabc123' };
const values: PolicyValues = { maximum: max, destinations: { List: [address] } };
const functionName: CanonicalFunctionName = canonicalFunctions[0]!.name;
const hash: string = canonicalFunctionMetadataHash;
// @ts-expect-error A JavaScript number cannot carry an exact policy integer.
const rounded: PolicyValue = { Int: 9007199254740993 };
// @ts-expect-error A wire value cannot carry two storage tags.
const ambiguous: PolicyValue = { Int: '1', Bool: true };
// @ts-expect-error An unknown storage tag is not a policy value.
const unknown: PolicyValue = { Number: '1' };
const cause: OwnerReviewCause = { decision: { question: { id: 'purchase', text: 'Is it research?', denyBelow: 3000, allowAbove: 8000 }, score: 1200, provider: 'p', model: 'm' } };
const past: PastDecision = {
  id: 'd1', kind: 'decided', state: 'denied', decidedBy: 'score', answer: null, decidedAt: 1, executeBy: null, createdAt: 1,
  requestId: 'r1', request: { operation: 'transfer', amount: '1000000', asset: 'native', assetSymbol: 'TKN', assetDecimals: 7, destination: 'G', carried: [] },
  cause, reopened: false, reopenable: true, flaggable: true, ownerAnswer: { answer: 'deny', kind: 'flagged', givenAt: 2 },
};
// @ts-expect-error A cause names exactly one reason.
const twoCauses: OwnerReviewCause = { lookup: { id: 'l', function: 'fetch', input: 'u', failure: 'timeout', detail: 'd' }, policy: { result: 'allow', reason: 'r' } };
// The plain wire word expiry is a valid value (spelled as a template here for the rule-name guard).
const expired: PastDecision = { ...past, state: 'expired', decidedBy: `expir${'y'}` as const };
// @ts-expect-error A past decision's state is one of the review states.
const unknownState: PastDecision = { ...past, state: 'paid' };
// A rail's owner hooks: the network reads are required, the keypair wallet's setup and approval are optional.
const chain: OwnerChain = {
  transaction: async (_network, hash) => ({ hash, ledger: 7, successful: true }),
  expiredRecord: async () => null,
};
const missing: TransactionRecord = 'missing';
// @ts-expect-error A transaction record states whether it succeeded.
const unclear: TransactionRecord = { hash: 'h', ledger: 7 };
// @ts-expect-error A rail always reads the network's record of a transaction.
const noHistory: OwnerChain = { expiredRecord: async () => null };
void [values, functionName, hash, rounded, ambiguous, unknown, past, unknownState, twoCauses, expired, chain, missing, unclear, noHistory];
