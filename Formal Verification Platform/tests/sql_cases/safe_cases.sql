-- SQL Safe Verification Test Cases

-- ─── 1. sql-division-by-zero ─────────────────────────────────────────────
SELECT a / NULLIF(b, 0) FROM items;
SELECT CASE WHEN b <> 0 THEN a / b ELSE NULL END FROM items;
SELECT a / b FROM items WHERE b <> 0;
SELECT a / 100 FROM items;

-- ─── 2. sql-not-null-violation ───────────────────────────────────────────
CREATE TABLE safe_users (
    user_id INT NOT NULL,
    username VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'active'
);

INSERT INTO safe_users (user_id, username, status) VALUES (1, 'Alice', 'active');
INSERT INTO safe_users (user_id, username) VALUES (2, 'Bob');

-- ─── 3. sql-recursive-cte-unbounded ──────────────────────────────────────
WITH RECURSIVE safe_cte_where(n) AS (
    SELECT 1
    UNION ALL
    SELECT n + 1 FROM safe_cte_where WHERE n < 100
)
SELECT * FROM safe_cte_where;

WITH RECURSIVE safe_cte_limit(n) AS (
    SELECT 1
    UNION ALL
    SELECT n + 1 FROM safe_cte_limit
)
SELECT * FROM safe_cte_limit LIMIT 50;

-- ─── 4. sql-null-comparison ──────────────────────────────────────────────
SELECT * FROM safe_users WHERE status IS NULL;
SELECT * FROM safe_users WHERE status IS NOT NULL;

-- ─── 5. sql-unguarded-update-delete ──────────────────────────────────────
UPDATE safe_users SET status = 'inactive' WHERE user_id = 1;
DELETE FROM safe_users WHERE user_id = 2;
