package ssago

import (
	"context"
	"fmt"
	"go/ast"
	"go/token"
	"go/types"
	"strings"
)

// SSA represents the Static Single Assignment form of a Go package,
// augmented with goroutine and channel tracking for concurrency analysis.
type SSA struct {
	PackagePath  string
	Functions    []*Function
	Goroutines   []*Goroutine
	Channels     []*Channel
	FileSet      *token.FileSet
	TypeInfo     *types.Info
}

// Function represents an SSA function with basic blocks and phi nodes.
type Function struct {
	Name       string
	Params     []string
	Blocks     []*BasicBlock
	Entry      *BasicBlock
	Exit       *BasicBlock
	Goroutines []*Goroutine // goroutines spawned within this function
}

// BasicBlock is a node in the SSA control flow graph.
type BasicBlock struct {
	ID        int
	Predecessors []*BasicBlock
	Successors   []*BasicBlock
	Instructions []Instruction
	PhiNodes     []PhiNode
}

// Instruction represents a single SSA instruction.
type Instruction struct {
	Op         string // "alloc", "load", "store", "send", "recv", "close", "call", "branch", "phi", "nil", "len", "index"
	Dest       string
	Operands   []string
	Channel    *Channel // set for send/recv/close ops
	Mem        string   // memory location for load/store
	Line       int
	Col        int
	Goroutine  *Goroutine
}

// PhiNode represents an SSA phi node at a control flow merge point.
type PhiNode struct {
	Dest    string
	Incoming []string
	Block   *BasicBlock
}

// Goroutine models a goroutine with its execution context.
type Goroutine struct {
	ID         int
	FuncName   string
	SpawnLine  int
	SpawnCol   int
	Blocks     []*BasicBlock
	Entry      *BasicBlock
	Exit       *BasicBlock
	ParentFunc *Function
}

// Channel models a Go channel with its state transitions.
type Channel struct {
	Name    string
	Type    string // "chan", "chan<-", "<-chan"
	Line    int
	Col     int
	IsUnbuffered bool
	BufferSize   int
	Sends       []ChannelOp
	Receives    []ChannelOp
	Closes      []ChannelOp
}

// ChannelOp records a send/recv/close operation on a channel.
type ChannelOp struct {
	Kind      string // "send", "recv", "close"
	Goroutine *Goroutine
	Block     *BasicBlock
	Line      int
	Col       int
	Value     string
}

// BuildSSA parses a Go package and constructs the augmented SSA representation.
func BuildSSA(ctx context.Context, pkgPath string) (*SSA, error) {
	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	default:
	}

	fset := token.NewFileSet()
	pkgs, err := loadPackage(fset, pkgPath)
	if err != nil {
		return nil, fmt.Errorf("loading package %q: %w", pkgPath, err)
	}

	ssa := &SSA{
		PackagePath: pkgPath,
		FileSet:     fset,
		TypeInfo:    &types.Info{},
	}

	for _, file := range pkgs {
		processFile(ssa, file)
	}

	// Assign IDs and build cross-references
	assignIDs(ssa)
	buildCFG(ssa)

	return ssa, nil
}

// Stats returns summary statistics about the SSA.
func (s *SSA) Stats() map[string]int {
	return map[string]int{
		"functions":  len(s.Functions),
		"goroutines": len(s.Goroutines),
		"channels":   len(s.Channels),
	}
}

func processFile(ssa *SSA, file *ast.File) {
	for _, decl := range file.Decls {
		fnDecl, ok := decl.(*ast.FuncDecl)
		if !ok {
			continue
		}
		fn := &Function{
			Name:   fnDecl.Name.Name,
			Entry:  &BasicBlock{ID: -1},
			Exit:   &BasicBlock{ID: -1},
		}

		if fnDecl.Type.Params != nil {
			for _, param := range fnDecl.Type.Params.List {
				for _, name := range param.Names {
					fn.Params = append(fn.Params, name.Name)
				}
			}
		}

		if fnDecl.Body != nil {
			builder := &blockBuilder{fn: fn, ssa: ssa}
			builder.build(fnDecl.Body)
		}

		ssa.Functions = append(ssa.Functions, fn)
	}
}

