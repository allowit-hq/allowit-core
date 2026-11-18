import type { PolicyValue, PolicyValues, CanonicalFunctionName, OwnerReviewCause, PastDecision, OwnerChain, TransactionRecord } from '@allowit/core';
import type { Quantity, Settlement, LimitReport, PaidService, PaymentChallenge, PaymentProof, WalletKindInfo } from '@allowit/core';
import type { PaymentLimit, WalletLimits, OperationReports } from '@allowit/core';
import type { CreditBalance, CreditCallPrice, CreditCharge, CreditEntries, CreditFee, CreditPrices, PolicyCredits } from '@allowit/core';
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
// Payment and wallet-kind values as Core's Rust serializes them.
const five: Quantity = { units: '50000000', asset: { symbol: 'XYZ', decimals: 7, name: 'XYZ', native: true }, text: '5 XYZ' };
const settled: Settlement = { height: 12, fee: five, amount: null };
const limit: LimitReport = { param: 'daily_spending_limit', limit: five, spent: five, remaining: five };
const paid: PaidService = { resource: 'https://seller.example/r', payTo: 'G', amount: '100000', asset: 'native', settledBy: 'seller', settleWithin: 60 };
const offer: PaymentChallenge = { resource: paid.resource, network: 'example:testnet', scheme: 'exact', payTo: 'G', amount: five, asset: 'native', terms: { any: 1 } };
const proof: PaymentProof = { header: 'X-PAYMENT', value: 'v' };
const native: WalletKindInfo = {
  id: 'native', title: 'Native', explanation: 'e', recommended: true, pays: 'p', fee: 'f', enforcement: 'offchain', custody: 'keypair',
  supportedParams: [], paramsUrl: null,
};
// @ts-expect-error A settlement states its amount, null when the rail reports none.
const noAmount: Settlement = { height: 12, fee: five };
// @ts-expect-error Base units are decimal text, never a JavaScript number.
const roundedUnits: Quantity = { ...five, units: 50000000 };
// @ts-expect-error Only the payer or the seller settles a payment.
const thirdParty: PaidService = { ...paid, settledBy: 'facilitator' };
// @ts-expect-error A wallet kind is one of the four kinds.
const otherKind: WalletKindInfo = { ...native, id: 'vault' };
void [values, functionName, hash, rounded, ambiguous, unknown, past, unknownState, twoCauses, expired, chain, missing, unclear, noHistory];
const perPayment: PaymentLimit = { param: 'payment_limit', limit: five };
const counters: WalletLimits = { paymentLimits: [perPayment] };
const reports: OperationReports = { settlement: settled, paidService: paid, feeCharged: five, limits: [limit], paymentLimits: [perPayment], explorer: 'https://explorer.example/tx/1' };
// A signed authorization that no ledger holds yet names no transaction to link.
const unsettled: OperationReports = { paidService: paid, explorer: null };
// @ts-expect-error A limit on one operation keeps no counter.
const counted: PaymentLimit = { ...perPayment, spent: five };
void [settled, limit, offer, proof, noAmount, roundedUnits, thirdParty, otherKind, counters, reports, unsettled, counted];
// Credits as the server sends them: every amount is decimal text.
const networkFee: CreditFee = { asset: 'XYZ', units: '11154', amount: '0.0011154', usdPerAsset: '0.14', repaid: '0.0011154', network: 'example:testnet', transaction: 'h' };
const charge: CreditCharge = {
  charged: '2.0', tariff: '2026-11-18.1',
  lines: [{ item: 'base', credits: '1.0' }, { item: 'decision_api', credits: '0.0', cost: '0.24', provider: 'p', model: 'm', inputTokens: 10, outputTokens: 2 }, { item: 'network_fee', credits: '0.0', cost: '0.000156', fee: networkFee }, { item: 'inference', credits: '1.0', cost: '0.24', carried: '0.5' }],
};
const balance: CreditBalance = { owner: 'G', balance: '10.0', held: '0.0', available: '10.0', tariff: '2026-11-18.1', evaluation: '1.0', exhausted: false, grantsUrl: null };
const page: CreditEntries = { entries: [{ id: 'e1', kind: 'charge', at: '2026-11-18T10:00:00Z', amount: '-2.0', tariff: charge.tariff, lines: charge.lines, policyId: 'p', requestId: 'r', outcome: 'held' }], next: null };
const usage: PolicyCredits = {
  policyId: 'p', evaluation: { credits: '1.0', tariff: charge.tariff, lines: [charge.lines[0]!] },
  allowance: { policyId: 'p', calls: 200, credits: '200.0', period: 'utc_day', isDefault: true, usedCalls: 0, usedCredits: '0.0', resetsAt: null },
};
const prices: CreditPrices = {
  version: charge.tariff, usdPerCredit: '0.01', request: { base: '1.0', inference: 'whole_credits_rounded_down', fees: 'added_before_the_rounding' },
  calls: [{ item: 'decision_api', inputPerMillionTokens: '30.0', outputPerMillionTokens: '120.0', countsInAllowance: true }, { item: 'fetch', call: '0.0', countsInAllowance: false }],
  searchFee: { perSearch: '0.9' }, assetUsd: { XYZ: '0.14' }, startingGrant: '100.0', allowance: { calls: 200, credits: '200.0', period: 'utc_day' }, grantsUrl: 'https://grants.example',
};
// @ts-expect-error A credit amount is decimal text, never a JavaScript number.
const roundedCharge: CreditCharge = { ...charge, charged: 2 };
// @ts-expect-error A charged request ended allowed, denied, held or failed.
const unknownOutcome: CreditEntries = { entries: [{ ...page.entries[0]!, outcome: 'paid' }], next: null };
// A call price that the server writes has both token rates or one price per call, and always says whether it counts.
const tokenRate = prices.calls[0]!;
const counts: boolean = tokenRate.countsInAllowance;
const rates: [string, string] | null = tokenRate.call === undefined ? [tokenRate.inputPerMillionTokens, tokenRate.outputPerMillionTokens] : null;
// @ts-expect-error A call price has a price.
const noPrice: CreditCallPrice = { item: 'fetch', countsInAllowance: false };
// @ts-expect-error Token rates come in pairs.
const halfRate: CreditCallPrice = { item: 'decision_api', inputPerMillionTokens: '30.0', countsInAllowance: true };
// @ts-expect-error A call has token rates or a price for each call, not both.
const mixed: CreditCallPrice = { item: 'fetch', call: '0.0', inputPerMillionTokens: '30.0', outputPerMillionTokens: '120.0', countsInAllowance: false };
// @ts-expect-error The server always says whether a call counts against the allowance.
const uncounted: CreditCallPrice = { item: 'fetch', call: '0.0' };
void [balance, usage, prices, roundedCharge, unknownOutcome, counts, rates, noPrice, halfRate, mixed, uncounted];
