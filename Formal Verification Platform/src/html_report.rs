use crate::report::{MergeOutcome, VerifyOptions};
use std::path::Path;

pub fn render_html_report(outcome: &MergeOutcome, options: &VerifyOptions) -> String {
    let totals = &outcome.coverage.totals;
    let total_checks = if totals.reachable_assertions > 0 {
        totals.reachable_assertions
    } else {
        totals.supported_assertions + totals.unsupported_assertions
    };
    let passed = totals.passed;
    let failed = totals.failed;
    let warnings = totals.warnings;
    let panics = totals.panic_sites;
    // K8's second meaning, made just as prominent as the findings: an assertion
    // kind this build cannot model is not a proof, and a report that lists three
    // findings without saying it skipped a fourth check reads as complete.
    let unsupported = totals.unsupported_assertions;
    let panic_sites_decided = totals.panic_sites_decided;
    let unsupported_kinds = if totals.unsupported_kinds.is_empty() {
        String::new()
    } else {
        format!(" [{}]", totals.unsupported_kinds.join(", "))
    };
    let coverage_gap = if unsupported > 0 {
        format!(
            r#"<div class="coverage-gap"><strong>{} assertion(s) were not attempted.</strong> This build does not model the kind(s){}; the proof-soundness ratio counts them as unchecked, not as proved.</div>"#,
            unsupported,
            html_escape(&unsupported_kinds)
        )
    } else {
        String::new()
    };

    let overall_status = match &outcome.breach {
        None => {
            if warnings > 0 && options.deny_warn {
                (
                    "FAIL",
                    "bg-red",
                    "Gate breached: undecided obligations under strict/deny-warn policy.",
                )
            } else if failed > 0 {
                (
                    "FAIL",
                    "bg-red",
                    "Formal verification counterexample discovered.",
                )
            } else {
                (
                    "PASS",
                    "bg-green",
                    "All verified obligations mathematically proved sound.",
                )
            }
        }
        Some(_) => ("FAIL", "bg-red", "Gate contract breached."),
    };

    let pass_pct = if total_checks > 0 {
        (passed as f64 / total_checks as f64) * 100.0
    } else {
        100.0
    };

    let mut findings_rows = String::new();
    for event in &outcome.findings {
        let badge_class = match event.status.as_str() {
            "FAIL" => "badge-fail",
            "WARN" => "badge-warn",
            _ => "badge-pass",
        };

        let mut witness_html = String::new();
        if !event.witness.is_empty() {
            witness_html.push_str(
                "<div class='witness-box'><strong>SMT Counterexample Witness:</strong><ul>",
            );
            for w in &event.witness {
                witness_html.push_str(&format!(
                    "<li><code>{}</code> = <strong>{}</strong> <em>(symbol: {})</em></li>",
                    html_escape(&w.input),
                    html_escape(&w.value),
                    html_escape(&w.symbol)
                ));
            }
            witness_html.push_str("</ul></div>");
        }

        let loc = if let Some(line) = event.line {
            format!("{}:{}", html_escape(&event.file), line)
        } else {
            html_escape(&event.file)
        };

        findings_rows.push_str(&format!(
            r#"<tr class="finding-row status-{}" data-search="{} {} {}" onclick="toggleDetails(this)">
                <td><span class="badge {}">{}</span></td>
                <td><code>{}</code></td>
                <td><span class="kind-tag">{}</span></td>
                <td><span class="file-loc">{}</span></td>
                <td>
                    <div class="details-summary">{}</div>
                    {}
                </td>
            </tr>"#,
            event.status.to_lowercase(),
            html_escape(&event.function).to_lowercase(),
            html_escape(&event.kind).to_lowercase(),
            html_escape(&event.file).to_lowercase(),
            badge_class,
            event.status,
            html_escape(&event.function),
            html_escape(&event.kind),
            loc,
            html_escape(&event.details),
            witness_html
        ));
    }

    let solver_time_str = if outcome.coverage.totals.solver_elapsed_ms > 0 {
        format!(
            "{:.2} s",
            outcome.coverage.totals.solver_elapsed_ms as f64 / 1000.0
        )
    } else {
        "N/A".to_string()
    };

    let build_time_str = if let Some(ms) = outcome.build_elapsed_ms {
        format!("{:.2} s", ms as f64 / 1000.0)
    } else {
        "N/A".to_string()
    };

    let run_id = options.run_id.as_deref().unwrap_or("standalone");
    let profile_note = crate::report::analysis_profile_note(options.release);

    let all_count = outcome.findings.len();
    let fail_count = outcome
        .findings
        .iter()
        .filter(|e| e.status == "FAIL")
        .count();
    let warn_count = outcome
        .findings
        .iter()
        .filter(|e| e.status == "WARN")
        .count();
    let display_fail = if all_count == 0 { failed } else { fail_count };
    let display_warn = if all_count == 0 { warnings } else { warn_count };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Keller Formal Verification Report</title>
    <style>
        :root {{
            --bg-main: #0d1117;
            --bg-card: #161b22;
            --bg-card-hover: #1f242c;
            --border: #30363d;
            --text-main: #c9d1d9;
            --text-muted: #8b949e;
            --text-bright: #f0f6fc;
            --green: #238636;
            --green-glow: rgba(35, 134, 54, 0.2);
            --red: #da3633;
            --red-glow: rgba(218, 54, 51, 0.2);
            --amber: #d29922;
            --amber-glow: rgba(210, 153, 34, 0.2);
            --blue: #58a6ff;
            --blue-glow: rgba(88, 166, 255, 0.15);
            --font-mono: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace;
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            background: var(--bg-main);
            color: var(--text-main);
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
            line-height: 1.5;
            padding: 2rem 3rem;
        }}
        .header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            border-bottom: 1px solid var(--border);
            padding-bottom: 1.5rem;
            margin-bottom: 2rem;
        }}
        .header h1 {{
            color: var(--text-bright);
            font-size: 1.8rem;
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }}
        .header .subtitle {{
            color: var(--text-muted);
            font-size: 0.9rem;
            margin-top: 0.35rem;
        }}
        .badge {{
            display: inline-block;
            padding: 0.25rem 0.65rem;
            border-radius: 9999px;
            font-size: 0.8rem;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}
        .badge-pass {{ background: var(--green); color: #fff; box-shadow: 0 0 10px var(--green-glow); }}
        .badge-fail {{ background: var(--red); color: #fff; box-shadow: 0 0 10px var(--red-glow); }}
        .badge-warn {{ background: var(--amber); color: #000; box-shadow: 0 0 10px var(--amber-glow); }}
        .status-pill-big {{
            padding: 0.6rem 1.4rem;
            border-radius: 8px;
            font-size: 1.1rem;
            font-weight: bold;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        .bg-green {{ background: var(--green-glow); border: 1px solid var(--green); color: #3fb950; }}
        .bg-red {{ background: var(--red-glow); border: 1px solid var(--red); color: #f85149; }}
        .metrics-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 1rem;
            margin-bottom: 2rem;
        }}
        .metric-card {{
            background: var(--bg-card);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 1.25rem;
            transition: transform 0.15s ease, border-color 0.15s ease;
        }}
        .metric-card:hover {{
            border-color: var(--blue);
            transform: translateY(-2px);
        }}
        .metric-card .title {{
            font-size: 0.85rem;
            color: var(--text-muted);
            text-transform: uppercase;
            letter-spacing: 0.05em;
            margin-bottom: 0.5rem;
        }}
        .metric-card .val {{
            font-size: 2rem;
            font-weight: 700;
            color: var(--text-bright);
        }}
        .metric-card .val.val-pass {{ color: #3fb950; }}
        .metric-card .val.val-fail {{ color: #f85149; }}
        .metric-card .val.val-warn {{ color: #e3b341; }}
        .coverage-gap {{
            background: rgba(210, 153, 34, 0.12);
            border: 1px solid var(--amber);
            border-radius: 8px;
            color: var(--text-bright);
            padding: 0.9rem 1.1rem;
            margin-bottom: 2rem;
            font-size: 0.9rem;
        }}
        .progress-bar-container {{
            background: var(--bg-card);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 1.25rem;
            margin-bottom: 2rem;
        }}
        .progress-bar {{
            height: 12px;
            background: #21262d;
            border-radius: 6px;
            overflow: hidden;
            display: flex;
            margin: 0.75rem 0;
        }}
        .progress-fill-pass {{ background: #2ea043; }}
        .progress-fill-fail {{ background: #da3633; }}
        .progress-fill-warn {{ background: #d29922; }}
        .explorer-section {{
            background: var(--bg-card);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 1.5rem;
        }}
        .toolbar {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 1.25rem;
            gap: 1rem;
            flex-wrap: wrap;
        }}
        .search-box {{
            flex: 1;
            min-width: 250px;
            padding: 0.6rem 1rem;
            background: #0d1117;
            border: 1px solid var(--border);
            border-radius: 6px;
            color: var(--text-bright);
            font-family: inherit;
        }}
        .search-box:focus {{ outline: none; border-color: var(--blue); }}
        .filter-buttons {{ display: flex; gap: 0.5rem; }}
        .filter-btn {{
            background: #21262d;
            border: 1px solid var(--border);
            color: var(--text-main);
            padding: 0.4rem 0.85rem;
            border-radius: 6px;
            cursor: pointer;
            font-size: 0.85rem;
            transition: all 0.15s ease;
        }}
        .filter-btn:hover, .filter-btn.active {{
            background: var(--blue);
            color: #fff;
            border-color: var(--blue);
        }}
        table.findings-table {{
            width: 100%;
            border-collapse: collapse;
            font-size: 0.9rem;
        }}
        table.findings-table th {{
            text-align: left;
            padding: 0.75rem 1rem;
            border-bottom: 2px solid var(--border);
            color: var(--text-muted);
            font-weight: 600;
        }}
        table.findings-table td {{
            padding: 0.85rem 1rem;
            border-bottom: 1px solid var(--border);
            vertical-align: top;
        }}
        table.findings-table tr:hover {{
            background: var(--bg-card-hover);
        }}
        code {{
            font-family: var(--font-mono);
            background: rgba(110, 118, 129, 0.15);
            padding: 0.2rem 0.4rem;
            border-radius: 4px;
            font-size: 0.85em;
            color: #79c0ff;
        }}
        .kind-tag {{
            background: #21262d;
            border: 1px solid #30363d;
            padding: 0.2rem 0.5rem;
            border-radius: 4px;
            font-size: 0.8rem;
            font-family: var(--font-mono);
        }}
        .file-loc {{
            color: var(--text-muted);
            font-family: var(--font-mono);
            font-size: 0.85em;
        }}
        .witness-box {{
            margin-top: 0.5rem;
            background: #0d1117;
            border: 1px solid #30363d;
            border-radius: 6px;
            padding: 0.6rem 0.85rem;
            font-size: 0.85rem;
        }}
        .witness-box ul {{
            list-style: none;
            margin-top: 0.35rem;
        }}
        .witness-box li {{
            margin-bottom: 0.25rem;
            font-family: var(--font-mono);
        }}
        footer {{
            margin-top: 3rem;
            text-align: center;
            color: var(--text-muted);
            font-size: 0.85rem;
            border-top: 1px solid var(--border);
            padding-top: 1.5rem;
        }}
    </style>
</head>
<body>
    <div class="header">
        <div>
            <h1>Keller-Verify Formal Verification Report</h1>
            <div class="subtitle">Run ID: <code>{}</code> &bull; Profile: {} &bull; Build: {} &bull; SMT: {}</div>
        </div>
        <div class="status-pill-big {}">
            {}
        </div>
    </div>

    <div class="metrics-grid">
        <div class="metric-card">
            <div class="title">Total Obligations</div>
            <div class="val">{}</div>
        </div>
        <div class="metric-card">
            <div class="title">Proved (Sound)</div>
            <div class="val val-pass">{}</div>
        </div>
        <div class="metric-card">
            <div class="title">Counterexamples</div>
            <div class="val val-fail">{}</div>
        </div>
        <div class="metric-card">
            <div class="title">Undecided / Warnings</div>
            <div class="val val-warn">{}</div>
        </div>
        <div class="metric-card">
            <div class="title">Panic Sites (Not Modelled)</div>
            <div class="val">{}</div>
        </div>
        <div class="metric-card">
            <div class="title">Not Modelled (Unsupported)</div>
            <div class="val val-warn">{}</div>
        </div>
        <div class="metric-card">
            <div class="title">Panic Sites Decided</div>
            <div class="val">{}</div>
        </div>
    </div>

    {}

    <div class="progress-bar-container">
        <div style="display:flex; justify-content:space-between; font-size:0.9rem;">
            <span>Proof Soundness Ratio</span>
            <strong>{:.1}% Verified Safe</strong>
        </div>
        <div class="progress-bar">
            <div class="progress-fill-pass" style="width: {:.1}%"></div>
            <div class="progress-fill-fail" style="width: {:.1}%"></div>
            <div class="progress-fill-warn" style="width: {:.1}%"></div>
        </div>
        <div style="display:flex; gap:1.5rem; font-size:0.8rem; color:var(--text-muted);">
            <span><span style="color:#2ea043;">&#9632;</span> Proved: {}</span>
            <span><span style="color:#da3633;">&#9632;</span> Fail: {}</span>
            <span><span style="color:#d29922;">&#9632;</span> Undecided: {}</span>
        </div>
    </div>

    <div class="explorer-section">
        <div class="toolbar">
            <input type="text" id="searchInput" class="search-box" placeholder="Search by function, kind, or file..." onkeyup="filterRows()">
            <div class="filter-buttons">
                <button class="filter-btn active" onclick="setFilter('all', this)">All ({})</button>
                <button class="filter-btn" onclick="setFilter('fail', this)">Failures ({})</button>
                <button class="filter-btn" onclick="setFilter('warn', this)">Warnings ({})</button>
                <button class="filter-btn" onclick="setFilter('pass', this)">Passing</button>
            </div>
        </div>

        <div id="countSummary" style="font-size: 0.85rem; color: var(--text-muted); margin-bottom: 0.75rem;">Showing {} findings</div>

        <table class="findings-table">
            <thead>
                <tr>
                    <th style="width: 80px;">Status</th>
                    <th style="width: 250px;">Function</th>
                    <th style="width: 140px;">Kind</th>
                    <th style="width: 200px;">Location</th>
                    <th>Details & Witness</th>
                </tr>
            </thead>
            <tbody id="findingsBody">
                {}
            </tbody>
        </table>
    </div>

    <footer>
        Keller-Verify &bull; Automated Z3 SMT Formal Verification Engine for Rust &bull; <a href="https://github.com/KELLERBABG/Formal-Verification" style="color:var(--blue); text-decoration:none;">GitHub</a>
    </footer>

    <script>
        let currentFilter = 'all';

        function setFilter(filter, btn) {{
            currentFilter = filter;
            document.querySelectorAll('.filter-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            filterRows();
        }}

        function filterRows() {{
            const query = document.getElementById('searchInput').value.toLowerCase();
            const rows = document.querySelectorAll('.finding-row');
            let visible = 0;

            rows.forEach(row => {{
                const matchesSearch = row.getAttribute('data-search').includes(query);
                const isFail = row.classList.contains('status-fail');
                const isWarn = row.classList.contains('status-warn');
                const isPass = row.classList.contains('status-pass');

                let matchesFilter = false;
                if (currentFilter === 'all') matchesFilter = true;
                else if (currentFilter === 'fail' && isFail) matchesFilter = true;
                else if (currentFilter === 'warn' && isWarn) matchesFilter = true;
                else if (currentFilter === 'pass' && isPass) matchesFilter = true;

                const show = matchesSearch && matchesFilter;
                row.style.display = show ? '' : 'none';
                if (show) visible++;
            }});

            const counter = document.getElementById('countSummary');
            if (counter) {{
                counter.textContent = 'Showing ' + visible + ' of ' + rows.length + ' findings';
            }}
        }}

        function toggleDetails(row) {{
            const witness = row.querySelector('.witness-box');
            if (witness) {{
                witness.style.display = (witness.style.display === 'none') ? '' : 'none';
            }}
        }}
    </script>
</body>
</html>"##,
        html_escape(run_id),
        html_escape(profile_note),
        build_time_str,
        solver_time_str,
        overall_status.1,
        overall_status.0,
        total_checks,
        passed,
        failed,
        warnings,
        panics,
        unsupported,
        panic_sites_decided,
        coverage_gap,
        pass_pct,
        pass_pct,
        if total_checks > 0 {
            (failed as f64 / total_checks as f64) * 100.0
        } else {
            0.0
        },
        if total_checks > 0 {
            (warnings as f64 / total_checks as f64) * 100.0
        } else {
            0.0
        },
        passed,
        failed,
        warnings,
        all_count,
        display_fail,
        display_warn,
        all_count,
        findings_rows
    )
}

pub fn write_html_report(
    path: &Path,
    outcome: &MergeOutcome,
    options: &VerifyOptions,
) -> Result<(), String> {
    let html = render_html_report(outcome, options);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, html).map_err(|e| e.to_string())?;
    Ok(())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::VerificationEvent;

    #[test]
    fn test_render_html_report_empty() {
        let outcome = MergeOutcome {
            breach: None,
            findings: Vec::new(),
            gated: Vec::new(),
            suppressed: 0,
            filtered_out: 0,
            max_findings: None,
            not_listed: 0,
            baseline_error: None,
            env: None,
            coverage: Default::default(),
            run_directory: None,
            run_status: "ok".to_string(),
            build_elapsed_ms: Some(150),
            advisory_excluded: 0,
        };
        let options = VerifyOptions::default();
        let html = render_html_report(&outcome, &options);
        assert!(html.contains("Keller-Verify Formal Verification Report"));
        assert!(html.contains("Proof Soundness Ratio"));
    }

    #[test]
    fn test_render_html_report_surfaces_the_unsupported_fraction() {
        // K8: a run that cannot model an assertion kind has to say so in the page,
        // not only on the console. The ratio below counts the unsupported one as
        // unchecked, and the banner names the kind.
        let mut totals = crate::types::Coverage::default();
        totals.reachable_assertions = 4;
        totals.supported_assertions = 3;
        totals.unsupported_assertions = 1;
        totals.unsupported_kinds = vec!["BoundsCheck".to_string()];
        totals.passed = 3;
        let outcome = MergeOutcome {
            breach: None,
            findings: Vec::new(),
            gated: Vec::new(),
            suppressed: 0,
            filtered_out: 0,
            max_findings: None,
            not_listed: 0,
            baseline_error: None,
            env: None,
            coverage: crate::report::RunCoverage {
                workers: 1,
                totals,
                compilers: Vec::new(),
                function_evidence: Vec::new(),
            },
            run_directory: None,
            run_status: "ok".to_string(),
            build_elapsed_ms: Some(150),
            advisory_excluded: 0,
        };
        let options = VerifyOptions::default();
        let html = render_html_report(&outcome, &options);
        assert!(
            html.contains("1 assertion(s) were not attempted"),
            "the unsupported fraction must be stated, not deducible"
        );
        assert!(html.contains("BoundsCheck"), "the skipped kind is named");
        assert!(
            html.contains("Not Modelled (Unsupported)"),
            "the count is a metric, not a footnote"
        );
    }

    #[test]
    fn test_render_html_report_with_finding() {
        let event = VerificationEvent {
            crate_name: "test".to_string(),
            function: "add".to_string(),
            block: "bb1".to_string(),
            status: "FAIL".to_string(),
            kind: "overflow".to_string(),
            defect_class: "overflow-add".to_string(),
            file: "src/lib.rs".to_string(),
            line: Some(42),
            details: "integer overflow".to_string(),
            witness: vec![crate::types::Witness {
                input: "_1".to_string(),
                symbol: "overflow_a".to_string(),
                value: "255".to_string(),
            }],
            witness_kind: "model".to_string(),
        };

        let outcome = MergeOutcome {
            breach: None,
            findings: vec![event.clone()],
            gated: vec![event],
            suppressed: 0,
            filtered_out: 0,
            max_findings: None,
            not_listed: 0,
            baseline_error: None,
            env: None,
            coverage: Default::default(),
            run_directory: None,
            run_status: "fail".to_string(),
            build_elapsed_ms: Some(150),
            advisory_excluded: 0,
        };
        let options = VerifyOptions::default();
        let html = render_html_report(&outcome, &options);
        assert!(html.contains("badge-fail"));
        assert!(html.contains("src/lib.rs:42"));
        assert!(html.contains("255"));
    }
}
