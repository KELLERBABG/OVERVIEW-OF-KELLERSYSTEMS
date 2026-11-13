package kv

import (
	"context"
	"fmt"
	"sync"

	"github.com/keller-vir/go-verify/internal/ssago"
)

// KellerVIRGraph is the hypergraph IR representing the program after lowering.
// It models goroutine and channel state transitions as hyperedges.
type KellerVIRGraph struct {
	vertices   map[string]*Vertex
	hyperedges []*Hyperedge
	nextVID    int
	mu         sync.RWMutex
}

// Vertex is a node in the hypergraph representing a program state point.
type Vertex struct {
	ID       int
	Label    string
	GoroutineID int
	BlockID    int
	// State annotations
	NilSet     map[string]bool // variables known to be nil
	SliceBounds map[string]*SliceBound
	ChannelState map[string]ChanState
}

// Hyperedge represents a transition in the program (instruction or control flow).
type Hyperedge struct {
	ID        int
	Kind      string // "alloc", "load", "store", "send", "recv", "close", "fork", "join", "branch", "nil_check", "index"
	Tail      []*Vertex // input vertices (tail set)
	Head      []*Vertex // output vertices (head set)
	Func      string   // semantic function
	GoroutineID int
	BlockID    int
	Line       int
	Col        int
}

// ChanState represents the state of a channel at a program point.
type ChanState struct {
	IsClosed   bool
	HasValue   bool
	BufferCount int
}

// SliceBound tracks bounds information for a slice variable.
type SliceBound struct {
	MinLen int
	MaxLen int
	Exact  bool
}

// NewGraph creates a new empty Keller-VIR hypergraph.
func NewGraph() *KellerVIRGraph {
	return &KellerVIRGraph{
		vertices: make(map[string]*Vertex),
	}
}

// VertexCount returns the number of vertices in the graph.
func (g *KellerVIRGraph) VertexCount() int {
	g.mu.RLock()
	defer g.mu.RUnlock()
	return len(g.vertices)
}

// HyperedgeCount returns the number of hyperedges.
func (g *KellerVIRGraph) HyperedgeCount() int {
	g.mu.RLock()
	defer g.mu.RUnlock()
	return len(g.hyperedges)
}

// Stats returns summary statistics.
func (g *KellerVIRGraph) Stats() map[string]int {
	g.mu.RLock()
	defer g.mu.RUnlock()
	kindCounts := make(map[string]int)
	for _, he := range g.hyperedges {
		kindCounts[he.Kind]++
	}
	return map[string]int{
		"vertices":   len(g.vertices),
		"hyperedges": len(g.hyperedges),
	}
}

// LowerToKellerVIR converts a Go SSA representation into the Keller-VIR hypergraph.
func LowerToKellerVIR(ctx context.Context, ssa *ssago.SSA) (*KellerVIRGraph, error) {
	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	default:
	}

	g := NewGraph()

	for _, fn := range ssa.Functions {
		if err := g.lowerFunction(ctx, fn); err != nil {
			return nil, fmt.Errorf("lowering function %q: %w", fn.Name, err)
		}
	}

	// Lower goroutine spawn points as fork hyperedges
	for _, goro := range ssa.Goroutines {
		g.lowerGoroutineFork(goro)
	}

	// Lower channel operations as state transitions
	for _, ch := range ssa.Channels {
		g.lowerChannelTransitions(ch)
	}

	return g, nil
}

func (g *KellerVIRGraph) lowerFunction(ctx context.Context, fn *ssago.Function) error {
	for _, block := range fn.Blocks {
		if block == nil {
			continue
		}
		var prevHead []*Vertex

		for _, instr := range block.Instructions {
			select {
			case <-ctx.Done():
				return ctx.Err()
			default:
			}

			tail := prevHead
			if len(tail) == 0 {
				tail = g.getOrCreateVertex(fmt.Sprintf("entry_%s_%d", fn.Name, block.ID), -1, block.ID)
			}

			head := g.applyInstruction(tail, instr, fn.Name, block.ID)
			prevHead = head
		}
	}
	return nil
}

