package main

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"github.com/keller-vir/go-verify/internal/concurrency"
	"github.com/keller-vir/go-verify/internal/deadlock"
	"github.com/keller-vir/go-verify/internal/kv"
	"github.com/keller-vir/go-verify/internal/nilcheck"
	"github.com/keller-vir/go-verify/internal/race"
	"github.com/keller-vir/go-verify/internal/report"
	"github.com/keller-vir/go-verify/internal/slice"
	"github.com/keller-vir/go-verify/internal/ssago"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintf(os.Stderr, "usage: go-verify <package-path> [-o report.json]\n")
		os.Exit(1)
	}

	pkgPath := os.Args[1]
	outputPath := ""
	for i := 2; i < len(os.Args); i++ {
		if os.Args[i] == "-o" && i+1 < len(os.Args) {
			outputPath = os.Args[i+1]
			i++
		}
	}

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	fmt.Fprintf(os.Stderr, "[go-verify] analyzing: %s\n", pkgPath)
	fmt.Fprintln(os.Stderr, "[go-verify] mode: audit-only (no source mutation)")

	// Phase 1: Parse and build Go SSA
	ssaResult, err := ssago.BuildSSA(ctx, pkgPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "[go-verify] ERROR: %v\n", err)
		os.Exit(2)
	}
	fmt.Fprintf(os.Stderr, "[go-verify] SSA: %d functions, %d goroutines, %d channels\n",
		ssaResult.FuncCount, ssaResult.GoroutineCount, ssaResult.ChannelCount)

	// Phase 2: Lower to Keller-VIR hypergraph IR
	kvGraph, err := kv.LowerToKellerVIR(ctx, ssaResult)
	if err != nil {
		fmt.Fprintf(os.Stderr, "[go-verify] ERROR lowering to KV: %v\n", err)
		os.Exit(2)
	}
	fmt.Fprintf(os.Stderr, "[go-verify] KV IR: %d hyperedges, %d vertices\n",
		kvGraph.HyperedgeCount(), kvGraph.VertexCount())

	// Phase 3: Concurrency analysis - build happens-before graph
	hbGraph := concurrency.BuildHappensBefore(ctx, ssaResult, kvGraph)
	fmt.Fprintf(os.Stderr, "[go-verify] HB graph: %d edges\n", hbGraph.EdgeCount())

	// Phase 4: Deadlock detection
	dlDetector := deadlock.NewDetector(kvGraph, hbGraph)
	dlResults := dlDetector.Detect(ctx)
	fmt.Fprintf(os.Stderr, "[go-verify] deadlocks found: %d\n", len(dlResults))

	// Phase 5: Data race detection
	raceDetector := race.NewDetector(ssaResult, kvGraph, hbGraph)
	raceResults := raceDetector.Detect(ctx)
	fmt.Fprintf(os.Stderr, "[go-verify] data races found: %d\n", len(raceResults))

	// Phase 6: Nil pointer dereference detection
	nilDetector := nilcheck.NewDetector(ssaResult, kvGraph)
	nilResults := nilDetector.Detect(ctx)
	fmt.Fprintf(os.Stderr, "[go-verify] nil derefs found: %d\n", len(nilResults))

	// Phase 7: Slice panic detection
	sliceDetector := slice.NewDetector(ssaResult, kvGraph)
	sliceResults := sliceDetector.Detect(ctx)
	fmt.Fprintf(os.Stderr, "[go-verify] slice panics found: %d\n", len(sliceResults))

	// Phase 8: Generate audit report (no source mutation)
	rep := report.AuditReport{
		PackagePath:    pkgPath,
		Timestamp:      time.Now().UTC(),
		SSAStats:       ssaResult.Stats(),
		KVStats:        kvGraph.Stats(),
		Deadlocks:      dlResults,
		DataRaces:      raceResults,
		NilDerefs:     nilResults,
		SlicePanics:   sliceResults,
	}

	if outputPath == "" {
		dir := filepath.Dir(pkgPath)
		outputPath = filepath.Join(dir, "go-verify-audit.json")
	}

	if err := rep.Write(outputPath); err != nil {
		fmt.Fprintf(os.Stderr, "[go-verify] ERROR writing report: %v\n", err)
		os.Exit(3)
	}

	fmt.Fprintf(os.Stderr, "[go-verify] report written: %s\n", outputPath)
	fmt.Fprintf(os.Stderr, "[go-verify] total findings: %d\n", rep.TotalFindings())
}
