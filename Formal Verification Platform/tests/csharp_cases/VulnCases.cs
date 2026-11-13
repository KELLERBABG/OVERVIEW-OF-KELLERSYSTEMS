namespace Test;

public class VulnCases
{
    public class Node
    {
        public int Val;
        public Node? Next;
    }

    // ─── 1. cs-null-reference ─────────────────────────────────────────

    public int VulnNullParam(Node n)
    {
        return n.Val; // cs-null-reference: unguarded dereference of parameter
    }

    public void VulnNullExplicit()
    {
        Node n = null;
        n.Val = 42; // cs-null-reference: definitely null dereference
    }

    public int VulnNullableType(Node? n)
    {
        return n.Val; // cs-null-reference: nullable reference dereferenced without check
    }

    // ─── 2. cs-index-out-of-range ─────────────────────────────────────

    public int VulnIndexNegative(int[] arr)
    {
        return arr[-1]; // cs-index-out-of-range: negative index
    }

    public int VulnIndexParam(int[] arr, int i)
    {
        return arr[i]; // cs-index-out-of-range: unverified index
    }

    public int VulnIndexConstExceed()
    {
        int[] arr = new int[5];
        return arr[10]; // cs-index-out-of-range: index exceeds known array size
    }

    // ─── 3. cs-span-oob ───────────────────────────────────────────────

    public int VulnSpanIndex(Span<int> span, int i)
    {
        return span[i]; // cs-span-oob: span index without check
    }

    public ReadOnlySpan<byte> VulnSpanSlice(ReadOnlySpan<byte> spanBytes, int start)
    {
        return spanBytes.Slice(start); // cs-span-oob: span slice without bounds check
    }

    // ─── 4. cs-div-by-zero ────────────────────────────────────────────

    public int VulnDivByZeroConst(int x)
    {
        return x / 0; // cs-div-by-zero: division by constant zero
    }

    public int VulnModByZeroConst(int x)
    {
        return x % 0; // cs-div-by-zero: modulo by constant zero
    }

    public int VulnDivByZeroVar(int a, int b)
    {
        return a / b; // cs-div-by-zero: division by potentially zero variable
    }

    // ─── 5. cs-invalid-cast ───────────────────────────────────────────

    public string VulnInvalidCast(object obj)
    {
        return (string)obj; // cs-invalid-cast: cast without is/as guard
    }
}
