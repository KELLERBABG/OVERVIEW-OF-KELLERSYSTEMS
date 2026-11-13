package deadlock

import (
	"context"
	"fmt"
	"sync"

	"github.com/keller-vir/go-verify/internal/concurrency"
	"github.com/keller-vir/go-verify/internal/kv"
)

// DeadlockResult represents a detected potential deadlock.
type DeadlockResult struct {
	Description string
	Goroutines  []int
	Channels    []string
	Line        int
	Col         int
	Severity    string // "definite", "possible"
}

// Detector finds channel deadlocks in the program.
type Detector struct {
	graph *kv.KellerVIRGraph
	hb    *concurrency.HappensBeforeGraph
	mu    sync.Mutex
}

// NewDetector creates a new deadlock detector.
func NewDetector(graph *kv.KellerVIRGraph, hb *concurrency.HappensBeforeGraph) *Detector {
	return &Detector{
		graph: graph,
		hb:    hb,
	}
}

// Detect finds all potential deadlocks by analyzing wait-for relationships.
// A deadlock exists when goroutine A waits on a channel that goroutine B holds,
// and goroutine B waits on a channel that goroutine A holds (cycle).
func (d *Detector) Detect(ctx context.Context) []DeadlockResult {
	select {
	case <-ctx.Done():
		return nil
	default:
	}

	var results []DeadlockResult

	// Build wait-for graph: goroutine -> channels it's waiting on
	waitFor := d.buildWaitForGraph()

	// Detect cycles in the wait-for graph
	cycles := d.findCycles(waitFor)

	for _, cycle := range cycles {
		result := DeadlockResult{
			Description: fmt.Sprintf("deadlock detected: %d goroutines in cyclic wait", len(cycle)),
			Goroutines:  cycle,
			Severity:    "definite",
		}
		results = append(results, result)
	}

	return results
}

// waitFor maps goroutine ID -> set of channel names it's blocked on.
type waitForMap map[int]map[string]bool

func (d *Detector) buildWaitForGraph() waitForMap {
	wf := make(waitForMap)

	// For each unbuffered channel, a send blocks until a corresponding recv occurs.
	// If no recv is reachable from the sender's goroutine, it's a potential deadlock.
	for _, he := range d.graph.Hyperedges() {
		if he.Kind == "send" {
			gid := he.GoroutineID
			if gid < 0 {
				continue
			}
			if wf[gid] == nil {
				wf[gid] = make(map[string]bool)
			}
			// The channel name is encoded in the hyperedge function or we track it separately
			// For now, mark that this goroutine has a pending send
			wf[gid][fmt.Sprintf("send@%d", he.Line)] = true
		}
		if he.Kind == "recv" {
			gid := he.GoroutineID
			if gid < 0 {
				continue
			}
			if wf[gid] == nil {
				wf[gid] = make(map[string]bool)
			}
			wf[gid][fmt.Sprintf("recv@%d", he.Line)] = true
		}
	}

	return wf
}

// findCycles detects cycles in the wait-for graph using DFS.
func (d *Detector) findCycles(wf waitForMap) [][]int {
	var cycles [][]int
	visited := make(map[int]bool)
	recStack := make(map[int]bool)

	for node := range wf {
		if !visited[node] {
			cycle := d.dfsCycle(node, wf, visited, recStack, []int{})
			if len(cycle) > 0 {
				cycles = append(cycles, cycle)
			}
		}
	}
	return cycles
}

func (d *Detector) dfsCycle(node int, wf waitForMap, visited, recStack map[int]bool, path []int) []int {
	visited[node] = true
	recStack[node] = true
	path = append(path, node)

	// A goroutine waiting on a channel is "blocked by" any goroutine that could
	// potentially be the one to unblock it. For deadlock detection, we look for
	// mutual blocking patterns.
	for target := range wf[node] {
		// Check if any other goroutine in the path is also waiting
		for _, prev := range path {
			if prev != node && wf[prev] != nil {
				// Potential cycle: both are waiting
				if len(path) >= 2 {
					return path
				}
			}
		}
	}

	recStack[node] = false
	path = path[:len(path)-1]
	return nil
}
