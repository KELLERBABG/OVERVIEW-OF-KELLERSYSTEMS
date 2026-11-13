package ssago

import (
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"strings"
)

// loadPackage loads all Go files from a package path.
func loadPackage(fset *token.FileSet, pkgPath string) ([]*ast.File, error) {
	// Check if it's a directory
	info, err := os.Stat(pkgPath)
	if err != nil {
		return nil, fmt.Errorf("stat %q: %w", pkgPath, err)
	}

	var files []*ast.File

	if info.IsDir() {
		entries, err := os.ReadDir(pkgPath)
		if err != nil {
			return nil, fmt.Errorf("reading dir %q: %w", pkgPath, err)
		}
		for _, entry := range entries {
			if entry.IsDir() {
				continue
			}
			name := entry.Name()
			if !strings.HasSuffix(name, ".go") {
				continue
			}
			if strings.HasSuffix(name, "_test.go") {
				continue // Skip test files in audit mode
			}
			fullPath := filepath.Join(pkgPath, name)
			f, err := parser.ParseFile(fset, fullPath, nil, 0)
			if err != nil {
				return nil, fmt.Errorf("parsing %s: %w", fullPath, err)
			}
			files = append(files, f)
		}
	} else {
		f, err := parser.ParseFile(fset, pkgPath, nil, 0)
		if err != nil {
			return nil, fmt.Errorf("parsing %s: %w", pkgPath, err)
		}
		files = append(files, f)
	}

	if len(files) == 0 {
		return nil, fmt.Errorf("no Go files found in %q", pkgPath)
	}

	return files, nil
}
