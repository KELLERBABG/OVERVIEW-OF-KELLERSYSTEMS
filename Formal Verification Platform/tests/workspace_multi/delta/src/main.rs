// Workspace member delta: one assertion, proved.
//
// A member is not only a list of findings. This one contributes a proof and an
// obligation file, so the merge has to add up coverage and evidence across members
// that agree on nothing, and the per-crate obligation directory is what keeps this
// crate's proof from landing on top of alpha's counterexample.

fn bounded(x: u8) -> u8 {
    if x < 250 {
        x + 1
    } else {
        0
    }
}

fn main() {
    println!("{}", bounded(7));
}