func (g *KellerVIRGraph) applyInstruction(tail []*Vertex, instr ssago.Instruction, funcName string, blockID int) []*Vertex {
	var head []*Vertex

	switch instr.Op {
	case "alloc":
		v := g.getOrCreateVertex(fmt.Sprintf("alloc_%s_%d", instr.Dest, instr.Line), -1, blockID)
		head = []*Vertex{v}
		g.addHyperedge(&Hyperedge{
			ID:    len(g.hyperedges),
			Kind:  "alloc",
			Tail:  tail,
			Head:  head,
			Func:  "alloc",
			BlockID: blockID,
			Line:  instr.Line,
			Col:   instr.Col,
		})

	case "load":
		v := g.getOrCreateVertex(fmt.Sprintf("load_%s_%d", instr.Dest, instr.Line), -1, blockID)
		head = []*Vertex{v}
		g.addHyperedge(&Hyperedge{
			ID:    len(g.hyperedges),
			Kind:  "load",
			Tail:  tail,
			Head:  head,
			Func:  "mem_load",
			BlockID: blockID,
			Line:  instr.Line,
			Col:   instr.Col,
		})

	case "store":
		v := g.getOrCreateVertex(fmt.Sprintf("store_%s_%d", instr.Dest, instr.Line), -1, blockID)
		head = []*Vertex{v}
		g.addHyperedge(&Hyperedge{
			ID:    len(g.hyperedges),
			Kind:  "store",
			Tail:  tail,
			Head:  head,
			Func:  "mem_store",
			BlockID: blockID,
			Line:  instr.Line,
			Col:   instr.Col,
		})

	case "send":
		v := g.getOrCreateVertex(fmt.Sprintf("send_%s_%d", instr.Dest, instr.Line), -1, blockID)
		head = []*Vertex{v}
		g.addHyperedge(&Hyperedge{
			ID:    len(g.hyperedges),
			Kind:  "send",
			Tail:  tail,
			Head:  head,
			Func:  "chan_send",
			BlockID: blockID,
			Line:  instr.Line,
			Col:   instr.Col,
		})

	case "recv":
		v := g.getOrCreateVertex(fmt.Sprintf("recv_%s_%d", instr.Dest, instr.Line), -1, blockID)
		head = []*Vertex{v}
		g.addHyperedge(&Hyperedge{
			ID:    len(g.hyperedges),
			Kind:  "recv",
			Tail:  tail,
			Head:  head,
			Func:  "chan_recv",
			BlockID: blockID,
			Line:  instr.Line,
			Col:   instr.Col,
		})

	case "close":
		v := g.getOrCreateVertex(fmt.Sprintf("close_%s_%d", instr.Dest, instr.Line), -1, blockID)
		head = []*Vertex{v}
		g.addHyperedge(&Hyperedge{
			ID:    len(g.hyperedges),
			Kind:  "close",
			Tail:  tail,
			Head:  head,
			Func:  "chan_close",
			BlockID: blockID,
			Line:  instr.Line,
			Col:   instr.Col,
		})

	case "goroutine":
		v := g.getOrCreateVertex(fmt.Sprintf("fork_%s_%d", instr.Dest, instr.Line), -1, blockID)
		head = []*Vertex{v}
		g.addHyperedge(&Hyperedge{
			ID:    len(g.hyperedges),
			Kind:  "fork",
			Tail:  tail,
			Head:  head,
			Func:  "goroutine_spawn",
			BlockID: blockID,
			Line:  instr.Line,
			Col:   instr.Col,
		})

	case "branch":
		v := g.getOrCreateVertex(fmt.Sprintf("branch_%d_%d", blockID, instr.Line), -1, blockID)
		head = []*Vertex{v}
		g.addHyperedge(&Hyperedge{
			ID:    len(g.hyperedges),
			Kind:  "branch",
			Tail:  tail,
			Head:  head,
			Func:  "cond_branch",
			BlockID: blockID,
			Line:  instr.Line,
			Col:   instr.Col,
		})

	case "index":
		v := g.getOrCreateVertex(fmt.Sprintf("index_%s_%d", instr.Dest, instr.Line), -1, blockID)
		head = []*Vertex{v}
		g.addHyperedge(&Hyperedge{
			ID:    len(g.hyperedges),
			Kind:  "index",
			Tail:  tail,
			Head:  head,
			Func:  "slice_index",
			BlockID: blockID,
			Line:  instr.Line,
			Col:   instr.Col,
		})

	case "len":
		v := g.getOrCreateVertex(fmt.Sprintf("len_%s_%d", instr.Dest, instr.Line), -1, blockID)
		head = []*Vertex{v}
		g.addHyperedge(&Hyperedge{
			ID:    len(g.hyperedges),
			Kind:  "len",
			Tail:  tail,
			Head:  head,
			Func:  "slice_len",
			BlockID: blockID,
			Line:  instr.Line,
			Col:   instr.Col,
		})

	default:
		v := g.getOrCreateVertex(fmt.Sprintf("%s_%d_%d", instr.Op, blockID, instr.Line), -1, blockID)
		head = []*Vertex{v}
		g.addHyperedge(&Hyperedge{
			ID:    len(g.hyperedges),
			Kind:  instr.Op,
			Tail:  tail,
			Head:  head,
			Func:  instr.Op,
			BlockID: blockID,
			Line:  instr.Line,
			Col:   instr.Col,
		})
	}

	return head
}

func (g *KellerVIRGraph) lowerGoroutineFork(goro *ssago.Goroutine) {
	if goro == nil || goro.ParentFunc == nil {
		return
	}
	// The fork is already represented in the parent function's blocks.
	// Here we ensure the goroutine's entry vertex is connected.
	if goro.Entry != nil {
		g.getOrCreateVertex(fmt.Sprintf("goroutine_entry_%d", goro.ID), goro.ID, -1)
	}
}

func (g *KellerVIRGraph) lowerChannelTransitions(ch *ssago.Channel) {
	for _, op := range ch.Sends {
		if op.Block != nil {
			g.getOrCreateVertex(fmt.Sprintf("chan_send_%s_%d", ch.Name, op.Line), -1, op.Block.ID)
		}
	}
	for _, op := range ch.Receives {
		if op.Block != nil {
			g.getOrCreateVertex(fmt.Sprintf("chan_recv_%s_%d", ch.Name, op.Line), -1, op.Block.ID)
		}
	}
	for _, op := range ch.Closes {
		if op.Block != nil {
			g.getOrCreateVertex(fmt.Sprintf("chan_close_%s_%d", ch.Name, op.Line), -1, op.Block.ID)
		}
	}
}

func (g *KellerVIRGraph) getOrCreateVertex(label string, goroutineID, blockID int) *Vertex {
	g.mu.Lock()
	defer g.mu.Unlock()

	if v, ok := g.vertices[label]; ok {
		return v
	}

	v := &Vertex{
		ID:           g.nextVID,
		Label:        label,
		GoroutineID: goroutineID,
		BlockID:      blockID,
		NilSet:       make(map[string]bool),
		SliceBounds:  make(map[string]*SliceBound),
		ChannelState: make(map[string]ChanState),
	}
	g.nextVID++
	g.vertices[label] = v
	return v
}

func (g *KellerVIRGraph) addHyperedge(he *Hyperedge) {
	g.mu.Lock()
	defer g.mu.Unlock()
	g.hyperedges = append(g.hyperedges, he)
}
