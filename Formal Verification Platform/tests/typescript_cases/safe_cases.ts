// TypeScript / JavaScript Safe Verification Test Cases

// ─── 1. ts-null-undefined-deref: Safe Patterns ───────────────────────────

export function safeGuardNull(node: any): any {
    if (node !== null && node !== undefined) {
        return node.value;
    }
    return null;
}

export function safeOptionalChaining(node: any): any {
    return node?.value;
}

export function safeNullishCoalescing(node: any): any {
    const fallback = { value: "default" };
    return (node ?? fallback).value;
}

export function safeTruthinessGuard(node: any): number {
    if (!node) return 0;
    return node.value;
}

// ─── 2. ts-type-error-call: Safe Patterns ────────────────────────────────

export function safeDeclaredFunctionCall(): number {
    function localHelper(): number {
        return 42;
    }
    return localHelper();
}

export function safeGuardedCallback(cb: any): void {
    if (typeof cb === 'function') {
        cb();
    }
    cb?.();
}

// ─── 3. ts-array-oob: Safe Patterns ──────────────────────────────────────

export function safeLoopArrayAccess(arr: number[]): number {
    let sum = 0;
    for (let i = 0; i < arr.length; i++) {
        sum += arr[i];
    }
    return sum;
}

export function safeGuardedIndex(arr: number[], i: number): number {
    if (i < 0 || i >= arr.length) {
        return 0;
    }
    return arr[i];
}

// ─── 4. ts-nan-propagation: Safe Patterns ────────────────────────────────

export function safeNumericArithmetic(a: number, b: number): number {
    const x = 100;
    const y = 200;
    const base = x + y;
    if (!Number.isNaN(a)) {
        return a + base;
    }
    return base;
}

// ─── 5. ts-prototype-pollution: Safe Patterns ────────────────────────────

export function safeFixedProperties(obj: any): void {
    if (!obj) return;
    obj.status = "active";
    obj["count"] = 10;
}

export function safeGuardedDynamicKey(obj: any, key: string, val: any): void {
    if (!obj) return;
    if (key !== '__proto__' && key !== 'constructor' && key !== 'prototype') {
        obj[key] = val;
    }
}

// ─── 6. ts-div-by-zero: Safe Patterns ────────────────────────────────────

export function safeDivByNonZeroLiteral(a: number): number {
    return a / 10;
}

export function safeDivGuarded(a: number, b: number): number {
    if (b === 0) return 0;
    return a / b;
}

export function safeDivPositive(a: number, b: number): number {
    if (b > 0) {
        return a / b;
    }
    return 0;
}
