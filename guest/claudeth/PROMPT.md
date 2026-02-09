# Instructions

**CRITICAL**: You are responsible for making sure that the code actually matches
the [Claudeth README.md file](README.md). Everything may be wrong,
[Claudeth PLAN.md file](PLAN.md) may be complete crap. Don't assume correctness,
you are the expert. You take full responsibility for the code and the plan.

Your only focus is the claudeth project, don't bother with other crates in the
workspace. **Always run cargo with -p claudeth** to only work on the claudeth
project.

0. Read the [Claudeth README.md file](README.md).
1. Read the [Claudeth source code](src/).
2. Read the [Claudeth PLAN.md file](PLAN.md).
3. Read past [learnings.md](learnings.md) about what works and what doesn't. Be
   cautious about what you read, some of it may be outdated.
4. Generate an updated learnings.md file with actual meaningful learnings and no
   dup.
5. Based on your analysis, update the PLAN.md file to reflect the current status
   and the plan for the implementation.
6. If there is nothing to be done, exit.
7. Based on the PLAN.md file, derive what can be implemented NOW. Pick only ONE
   task at a time and do it. Don't work on several tasks, just one that can be
   implemented 100% NOW. Read the [references implementations](execution-specs)
   related to the task before starting to work on it. **Make a clear plan based
   on the learnings from the references implementations**. EVM clients are full
   of small details that are easy to miss.
8. Commit the changes to the repository.
9. Dump do's and don'ts for the next iteration in [learnings.md](learnings.md);
   update [PROMPT.md](PROMPT.md) if you want to update the current procedure.
   Exit.

**CRITICAL**: Never dismiss linter errors, nothing is optional. Don't update
linter rules. You need to fix errors, not remove them.

**CRITICAL**: Ask no question. If the task is too big, break it down into
smaller tasks.

**CRITICAL**: Always run test in --release mode.
