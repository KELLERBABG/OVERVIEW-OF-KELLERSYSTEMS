namespace Test;

public class SafeCases
{
    public class Node
    {
        public int Val;
        public Node? Next;
    }

    // ─── 1. cs-null-reference (safe) ──────────────────────────────────

    public int SafeNullGuardIf(Node n)
    {
        if (n != null)
        {
            return n.Val;
        }
        return 0;
    }

    public int SafeNullGuardReturn(Node n)
    {
        if (n == null)
        {
            return 0;
        }
        return n.Val;
    }

    public int SafeNullCoalescing(Node? n)
    {
        return n?.Val ?? 0;
    }

    public int SafeNullNew()
    {
        var n = new Node();
        return n.Val;
    }

    public int SafeNullThis()
    {
        return this.GetHashCode();
    }

    public int SafeNullStatic()
    {
        return Math.Max(1, 2);
    }

    // ─── 2. cs-index-out-of-range (safe) ──────────────────────────────

    public int SafeIndexForLoop(int[] arr)
    {
        if (arr == null)
        {
            return 0;
        }
        int sum = 0;
        for (int i = 0; i < arr.Length; i++)
        {
            sum += arr[i];
        }
        return sum;
    }

    public int SafeIndexGuarded(int[] arr, int i)
    {
        if (arr != null && i >= 0 && i < arr.Length)
        {
            return arr[i];
        }
        return -1;
    }

    public int SafeIndexConst()
    {
        var arr = new int[5];
        return arr[2];
    }

    // ─── 3. cs-span-oob (safe) ────────────────────────────────────────

    public int SafeSpanLoop(Span<int> span)
    {
        int sum = 0;
        for (int i = 0; i < span.Length; i++)
        {
            sum += span[i];
        }
        return sum;
    }

    public int SafeSpanGuarded(Span<int> span, int i)
    {
        if (i < span.Length)
        {
            return span[i];
        }
        return 0;
    }

    // ─── 4. cs-div-by-zero (safe) ────────────────────────────────────

    public int SafeDivConst(int x)
    {
        return x / 2;
    }

    public int SafeDivGuarded(int a, int b)
    {
        if (b != 0)
        {
            return a / b;
        }
        return 0;
    }

    public int SafeDivGuardReturn(int a, int b)
    {
        if (b == 0)
        {
            return 0;
        }
        return a / b;
    }

    // ─── 5. cs-invalid-cast (safe) ────────────────────────────────────

    public string SafeCastIs(object obj)
    {
        if (obj is string)
        {
            return (string)obj;
        }
        return "";
    }

    public int SafeNumericCast(double d)
    {
        return (int)d;
    }
}
