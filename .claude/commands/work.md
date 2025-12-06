# Work Command

Autonomous issue management and PR workflow for the repository.

## Arguments

- `$ARGUMENTS` - Optional: A pull request URL to review and create issues from

## Instructions

You are an autonomous agent that manages GitHub issues and pull requests. Follow
this workflow:

### Pre-flight: Discover CI Requirements

**CRITICAL**: Before any work, check the CI configuration to know what checks
must pass:

```bash
ls .github/workflows/
cat .github/workflows/*.yml
```

Common CI checks to run locally before pushing:

- `trunk check --ci` - Linting and formatting
- `cargo build` - Rust compilation
- `cargo test` - Rust tests
- `cargo clippy` - Rust lints

**NEVER push code that would fail CI. Always run CI checks locally first.**

### Mode 1: PR Review (when argument provided)

If `$ARGUMENTS` contains a PR URL:

1. **Fetch the PR** using `gh pr view <url> --json body,comments,reviews,diff`
2. **Analyze the PR** for code quality issues, improvements, and suggestions
3. **Create consolidated issues** - Group related comments into meaningful
   issues that:
   - Address multiple related concerns in a single issue
   - Minimize potential merge conflicts (group by file/module)
   - Are actionable and well-scoped
   - Include clear "Why", "What", and "How" sections
4. **Proceed to Mode 2** to work on the created issues

### Mode 2: Issue Resolution (default, or after Mode 1)

1. **Fetch open unassigned issues**:

   ```bash
   gh issue list --repo <repo> --state open --json number,title,assignees,body --jq '.[] | select(.assignees | length == 0)'
   ```

2. **Get current GitHub user**:

   ```bash
   gh api user --jq '.login'
   ```

3. **Assign all issues to yourself** in parallel:

   ```bash
   gh issue edit <number> --repo <repo> --add-assignee <username>
   ```

4. **For each issue, in parallel**:
   - Create a git worktree at `/tmp/<repo>-issue-<number>` with branch
     `fix/issue-<number>`
   - Implement the fix
   - Run ALL CI checks locally (see Pre-flight section)
   - Fix any CI failures before committing
   - Commit with message referencing the issue (`Fixes #<number>`)
   - Push and create PR using `gh pr create`

5. **Monitor and report** PR URLs when complete

### Mode 3: Rebase Loop

After initial PRs are created, or when user says "rebase":

1. **Fetch open PRs**:

   ```bash
   gh pr list --repo <repo> --state open --json number,title,headRefName
   ```

2. **Check for review comments**:

   ```bash
   gh api repos/<owner>/<repo>/pulls/<number>/comments
   ```

3. **For each PR, in parallel**:
   - Go to its worktree
   - `git fetch origin main && git rebase origin/main`
   - Resolve any conflicts
   - Address any review comments
   - Run ALL CI checks locally (see Pre-flight section)
   - Fix any issues and amend commit
   - Force push

4. **Report status** for all PRs

### Cleanup

When all PRs are merged:

- Remove all worktrees: `git worktree remove /tmp/<repo>-issue-<number> --force`
- Verify with `git worktree list`
- Update main: `git checkout main && git pull origin main`

## Key Principles

1. **CI First**: Always check `.github/workflows/` and run CI locally before
   pushing
2. **Parallelism**: Use parallel Task agents for independent work
3. **Minimize conflicts**: Group related changes, work on separate files when
   possible
4. **Quality gates**: NEVER push code that fails CI checks
5. **Clear communication**: Report PR URLs and status clearly
6. **Clean state**: Always clean up worktrees when done

## Repository Detection

Detect the repository from:

```bash
gh repo view --json nameWithOwner --jq '.nameWithOwner'
```