// blockBuilder walks the AST and constructs basic blocks with SSA instructions.
type blockBuilder struct {
	fn   *Function
	ssa  *SSa
	nextBlockID int
}

func (b *blockBuilder) build(body *ast.BlockStmt) {
	current := b.fn.Entry
	for _, stmt := range body.List {
		b.processStmt(current, stmt)
	}
}

func (b *blockBuilder) processStmt(block *BasicBlock, stmt ast.Stmt) {
	switch s := stmt.(type) {
	case *ast.AssignStmt:
		b.processAssign(block, s)
	case *ast.ExprStmt:
		b.processExprStmt(block, s)
	case *ast.IfStmt:
		b.processIf(block, s)
	case *ast.ForStmt:
		b.processFor(block, s)
	case *ast.RangeStmt:
		b.processRange(block, s)
	case *ast.DeclStmt:
		if gen, ok := s.Decl.(*ast.GenDecl); ok {
			for _, spec := range gen.Specs {
				if vs, ok := spec.(*ast.ValueSpec); ok {
					for _, name := range vs.Names {
						block.Instructions = append(block.Instructions, Instruction{
							Op:     "alloc",
							Dest:   name.Name,
							Line:   b.ssa.FileSet.Position(s.Pos()).Line,
							Col:    b.ssa.FileSet.Position(s.Pos()).Column,
						})
					}
				}
			}
		}
	case *ast.ReturnStmt:
		block.Instructions = append(block.Instructions, Instruction{
			Op:   "return",
			Line: b.ssa.FileSet.Position(s.Pos()).Line,
			Col:  b.ssa.FileSet.Position(s.Pos()).Column,
		})
	case *ast.SendStmt:
		b.processSend(block, s)
	case *ast.IncDecStmt:
		block.Instructions = append(block.Instructions, Instruction{
			Op:   "store",
			Dest: exprString(s.X),
			Line: b.ssa.FileSet.Position(s.Pos()).Line,
			Col:  b.ssa.FileSet.Position(s.Pos()).Column,
		})
	case *ast.GoStmt:
		b.processGo(block, s)
	}
}

func (b *blockBuilder) processAssign(block *BasicBlock, stmt *ast.AssignStmt) {
	line := b.ssa.FileSet.Position(stmt.Pos()).Line
	col := b.ssa.FileSet.Position(stmt.Pos()).Column

	for i, lhs := range stmt.Lhs {
		if id, ok := lhs.(*ast.Ident); ok {
			if i < len(stmt.Rhs) {
				rhs := stmt.Rhs[i]
				op := "alloc"
				switch r := rhs.(type) {
				case *ast.CallExpr:
					if fun, ok := r.Fun.(*ast.Ident); ok {
						switch fun.Name {
						case "make":
							op = "alloc"
						case "len":
							op = "len"
						case "cap":
							op = "len"
						default:
							op = "call"
						}
					}
				case *ast.UnaryExpr:
					if r.Op == token.NOT {
						op = "load"
					}
				case *ast.BinaryExpr:
					op = "arith"
				case *ast.IndexExpr:
					op = "index"
				case *ast.SelectorExpr:
					op = "field"
				}

				block.Instructions = append(block.Instructions, Instruction{
					Op:      op,
					Dest:    id.Name,
					Operands: []string{exprString(rhs)},
					Line:    line,
					Col:     col,
				})
			}
		}
	}
}

func (b *blockBuilder) processExprStmt(block *BasicBlock, stmt *ast.ExprStmt) {
	line := b.ssa.FileSet.Position(stmt.Pos()).Line
	col := b.ssa.FileSet.Position(stmt.Pos()).Column

	if call, ok := stmt.X.(*ast.CallExpr); ok {
		block.Instructions = append(block.Instructions, Instruction{
			Op:      "call",
			Dest:    "_",
			Operands: []string{exprString(call)},
			Line:    line,
			Col:     col,
		})
	}
}

