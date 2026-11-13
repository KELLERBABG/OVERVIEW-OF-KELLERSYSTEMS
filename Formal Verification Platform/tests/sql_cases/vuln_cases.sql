-- SQL Vulnerable Verification Test Cases

-- ─── 1. sql-division-by-zero ─────────────────────────────────────────────
SELECT a / 0 FROM items;
SELECT a / b FROM items;

-- ─── 2. sql-not-null-violation ───────────────────────────────────────────
CREATE TABLE vuln_users (
    user_id INT NOT NULL,
    username VARCHAR(50) NOT NULL,
    bio TEXT
);

-- Explicit NULL into NOT NULL column
INSERT INTO vuln_users (user_id, username, bio) VALUES (1, NULL, 'Developer');

-- Omitted NOT NULL column without DEFAULT
INSERT INTO vuln_users (user_id, bio) VALUES (2, 'Developer');

-- ─── 3. sql-recursive-cte-unbounded ──────────────────────────────────────
WITH RECURSIVE unbounded_cte(n) AS (
    SELECT 1
    UNION ALL
    SELECT n + 1 FROM unbounded_cte
)
SELECT * FROM unbounded_cte;

-- ─── 4. sql-null-comparison ──────────────────────────────────────────────
SELECT * FROM vuln_users WHERE username = NULL;
SELECT * FROM vuln_users WHERE username <> NULL;

-- ─── 5. sql-unguarded-update-delete ──────────────────────────────────────
UPDATE vuln_users SET bio = 'Default bio';
DELETE FROM vuln_users;
