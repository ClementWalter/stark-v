# Instructions

**CRITICAL**: You are responsible for making sure that the code actually matches
the [Claudeth README.md file](README.md). Everything may be wrong,
[Claudeth PLAN.md file](PLAN.md) may be complete crap. Don't assume correctness,
you are the expert. You take full responsibility for the code and the plan.

Your only focus is the claudeth project, don't bother with other crates in the
workspace. **Always run cargo with -p claudeth** to only work on the claudeth
project. Always run tests with --release mode.

1. Read the [Claudeth README.md file](README.md).
2. Read the [Claudeth source code](src/).
3. Read the [Claudeth PLAN.md file](PLAN.md).
4. Based on your analysis, update the PLAN.md file to reflect the current status
   and the plan for the implementation.
5. If there is nothing to be done, exit.
6. Based on the PLAN.md file, derive what can be implemented in parallel NOW.
   Spawn as many rust experts sub agents as needed to work on these identified
   tasks ONLY. Don't work on sequential tasks, just on what can be implemented
   in parallel NOW.
7. The experts NEED to have 100% test coverage on their work. You are
   responsible for a final validation of the whole work of the team.
8. If everything looks good, commit the changes to the repository.
9. Else, send feedback to the experts and prompt new team.
10. Go back to step 4.

**CRITICAL**: Never dismiss linter errors, nothing is optional. Don't update
linter rules. You need to fix errors, not remove them.

**CRITICAL**: Ask no question. If the task is too big, break it down into
smaller tasks.

**CRITICAL**: Always run test in --release mode.

**CRITICAL**: Write down in `learnings.md` do's and don'ts for the next
iteration.
