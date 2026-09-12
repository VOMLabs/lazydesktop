---
description: Create well-formatted commits with conventional commit messages
---

# Commit Command

You are an AI agent that helps create well-formatted git commits with conventional commit messages, follow these instructions exactly. Always run and push the commit, you don't need to ask for confirmation unless there is a big issue or error.

## Instructions for Agent

When the user runs this command, execute the following workflow:

1. **Check command mode**:
   - If user provides $ARGUMENTS (a simple message), skip to step 3

2. **Run pre-commit validation**:
   - Execute `pnpm lint` and report any issues
   - Execute `pnpm build` and ensure it succeeds
   - If either fails, ask user if they want to proceed anyway or fix issues first
   
3. **Analyze git status**:
   - Run `git status --porcelain` to check for changes
   - If no files are staged, run `git add .` to stage all modified files
   - If files are already staged, proceed with only those files
   
4. **Analyze the changes**:
   - Run `git diff --cached` to see what will be committed
   - Analyze the diff to determine the primary change type (feat, fix, docs, etc.)
   - Identify the main scope and purpose of the changes
   - Decide whether the changes are big or small (see "Commit Splitting" below)
   
5. **Generate commit message(s)**:
   - Choose the appropriate type and scope from the reference below
   - Create message following format: `<type>(<scope>): <description>`
   - Keep description concise, clear, and in imperative mood
   - **Do not use emojis** in commit messages
   - Show the proposed message(s) to user for confirmation
   
6. **Execute the commit(s)**:
   - Run `git commit -m "<generated message>"` for each logical commit
   - Display the commit hash and confirm success
   - Provide brief summary of what was committed

## Commit Splitting

- **Big changes**: Always split the work into multiple logical commits, one per concern (e.g., a refactor commit, a feature commit, a build commit). Each commit should be self-contained and buildable.
- **Small changes**: If only small changes were made that serve a single purpose, use just 1 commit.
- **Atomic commits**: Each commit should contain related changes that serve a single purpose

## Commit Message Guidelines

When generating commit messages, follow these rules:

- **No emojis**: Never prefix commit messages with emoji icons
- **Atomic commits**: Each commit should contain related changes that serve a single purpose
- **Imperative mood**: Write as commands (e.g., "add feature" not "added feature")
- **Concise first line**: Keep under 72 characters
- **Conventional format**: Use `<type>(<scope>): <description>` where type is one of:
  - `feat`: A new feature
  - `fix`: A bug fix
  - `docs`: Documentation changes
  - `style`: Code style changes (formatting, etc.)
  - `refactor`: Code changes that neither fix bugs nor add features
  - `perf`: Performance improvements
  - `test`: Adding or fixing tests
  - `build`: Changes to the build process, tools, etc.
  - `ci`: CI/CD improvements
  - `chore`: Changes to the build process, tools, etc.
  - `revert`: Reverting changes
- **Scope**: The module or component (e.g., `vcs`, `ai-core`, `settings`). Omit when it adds no value.
- **Present tense, imperative mood**: Write commit messages as commands (e.g., "add feature" not "added feature")
- **Concise first line**: Keep the first line under 72 characters

## Reference: Good Commit Examples

Use these as examples when generating commit messages:
- feat(app): add user authentication system
- fix(renderer): resolve memory leak in rendering process
- docs(api): update API documentation with new endpoints
- refactor(parser): simplify error handling logic
- fix(components): resolve linter warnings in component files
- chore(tooling): improve developer tooling setup process
- feat(vcs): implement business logic for transaction validation
- fix(ui): address minor styling inconsistency in header
- fix(auth): patch critical security vulnerability in auth flow
- style(components): reorganize component structure for better readability
- fix(legacy): remove deprecated legacy code
- feat(validation): add input validation for user registration form
- fix(ci): resolve failing CI pipeline tests
- feat(analytics): implement analytics tracking for user engagement
- fix(auth): strengthen authentication password requirements
- feat(accessibility): improve form accessibility for screen readers

Example commit sequence (big change, multiple logical commits):
- refactor(config): migrate YAML persistence to Lua
- feat(app): add GPUI frontend crate
- build: unify GPUI dependencies

Example commit sequence (small change, single commit):
- fix(ui): correct button alignment in toolbar

## Agent Behavior Notes

- **Error handling**: If validation fails, give user option to proceed or fix issues first  
- **Auto-staging**: If no files are staged, automatically stage all changes with `git add .`
- **File priority**: If files are already staged, only commit those specific files
- **Always run and push the commit**: You don't need to ask for confirmation unless there is a big issue or error `git push`.
- **Message quality**: Ensure commit messages are clear, concise, and follow conventional format
- **Success feedback**: After successful commit, show commit hash and brief summary