package nilcheck

import (
	"context"
	"fmt"
	"sync"

	"github.com/keller-vir/go-verify/internal/kv"
	"github.com/keller-vir/go-verify/internal/ssago"
)

// NilResult represents a detected nil pointer dereference.
type NilResult struct {
	Description string
	VarName     string
	Line        int
	Col         int
	FuncName    string
	Severity    string // "definite", "possible"
}

// Detector finds potential nil pointer dereferences through SSA analysis.
type Detector struct {
	ssa  *ssago.SSA
	kv   *kv.KellerVIRGraph
	mu   sync.Mutex
}

// NewDetector creates a new nil pointer detector.
func NewDetector(ssa *ssago.SSA, kv *kv.KellerVIRGraph) *Detector {
	return &Detector{
		ssa: ssa,
		kv:  kv,
	}
}

// Detect finds all potential nil pointer dereferences.
// Strategy: track variables that could be nil (from map lookups, type assertions,
// function returns of pointer type) and flag dereferences without nil checks.
func (d *Detector) Detect(ctx context.Context) []NilResult {
	select {
	case <-ctx.Done():
		return nil
	default:
	}

	var results []NilResult

	for _, fn := range d.ssa.Functions {
		// Track which variables are potentially nil
		potentiallyNil := make(map[string]bool)
		nilChecked := make(map[string]bool)

		for _, block := range fn.Blocks {
			if block == nil {
				continue
			}
			for _, instr := range block.Instructions {
				switch instr.Op {
				case "alloc":
					// A fresh allocation is not nil (unless it's a pointer from a map)
					// Mark as potentially nil if it looks like a map lookup or type assertion
					if isNilSource(instr.Operands) {
						potentiallyNil[instr.Dest] = true
					}

				case "load":
					// Loading from a pointer that might be nil
					if potentiallyNil[instr.Dest] && !nilChecked[instr.Dest] {
						results = append(results, NilResult{
							Description: fmt.Sprintf("potential nil dereference of %q at line %d in %s",
								instr.Dest, instr.Line, fn.Name),
							VarName:  instr.Dest,
							Line:     instr.Line,
							Col:      instr.Col,
							FuncName: fn.Name,
							Severity: "possible",
						})
					}

				case "index":
					// Indexing into a nil slice/map
					if potentiallyNil[instr.Dest] && !nilChecked[instr.Dest] {
						results = append(results, NilResult{
							Description: fmt.Sprintf("potential nil index on %q at line %d in %s",
								instr.Dest, instr.Line, fn.Name),
							VarName:  instr.Dest,
							Line:     instr.Line,
							Col:      instr.Col,
							FuncName: fn.Name,
							Severity: "possible",
						})
					}

				case "branch":
					// A branch might be a nil check (if x != nil)
					for _, operand := range instr.Operands {
						if isNilCheck(operand) {
							// Extract variable name from the check
							varName := extractVarFromNilCheck(operand)
							if varName != "" {
								nilChecked[varName] = true
							}
						}
					}
				}
			}
		}
	}

	return results
}

// isNilSource checks if an operand suggests a nil-able source.
func isNilSource(operands []string) bool {
	for _, op := range operands {
		if contains(op, "map[") || contains(op, ".(") || contains(op, "interface") {
			return true
		}
	}
	return false
}

// isNilCheck checks if an operand string represents a nil comparison.
func isNilCheck(s string) bool {
	return contains(s, "!= nil") || contains(s, "== nil")
}

// extractVarFromNilCheck extracts the variable name from a nil check expression.
func extractVarFromNilCheck(s string) string {
	// Simple heuristic: look for identifier before "!= nil" or "== nil"
	for i := 0; i < len(s); i++ {
		if i+8 <= len(s) && s[i:i+8] == "!= nil" {
			return s[:i-1] // crude extraction
		}
		if i+7 <= len(s) && s[i:i+7] == "== nil" {
			return s[:i-1]
		}
	}
	return ""
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > 0 && indexOf(s, substr) >= 0)
}

func indexOf(s, sub string) int {
	for i := 0; i <= len(s)-len(sub); i++ {
		if s[i:i+len(sub)] == sub {
			return i
		}
	}
	return -1
}
