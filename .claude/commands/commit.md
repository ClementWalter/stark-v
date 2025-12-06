# Smart Git Commit

I'll analyze your changes and create a meaningful commit message.

First, make sure the linter pass by running `trunk check --ci --fix`. Fix any
error until the linter pass.

**CRITICAL**: Check branch before committing. Never ever commit to the main
branch. Use

```bash
git rev-parse --abbrev-ref HEAD
```

to get the current branch name. If this is `main`, start by creating a new
branch following these steps:

1. Pick a relevant name based on current changes
2. Create the new branch: `git checkout -b <branch-name>`
3. Then proceed with the commit

First, let me check what's changed:

```bash
# Check if we have changes to commit
if ! git diff --cached --quiet || ! git diff --quiet; then
    echo "Changes detected:"
    git status --short
else
    echo "No changes to commit"
    exit 0
fi

# Show detailed changes
git diff --cached --stat
git diff --stat
```

I will not forget to add all untracked files and review their content afterwards
to update the commit message.

Now I'll analyze the changes to determine:

1. What files were modified
2. The nature of changes (feature, fix, refactor, etc.)
3. The scope/component affected

If the analysis or commit encounters errors:

- I'll explain what went wrong
- Suggest how to resolve it
- Ensure no partial commits occur

```bash
# If nothing is staged, I'll stage modified files (not untracked)
if git diff --cached --quiet; then
    echo "No files staged. Staging modified files..."
    git add -u
fi

# Show what will be committed
git diff --cached --name-status
```

Based on the analysis, I'll create a conventional commit message:

- **Type**: feat|fix|docs|style|refactor|test|chore
- **Scope**: component or area affected
- **Subject**: clear description in present tense
- **Body**: why the change was made (if needed)

I will not add myself as co-author of the commit, nor add any "Generated with
Claude Code" footer or any other mention of AI assistance. The commit should be
entirely attributed to you.

Before committing, I'll run the `trunk check --ci --fix` command to ensure that
the code is formatted correctly. If there are any errors, I'll ask the you to
fix them manually.

The commit message will be concise, meaningful, and follow your project's
conventions if I can detect them from recent commits.
