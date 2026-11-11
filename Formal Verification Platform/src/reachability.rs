use rustc_middle::mir::{BasicBlock, Body, TerminatorKind};
use std::collections::HashSet;

/// Compute reachable blocks from the entry block (bb0) using a worklist algorithm.
/// Blocks not reachable from entry are dead code and should be skipped.
pub fn compute_reachable_blocks(mir: &Body<'_>) -> HashSet<BasicBlock> {
    let mut reachable = HashSet::new();
    let start = BasicBlock::from_usize(0);
    let mut worklist = vec![start];
    reachable.insert(start);

    while let Some(bb) = worklist.pop() {
        let block_data = &mir.basic_blocks[bb];
        if let Some(terminator) = &block_data.terminator {
            // Every variant, in a `match` with no wildcard arm: an edge this
            // walk does not know about used to fall through to "no successors",
            // which silently dropped everything after the block from the
            // analysis — including the assertions in it, which were then never
            // *encountered*, so the coverage figure could not notice either. An
            // `asm!` block is where that was reproduced (`tests/statement_writes`:
            // the assertion after the asm was not checked at all).
            let targets: Vec<BasicBlock> = match &terminator.kind {
                TerminatorKind::Goto { target } => vec![*target],
                TerminatorKind::SwitchInt { targets, .. } => targets.all_targets().to_vec(),
                TerminatorKind::Drop { target, .. } => vec![*target],
                TerminatorKind::Call {
                    target: Some(t), ..
                } => vec![*t],
                TerminatorKind::Assert { target, .. } => vec![*target],
                TerminatorKind::Yield { resume, .. } => vec![*resume],
                TerminatorKind::InlineAsm { targets, .. } => targets.to_vec(),
                TerminatorKind::FalseEdge { real_target, .. } => vec![*real_target],
                TerminatorKind::FalseUnwind { real_target, .. } => vec![*real_target],
                TerminatorKind::Call { target: None, .. }
                | TerminatorKind::TailCall { .. }
                | TerminatorKind::UnwindResume
                | TerminatorKind::UnwindTerminate(_)
                | TerminatorKind::Return
                | TerminatorKind::Unreachable
                | TerminatorKind::CoroutineDrop => vec![],
            };
            for t in targets {
                if reachable.insert(t) {
                    worklist.push(t);
                }
            }
        }
    }
    reachable
}

/// Check if a function name is a panic-internal function we should skip.
///
/// Previously this matched any name containing the substring "Panic", which
/// silently dropped real user functions (and trait impls) whose names merely
/// contained the word. We now match only the well-known core/std panic entry
/// points and their helpers.
pub fn is_panic_internal(name: &str) -> bool {
    name.starts_with("core::panicking::")
        || name.starts_with("std::panicking::")
        || name.starts_with("alloc::alloc::handle_alloc_error")
        || name.contains("::panic_fmt")
        || name.contains("::begin_panic")
        || name.contains("::panic_bounds_check")
        || name.contains("::panic_misaligned_pointer_dereference")
        || name.contains("::panic_nounwind")
}

#[cfg(test)]
mod tests {
    use super::is_panic_internal;

    #[test]
    fn only_the_real_panic_entry_points_are_internal() {
        // The regression this pins: the filter used to match any name containing
        // "Panic", so a user function or trait impl that merely had the word in
        // its name was dropped from the analysis — checked by nothing, and absent
        // from the report as though it had been proved.
        for name in [
            "core::panicking::panic_bounds_check",
            "std::panicking::begin_panic_handler",
            "alloc::alloc::handle_alloc_error",
            "keller::my_panic_fmt_helper::panic_fmt",
        ] {
            assert!(is_panic_internal(name), "{name} is a panic entry point");
        }
        for name in [
            "my_crate::PanicHandler::run",
            "my_crate::panic_stats",
            "my_crate::Panickers::new",
            "core::slice::index::slice_start_index_len_fail",
        ] {
            assert!(
                !is_panic_internal(name),
                "{name} is not a panic entry point and must be analysed"
            );
        }
    }
}
