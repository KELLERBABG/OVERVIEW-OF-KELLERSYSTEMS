# Code of Conduct

## The short version

Be decent to the people you are working with. Argue about the code, the evidence
and the claims, not about the person who wrote them. This project's whole subject
is *being able to say why you believe something*, so a discussion that ends with
"here is the run, here is the obligation, here is what `z3 -smt2` answered" is the
normal shape of one, and a claim nobody can check is where the trouble starts.

## What is expected

- **Evidence over assertion.** "The analysis is wrong here" is a claim; the
  `report.json`, the emitted obligation and the re-discharge output are the
  argument. Ask for them, and provide them.
- **No dismissal of a report because it is inconvenient.** A false alarm is still a
  bug in a tool that people gate builds on. It is reported as a correctness bug
  rather than a security one ([SECURITY.md](SECURITY.md) explains the difference),
  and neither kind is answered with "works as intended" alone.
- **Patience with what is deliberately unimplemented.** Finding the tool's limits
  is useful; demanding a guarantee the tool never made is not.
- **Credit where the work is.** Say who wrote the patch, the measurement or the
  counterexample, and do not restate someone else's reasoning as your own.

Unacceptable, and unrelated to how good the code is: harassment, personal attacks,
demeaning comments, deliberate misgendering, sexual attention, publishing private
information, and sustained disruption of a discussion after being asked to stop.

## Scope

This applies to the repository's issues, pull requests, review comments, commit
messages and any other space where someone is participating in this project.

## Reporting and enforcement

Report a problem through the repository's issue tracker, or privately if the report
itself would expose someone — a report about a person is not something to put in a
public thread, and the same care applies here as the tool asks for in
[SECURITY.md](SECURITY.md): say what happened, with specifics, and who it involved.

The maintainer will read it, say what they understood, and act on it: a warning, a
request to stop interacting on a thread, or removal from the project, depending on
severity and repetition. The reporter is told the outcome unless doing so would
identify them against their wishes. Maintainers hold themselves to this document
too — a report about a maintainer goes to the same place and is not answered by the
person it is about.
