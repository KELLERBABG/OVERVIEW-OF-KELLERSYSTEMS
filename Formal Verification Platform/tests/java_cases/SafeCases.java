package test;

public class SafeCases {
    static class Node {
        int val;
        Node next;
    }

    // ─── 1. java-npe (safe) ──────────────────────────────────

    public int safeNpeGuardIf(Node n) {
        if (n != null) {
            return n.val;
        }
        return 0;
    }

    public int safeNpeGuardReturn(Node n) {
        if (n == null) {
            return 0;
        }
        return n.val;
    }

    public int safeNpeNew() {
        Node n = new Node();
        return n.val;
    }

    public int safeNpeThis() {
        return this.hashCode();
    }

    public int safeNpeStatic() {
        return Math.max(1, 2);
    }

    // ─── 2. java-array-oob (safe) ────────────────────────────

    public int safeArrayOobForLoop(int[] arr) {
        if (arr == null) {
            return 0;
        }
        int sum = 0;
        for (int i = 0; i < arr.length; i++) {
            sum += arr[i];
        }
        return sum;
    }

    public int safeArrayOobGuarded(int[] arr, int i) {
        if (arr != null && i >= 0 && i < arr.length) {
            return arr[i];
        }
        return -1;
    }

    public int safeArrayOobConst() {
        int[] arr = new int[5];
        return arr[2];
    }

    // ─── 3. java-div-by-zero (safe) ──────────────────────────

    public int safeDivConst(int x) {
        return x / 2;
    }

    public int safeDivGuarded(int a, int b) {
        if (b != 0) {
            return a / b;
        }
        return 0;
    }

    public int safeDivGuardReturn(int a, int b) {
        if (b == 0) {
            return 0;
        }
        return a / b;
    }

    // ─── 4. java-class-cast (safe) ───────────────────────────

    public String safeClassCast(Object obj) {
        if (obj instanceof String) {
            return (String) obj;
        }
        return "";
    }

    public String safeClassCastGuardReturn(Object obj) {
        if (!(obj instanceof String)) {
            return "";
        }
        return (String) obj;
    }

    public Object safeUpcast(String s) {
        return (Object) s;
    }

    // ─── 5. java-negative-array-size (safe) ──────────────────

    public int[] safeNegativeArraySizeConst() {
        return new int[10];
    }

    public int[] safeNegativeArraySizeZero() {
        return new int[0];
    }

    public int[] safeNegativeArraySizeGuarded(int n) {
        if (n >= 0) {
            return new int[n];
        }
        return new int[0];
    }

    public int[] safeNegativeArraySizeGuardReturn(int n) {
        if (n < 0) {
            return new int[0];
        }
        return new int[n];
    }

    public int[] safeNegativeArraySizeFromLength(int[] arr) {
        if (arr == null) {
            return new int[0];
        }
        return new int[arr.length];
    }
}
