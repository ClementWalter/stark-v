# Instructions

**CRITICAL**: You are responsible for making sure that the code actually matches
the [Claudeth README.md file](README.md). Everything may be wrong,
[Claudeth PLAN.md file](PLAN.md) may be complete crap, even empty. The code may
be all to be trashed. Don't assume correctness, you are the expert. You take
full responsibility for the code and the plan.

Your only focus is the claudeth project, **don't edit files elsewhere**,
**always run cargo with -p claudeth** to only work on the claudeth project.

Other relevant part are:

- [the sdk](../../crates/sdk/) to compile and run with the RISC-V target;
- [the runner](../../crates/runner/) to understand the execution workflow.

YOUR JOB NOW:

0. Read the [Claudeth README.md file](README.md).
1. Read the [Claudeth source code](src/).
2. Read the [Claudeth PLAN.md file](PLAN.md).
3. Read past [LEARNINGS.md](LEARNINGS.md) about what works and what doesn't.
4. Based on your analysis, update the PLAN.md file to reflect the current status
   and the remaining tasks to be implemented **TO TOTALLY COMPLETE THE
   PROJECT**. The PLAN.md should feature a detailed list of tasks to be
   implemented in the form Why/What/How, ordered by priority.
5. If there is nothing to be done, exit.
6. Based on the PLAN.md file, pick the FIRST task and do it. Don't work on
   several tasks, just the most priority one that can be implemented 100% NOW.
   Read the [references implementations](execution-specs) related to the task
   before starting to work on it. **Make a clear plan based on the learnings
   from the references implementations**. EVM clients are full of small details
   that are easy to miss.
7. Commit the changes to the repository.
8. Generate an updated LEARNINGS.md file with actual meaningful learnings in the
   form of do's and don'ts, and no dup.
9. Update [PROMPT.md](PROMPT.md) if and only if you want to update the current
   procedure.
10. Exit.

**CRITICAL**: Never dismiss linter errors, nothing is optional. Don't update
linter rules. You need to fix errors, not remove them.

**CRITICAL**: Ask no question. If the task is too big, break it down into
smaller tasks, update plan and exit.

**CRITICAL**: Always run test in --release mode.
