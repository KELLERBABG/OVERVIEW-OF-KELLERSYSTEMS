package race

import (
	"context"
	"fmt"
	"sync"

	"github.com/keller-vir/go-verify/internal/concurrency"
	"github.com/keller-vir/go-verify/internal/kv"
	"github.com/keller-vir/go-verify/internal/ssago"
)

// RaceResult represents a detected data race.
type RaceResult struct {
	Description string
	VarName     string
	OpA         string // first conflicting access
	OpB         string // second conflicting access
	GoroutineA  int
	GoroutineB  int
	LineA       int
	LineB       int
	Severity    string // "definite", "possible"
}

// Detector finds data races on shared memory.
type Detector struct {
	ssa  *ssago.SSA
	kv   *kv.KellerVIRGraph
	hb   *concurrency.HappensBeforeGraph
	mu   sync.Mutex
}

// NewDetector creates a new data race detector.
func NewDetector(ssa *ssago.SSA, kv *kv.KellerVIRGraph, hb *concurrency.HappensBeforeGraph) *Detector {
	return &Detector{
		ssa: ssa,
		kv:  kv,
		hb:  hb,
	}
}

// Detect finds all data races by checking for conflicting accesses
// that are not ordered by happens-before.
func (d *Detector) Detect(ctx context.Context) []RaceResult {
	select {
	case <-ctx.Done():
		return nil
	default:
	}

	var results []RaceResult

	// Collect all memory accesses (load/store) with their goroutine context
	type memAccess struct {
		varName   string
		op        string // "load" or "store"
		goroutine int
		line      int
		col       int
	}

	var accesses []memAccess

	for _, fn := range d.ssa.Functions {
		for _, block := range fn.Blocks {
			if block == nil {
				continue
			}
			for _, instr := range block.Instructions {
				if instr.Op == "load" || instr.Op == "store" {
					gid := -1
					if instr.Goroutine != nil {
						gid = instr.Goroutine.ID
					}
					accesses = append(accesses, memAccess{
						varName:   instr.Dest,
						op:        instr.Op,
						goroutine: gid,
						line:      instr.Line,
						col:       instr.Col,
					})
				}
			}
		}
	}

	// Check all pairs of accesses to the same variable in different goroutines
	for i := 0; i < len(accesses); i++ {
		for j := i + 1; j < len(accesses); j++ {
			a, b := accesses[i], accesses[j]

			// Must be same variable
			if a.varName != b.varName {
				continue
			}
			// Must be in different goroutines (or one is in main, other in goroutine)
			if a.goroutine == b.goroutine && a.goroutine >= 0 {
				continue // Same goroutine: program order applies
			}
			// At least one must be a write (store) for it to be a race
			if a.op != "store" && b.op != "store" {
				continue // Two reads don't conflict
			}

			// Check if happens-before orders them
			opA := fmt.Sprintf("%s@%d:%d", a.op, a.line, a.col)
			opB := fmt.Sprintf("%s@%d:%d", b.op, b.line, b.col)

			if d.hb.HappensBefore(opA, opB) || d.hb.HappensBefore(opB, opA) {
				continue // Ordered by happens-before: no race
			}

			severity := "possible"
			if a.goroutine >= 0 && b.goroutine >= 0 && a.goroutine != b.goroutine {
				severity = "definite"
			}

			results = append(results, RaceResult{
				Description: fmt.Sprintf("data race on %q: %s@%d and %s@%d in different goroutines",
					a.varName, a.op, a.line, b.op, b.line),
				VarName:    a.varName,
				OpA:        opA,
				OpB:        opB,
				GoroutineA: a.goroutine,
				GoroutineB: b.goroutine,
				LineA:      a.line,
				LineB:      b.line,
				Severity:   severity,
			})
		}
	}

	return results
}
