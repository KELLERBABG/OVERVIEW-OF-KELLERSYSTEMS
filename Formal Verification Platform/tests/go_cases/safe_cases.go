package main

type Node struct {
	val  int
	next *Node
}

func safeDeref(p *int) int {
	if p != nil {
		return *p
	}
	return 0
}

func safeFieldAccess(n *Node) int {
	if n == nil {
		return 0
	}
	return n.val
}

func safeSliceRange(s []int) int {
	sum := 0
	for i := range s {
		sum += s[i]
	}
	return sum
}

func safeSliceBoundedLoop(s []int) int {
	sum := 0
	for i := 0; i < len(s); i++ {
		sum += s[i]
	}
	return sum
}

func safeSliceGuardedIf(s []int, i int) int {
	if i < len(s) {
		return s[i]
	}
	return -1
}

func safeDivGuarded(a int, b int) int {
	if b != 0 {
		return a / b
	}
	return 0
}

func safeDivConstant(x int) int {
	return x / 2
}

func safeDataRaceParam(ch chan int) {
	for i := 0; i < 5; i++ {
		go func(val int) {
			ch <- val
		}(i)
	}
}

func safeBufferedChannel() int {
	ch := make(chan int, 1)
	ch <- 42
	return <-ch
}

func safeSelectDefault(ch chan int) {
	select {
	case ch <- 1:
	default:
	}
}
