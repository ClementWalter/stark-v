# New issue

Use the gh cli to create an issue with the current repository. Describe the
issue body as Why/What/How, ie first why we need to do something, then what is
supposed to be done or fixed, then a concise to-do to actually implement the
what. Make sure to use the correct formatting and syntax for the issue content.

You will not change the current codebase nor create any code; focus on project
management and creating the issue properly.

Get the issue content based on our previous conversion if any, the current
context (any pending diff) and `$ARGUMENTS` if any. When invoking the command,
first displays the proposed issue content, and then asks you to confirm it. You
will make sure to create the issue by pushing markdown syntax, without escaping
backticks.

Assign to the issue the current github user.
