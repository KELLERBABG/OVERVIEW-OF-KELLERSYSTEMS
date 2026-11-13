package concurrency

import (
	"context"
	"sync"

	"github.com/keller-vir/go-verify/internal/kv"
	"github.com/keller-vir/go-verify/internal/ssago"
)

// HappensBeforeGraph models the happens-before relationship between operations.
// An edge from A to B means operation A happens-before operation B.
type HappensBeforeGraph struct {
	mu    sync.RWMutex
	edges map[string]map[string]bool // from -> set of "to"
	nodes []string
}

// NewHappensBefore creates an empty happens-before graph.
func NewHappensBefore() *HappensBeforeGraph {
	return &HappensBeforeGraph{
		edges: make(map[string]map[string]bool),
	}
}

// EdgeCount returns the total number of edges.
func (g *HappensBeforeGraph) EdgeCount() int {
	g.mu.RLock()
	defer g.mu.RUnlock()
	count := 0
	for _, targets := range g.edges {
		count += len(targets)
	}
	return count
}

// AddEdge adds a happens-before edge from opA to opB.
func (g *HappensBeforeGraph) AddEdge(opA, opB string) {
	g.mu.Lock()
	defer g.mu.Unlock()
	if g.edges[opA] == nil {
		g.edges[opA] = make(map[string]bool)
	}
	g.edges[opA][opB] = true
	for _, n := range []string{opA, opB} {
		found := false
		for _, existing := range g.nodes {
			if existing == n {
				found = true
				break
			}
		}
		if !found {
			g.nodes = append(g.nodes, n)
		}
	}
}

// HappensBefore returns true if opA happens-before opB.
func (g *HappensBeforeGraph) HappensBefore(opA, opB string) bool {
	g.mu.RLock()
	defer g.mu.RUnlock()
	return g.reachable(opA, opB)
}

func (g *HappensBeforeGraph) reachable(from, to string) bool {
	visited := make(map[string]bool)
	queue := []string{from}
	for len(queue) > 0 {
		current := queue[0]
		queue = queue[1:]
		if current == to {
			return true
		}
		if visited[current] {
			continue
		}
		visited[current] = true
		if targets, ok := g.edges[current]; ok {
			for t := range targets {
				if !visited[t] {
					queue = append(queue, t)
				}
			}
		}
	}
	return false
}

// BuildHappensBefore constructs the happens-before graph from SSA and KV IR.
// Rules:
// 1. Program order within a goroutine: sequential instructions are ordered.
// 2. Channel send happens-before matching channel receive (for unbuffered channels).
// 3. Goroutine spawn happens-before all operations in the spawned goroutine.
// 4. close(ch) happens-before subsequent recv on ch returning zero value.
func BuildHappensBefore(ctx context.Context, ssa *ssago.SSA, graph *kv.KellerVIRGraph) *HappensBeforeGraph {
	select {
	case <-ctx.Done():
		return NewHappensBefore()
	default:
	}

	hb := NewHappensBefore()

	// Rule 1: Program order within each function/goroutine
	for _, fn := range ssa.Functions {
		for i := 0; i < len(fn.Blocks)-1; i++ {
			blockA := fn.Blocks[i]
			blockB := fn.Blocks[i+1]
			if blockA == nil || blockB == nil {
				continue
			}
			for _, instrA := range blockA.Instructions {
				for _, instrB := range blockB.Instructions {
					opA := opKey(instrA)
					opB := opKey(instrB)
					hb.AddEdge(opA, opB)
				}
			}
		}
	}

	// Rule 2: Channel synchronization
	for _, ch := range ssa.Channels {
		for _, sendOp := range ch.Sends {
			for _, recvOp := range ch.Receives {
				if sendOp.Block != nil && recvOp.Block != nil {
					sendKey := opKeyFromChannel(sendOp)
					recvKey := opKeyFromChannel(recvOp)
					hb.AddEdge(sendKey, recvKey)
				}
			}
		}

		// Rule 4: close happens-before subsequent recv
		for _, closeOp := range ch.Closes {
			for _, recvOp := range ch.Receives {
				if closeOp.Block != nil && recvOp.Block != nil {
					closeKey := opKeyFromChannel(closeOp)
					recvKey := opKeyFromChannel(recvOp)
					hb.AddEdge(closeKey, recvKey)
				}
			}
		}
	}

	// Rule 3: Goroutine spawn happens-before goroutine body
	for _, goro := range ssa.Goroutines {
		if goro.ParentFunc != nil {
			for _, block := range goro.ParentFunc.Blocks {
				if block == nil {
					continue
				}
				for _, instr := range block.Instructions {
					if instr.Op == "goroutine" {
						spawnKey := opKey(instr)
						// All operations in the goroutine happen after spawn
						for _, gBlock := range goro.Blocks {
							if gBlock == nil {
								continue
							}
							for _, gInstr := range gBlock.Instructions {
								hb.AddEdge(spawnKey, opKey(gInstr))
							}
						}
					}
				}
			}
		}
	}

	return hb
}

func opKey(instr ssago.Instruction) string {
	return fmt.Sprintf("%s@%d:%d", instr.Op, instr.Line, instr.Col)
}

func opKeyFromChannel(op ssago.ChannelOp) string {
	return fmt.Sprintf("%s@%d:%d", op.Kind, op.Line, op.Col)
}