func (b *blockBuilder) processIf(block *BasicBlock, stmt *ast.IfStmt) {
	line := b.ssa.FileSet.Position(stmt.Pos()).Line

	// Create branch instruction
	block.Instructions = append(block.Instructions, Instruction{
		Op:      "branch",
		Dest:    "_",
		Operands: []string{exprString(stmt.Cond)},
		Line:    line,
		Col:     b.ssa.FileSet.Position(stmt.Pos()).Column,
	})

	// Then block
	thenBlock := &BasicBlock{ID: -1}
	block.Successors = append(block.Successors, thenBlock)
	thenBlock.Predecessors = append(thenBlock.Predecessors, block)
	b.fn.Blocks = append(b.fn.Blocks, thenBlock)

	if stmt.Body != nil {
		for _, s := range stmt.Body.List {
			b.processStmt(thenBlock, s)
		}
	}

	// Else block (optional)
	var elseBlock *BasicBlock
	if stmt.Else != nil {
		elseBlock = &BasicBlock{ID: -1}
		block.Successors = append(block.Successors, elseBlock)
		elseBlock.Predecessors = append(elseBlock.Predecessors, block)
		b.fn.Blocks = append(b.fn.Blocks, elseBlock)

		if bs, ok := stmt.Else.(*ast.BlockStmt); ok {
			for _, s := range bs.List {
				b.processStmt(elseBlock, s)
			}
		}
	}

	// Merge block
	mergeBlock := &BasicBlock{ID: -1}
	if thenBlock != nil {
		thenBlock.Successors = append(thenBlock.Successors, mergeBlock)
	}
	if elseBlock != nil {
		elseBlock.Successors = append(elseBlock.Successors, mergeBlock)
	}
	mergeBlock.Predecessors = append(mergeBlock.Predecessors, block)
	b.fn.Blocks = append(b.fn.Blocks, mergeBlock)
}

func (b *blockBuilder) processFor(block *BasicBlock, stmt *ast.ForStmt) {
	line := b.ssa.FileSet.Position(stmt.Pos()).Line

	block.Instructions = append(block.Instructions, Instruction{
		Op:   "branch",
		Dest: "_",
		Operands: func() []string {
			if stmt.Cond != nil {
				return []string{exprString(stmt.Cond)}
			}
			return []string{"true"}
		}(),
		Line: line,
		Col:  b.ssa.FileSet.Position(stmt.Pos()).Column,
	})

	bodyBlock := &BasicBlock{ID: -1}
	block.Successors = append(block.Successors, bodyBlock)
	bodyBlock.Predecessors = append(bodyBlock.Predecessors, block)
	b.fn.Blocks = append(b.fn.Blocks, bodyBlock)

	if stmt.Body != nil {
		for _, s := range stmt.Body.List {
			b.processStmt(bodyBlock, s)
		}
	}

	// Back edge
	bodyBlock.Successors = append(bodyBlock.Successors, block)
	block.Predecessors = append(block.Predecessors, bodyBlock)
}

func (b *blockBuilder) processRange(block *BasicBlock, stmt *ast.RangeStmt) {
	line := b.ssa.FileSet.Position(stmt.Pos()).Line

	block.Instructions = append(block.Instructions, Instruction{
		Op:      "range",
		Dest:    "_",
		Operands: []string{exprString(stmt.X)},
		Line:    line,
		Col:     b.ssa.FileSet.Position(stmt.Pos()).Column,
	})

	bodyBlock := &BasicBlock{ID: -1}
	block.Successors = append(block.Successors, bodyBlock)
	bodyBlock.Predecessors = append(bodyBlock.Predecessors, block)
	b.fn.Blocks = append(b.fn.Blocks, bodyBlock)

	if stmt.Body != nil {
		for _, s := range stmt.Body.List {
			b.processStmt(bodyBlock, s)
		}
	}

	bodyBlock.Successors = append(bodyBlock.Successors, block)
	block.Predecessors = append(block.Predecessors, bodyBlock)
}

