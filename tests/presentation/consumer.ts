import type { PolicyValue, PolicyValues, CanonicalFunctionName } from '@allowit/core';
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
void [values, functionName, hash, rounded, ambiguous, unknown];
