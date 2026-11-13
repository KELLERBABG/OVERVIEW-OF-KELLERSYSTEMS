// TypeScript / JavaScript Vulnerable Verification Test Cases

// ─── 1. ts-null-undefined-deref ──────────────────────────────────────────

export function vulnNullDeref(node: any): any {
    return node.value; // ts-null-undefined-deref: unverified parameter dereference
}

export function vulnExplicitNull(): any {
    const x: any = null;
    return x.method(); // ts-null-undefined-deref: proved defect on null
}

export function vulnExplicitUndefined(): any {
    const y: any = undefined;
    return y.property; // ts-null-undefined-deref: proved defect on undefined
}

// ─── 2. ts-type-error-call ───────────────────────────────────────────────

export function vulnNonFunctionCall(): void {
    const x: any = 42;
    x(); // ts-type-error-call: calling number
}

export function vulnUnverifiedCallback(cb: any): void {
    cb(); // ts-type-error-call: unverified function call without typeof guard
}

// ─── 3. ts-array-oob ─────────────────────────────────────────────────────

export function vulnNegativeIndex(arr: number[]): number {
    return arr[-1]; // ts-array-oob: negative index
}

export function vulnUnboundedIndex(arr: number[], i: number): number {
    return arr[i]; // ts-array-oob: arbitrary index without bounds check
}

// ─── 4. ts-nan-propagation ───────────────────────────────────────────────

export function vulnNanArithmetic(): number {
    return NaN + 5; // ts-nan-propagation: arithmetic with literal NaN
}

export function vulnUndefinedArithmetic(): number {
    return undefined * 3; // ts-nan-propagation: arithmetic with literal undefined
}

export function vulnUnverifiedParamArithmetic(x: any): number {
    return x - 10; // ts-nan-propagation: arithmetic on unverified parameter
}

// ─── 5. ts-prototype-pollution ───────────────────────────────────────────

export function vulnProtoAssignDirect(obj: any, val: any): void {
    obj.__proto__ = val; // ts-prototype-pollution: direct assignment to __proto__
}

export function vulnProtoAssignIndex(obj: any, val: any): void {
    obj["__proto__"] = val; // ts-prototype-pollution: direct assignment to ["__proto__"]
}

export function vulnConstructorProtoAssign(obj: any, val: any): void {
    obj.constructor.prototype = val; // ts-prototype-pollution: assignment to constructor.prototype
}

export function vulnDynamicKeyAssign(obj: any, key: any, val: any): void {
    obj[key] = val; // ts-prototype-pollution: unverified dynamic property key
}

// ─── 6. ts-div-by-zero ───────────────────────────────────────────────────

export function vulnDivZeroLiteral(a: number): number {
    return a / 0; // ts-div-by-zero: division by literal 0
}

export function vulnModZeroLiteral(a: number): number {
    return a % 0; // ts-div-by-zero: modulo by literal 0
}

export function vulnDivVariable(a: number, b: number): number {
    return a / b; // ts-div-by-zero: division by unverified divisor
}