func (b *blockBuilder) processSend(block *BasicBlock, stmt *ast.SendStmt) {
	line := b.ssa.FileSet.Position(stmt.Pos()).Line
	col := b.ssa.FileSet.Position(stmt.Pos()).Column

	chName := exprString(stmt.Chan)
	valName := exprString(stmt.Value)

	// Find or create channel
	ch := b.ssa.findOrCreateChannel(chName, line, col)

	op := ChannelOp{
		Kind:      "send",
		Block:     block,
		Line:      line,
		Col:       col,
		Value:     valName,
	}
	if block.Goroutine != nil {
		op.Goroutine = block.Goroutine
	}
	ch.Sends = append(ch.Sends, op)

	block.Instructions = append(block.Instructions, Instruction{
		Op:        "send",
		Dest:      "_",
		Operands:  []string{valName},
		Channel:   ch,
		Line:      line,
		Col:       col,
		Goroutine: block.Goroutine,
	})
}

func (b *blockBuilder) processGo(block *BasicBlock, stmt *ast.GoStmt) {
	line := b.ssa.FileSet.Position(stmt.Pos()).Line
	col := b.ssa.FileSet.Position(stmt.Pos()).Column

	var funcName string
	if call, ok := stmt.Call.Fun.(*ast.Ident); ok {
		funcName = call.Name
	} else if sel, ok := stmt.Call.Fun.(*ast.SelectorExpr); ok {
		funcName = sel.Sel.Name
	}

	g := &Goroutine{
		ID:         -1,
		FuncName:   funcName,
		SpawnLine:  line,
		SpawnCol:   col,
		ParentFunc: b.fn,
		Entry:      &BasicBlock{ID: -1},
		Exit:       &BasicBlock{ID: -1},
	}

	b.ssa.Goroutines = append(b.ssa.Goroutines, g)
	b.fn.Goroutines = append(b.fn.Goroutines, g)

	block.Instructions = append(block.Instructions, Instruction{
		Op:      "goroutine",
		Dest:    "_",
		Operands: []string{funcName},
		Line:    line,
		Col:     col,
	})
}

func (b *blockBuilder) processRecv(block *BasicBlock, stmt ast.Stmt) {
	// Handled in processAssign when RHS is a recv expression
}

func assignIDs(ssa *SSA) {
	blockID := 0
	for _, fn := range ssa.Functions {
		for _, bb := range fn.Blocks {
			if bb.ID == -1 {
				bb.ID = blockID
				blockID++
			}
		}
	}

	goroutineID := 0
	for _, g := range ssa.Goroutines {
		if g.ID == -1 {
			g.ID = goroutineID
			goroutineID++
		}
	}
}

func buildCFG(ssa *SSA) {
	// Ensure all blocks have proper predecessor/successor links
	for _, fn := range ssa.Functions {
		if len(fn.Blocks) > 0 {
			fn.Entry = fn.Blocks[0]
			fn.Exit = fn.Blocks[len(fn.Blocks)-1]
		}
	}
}

func exprString(expr ast.Expr) string {
	switch e := expr.(type) {
	case *ast.Ident:
		return e.Name
	case *ast.BasicLit:
		return e.Value
	case *ast.CallExpr:
		parts := make([]string, 0, len(e.Args))
		for _, a := range e.Args {
			parts = append(parts, exprString(a))
		}
		return fmt.Sprintf("%s(%s)", exprString(e.Fun), strings.Join(parts, ","))
	case *ast.SelectorExpr:
		return exprString(e.X) + "." + e.Sel.Name
	case *ast.BinaryExpr:
		return fmt.Sprintf("%s %s %s", exprString(e.X), e.Op, exprString(e.Y))
	case *ast.UnaryExpr:
		return fmt.Sprintf("%c%s", e.Op, exprString(e.X))
	case *ast.IndexExpr:
		return fmt.Sprintf("%s[%s]", exprString(e.X), exprString(e.Index))
	case *ast.StarExpr:
		return "*" + exprString(e.X)
	default:
		return "_expr"
	}
}

func (s *SSA) findOrCreateChannel(name string, line, col int) *Channel {
	for _, ch := range s.Channels {
		if ch.Name == name {
			return ch
		}
	}
	ch := &Channel{
		Name:    name,
		Type:    "chan",
		Line:    line,
		Col:     col,
	}
	s.Channels = append(s.Channels, ch)
	return ch
}
