package main

type Item struct {
	score int
}

func vulnNilDeref(p *int) int {
	return *p // go-nil-deref: unguarded dereference
}

func vulnExplicitNil() int {
	var p *int = nil
	return *p // go-nil-deref: dereference of nil pointer
}

func vulnFieldNil(item *Item) int {
	return item.score // go-nil-deref: unguarded field access
}

func vulnSliceOob(s []int, i int) int {
	return s[i] // go-slice-oob: index without bounds check
}

func vulnNegativeSliceIndex(s []int) int {
	return s[-1] // go-slice-oob: negative index
}

func vulnDivByZeroConstant(x int) int {
	return x / 0 // go-div-by-zero: division by constant zero
}

func vulnDivByZeroVar(a int, b int) int {
	return a / b // go-div-by-zero: division by unverified divisor
}

func vulnModByZero(a int) int {
	return a % 0 // go-div-by-zero: modulo by constant zero
}

func vulnDataRaceLoopCapture() {
	for i := 0; i < 5; i++ {
		go func() {
			println(i) // go-data-race: captures loop variable by reference
		}()
	}
}

func vulnDeadlockUnbufferedSingle() int {
	ch := make(chan int)
	ch <- 42 // go-channel-deadlock: send on unbuffered channel in single goroutine
	return <-ch
}

func vulnSendClosed() {
	ch := make(chan int, 10)
	close(ch)
	ch <- 10 // go-channel-deadlock: send on closed channel
}
