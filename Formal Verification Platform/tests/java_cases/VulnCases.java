package test;

public class VulnCases {
    static class Node {
        int val;
        Node next;
    }

    // ─── 1. java-npe ─────────────────────────────────────────

    public int vulnNpeParam(Node n) {
        return n.val; // java-npe: unguarded dereference of parameter
    }

    public void vulnNpeExplicitNull() {
        Node n = null;
        n.val = 42; // java-npe: dereference of definitely null
    }

    public int vulnNpeMethodCall(Node n) {
        return n.hashCode(); // java-npe: unguarded method call
    }

    public int vulnNpeArrayLength(int[] arr) {
        return arr.length; // java-npe: unguarded array length
    }

    // ─── 2. java-array-oob ───────────────────────────────────

    public int vulnArrayOobNegative(int[] arr) {
        return arr[-1]; // java-array-oob: negative index
    }

    public int vulnArrayOobParam(int[] arr, int i) {
        return arr[i]; // java-array-oob: index without bounds check
    }

    public int vulnArrayOobConstExceed() {
        int[] arr = new int[5];
        return arr[10]; // java-array-oob: index exceeds array size
    }

    // ─── 3. java-div-by-zero ─────────────────────────────────

    public int vulnDivByZeroConst(int x) {
        return x / 0; // java-div-by-zero: division by constant zero
    }

    public int vulnModByZeroConst(int x) {
        return x % 0; // java-div-by-zero: modulo by constant zero
    }

    public int vulnDivByZeroVar(int a, int b) {
        return a / b; // java-div-by-zero: division by unverified divisor
    }

    // ─── 4. java-class-cast ──────────────────────────────────

    public String vulnClassCast(Object obj) {
        return (String) obj; // java-class-cast: downcast without instanceof guard
    }

    // ─── 5. java-negative-array-size ─────────────────────────

    public int[] vulnNegativeArraySizeConst() {
        return new int[-5]; // java-negative-array-size: negative constant size
    }

    public int[] vulnNegativeArraySizeVar(int n) {
        return new int[n]; // java-negative-array-size: unverified size
    }
}
