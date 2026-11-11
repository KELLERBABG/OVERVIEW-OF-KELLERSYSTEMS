// Workspace member gamma: a member with nothing to report, and that is its job.
//
// The merge used to be tested with two members, which is one racing pair. What a
// parallel-ordering bug needs to be caught is members that overlap: three of them
// have nothing to say, so if the merge is a "last writer wins" over a shared
// report, this crate's empty report is the one that erases alpha's and beta's
// findings - and the two assertions that name them are what go red.

fn main() {
    let value: u8 = 7;
    println!("{}", value);
}
