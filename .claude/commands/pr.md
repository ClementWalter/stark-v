# PR workflow

Use the [commit](./commit.md) command if there are any pending changes, possibly
related to untracked files. Untracked files should be committed, and not
ignored.

If you find a bunch of unrelated changes, feel free to make several commits.

Once the git tree is clean, do a `git pull --rebase origin main` to make sure to
observe only the latest changes against the main branch. The branch itself may
have been used for other PRs so don't use the whole branch history but only the
diff between origin/main and the rebased current branch.

Use the git and gh cli tools to fetch the diff between origin/main and the
rebased current branch. Generate a concise summary of the content and purpose of
these changes based on the observed diff.

IMPORTANT: Do not add yourself as co-author in commit messages, nor add any
"Generated with Claude Code" footer or any other mention of AI assistance in
commits or PR descriptions. All work should be attributed solely to the user.

If some $ARGUMENTS are given, add to the summary "Close $ARGUMENTS". Otherwise,
you MUST use the [new-issue](./new-issue.md) command to create an issue. This is
mandatory - do NOT create issues in the current repository.

If there is already an open PR for the current branch, update it instead of
creating a new one.
