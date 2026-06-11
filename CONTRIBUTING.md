## <img alt="contributing icon" src="./assets/readme/contributing.svg" height="24" style="vertical-align: middle;"> Contributing

> [!NOTE]
> We deeply value and appreciate all contributions to this repository! Every piece of code, bug report, and suggestion helps make the tool better. Open an issue first for major changes or new features. Small fixes (typos, minor bugs) can go directly to a Pull Request.

### <img alt="documentation icon" height="18" src="./assets/readme/documentation.svg" style="vertical-align: middle;">&nbsp;&nbsp;Ground Rules

- **Keep it minimal** — the design should be quiet, fast, and functional.
- **Rust Best Practices** — use `cargo clippy -- -D warnings`.
- **Respect the style** — use custom properties from `src/cli/ui.rs`, no hardcoded colors.
- **Concurrency** — use `tokio` for blocking operations.

### <img alt="git-branch icon" height="18" src="./assets/readme/git-branch.svg" style="vertical-align: middle;">&nbsp;&nbsp;Contribution Flow

1. **Fork** the repository and clone it locally.
2. **Branch**: Create a new branch (`feat/...` or `fix/...`).
3. **Develop**: Make your changes following the code style.
4. **Validate**: Run `cargo check` and `cargo clippy` to ensure no errors.
5. **PR**: Open a Pull Request with a clear description of the change.

### <img alt="commit-style icon" height="18" src="./assets/readme/commit-style.svg" style="vertical-align: middle;">&nbsp;&nbsp;Commit Style

Keep your commit messages simple and descriptive:

- **feat**: new feature or addition.
- **fix**: bug fix or correction.
- **refactor**: code improvement (no API changes).
- **docs**: documentation updates.

### <img alt="features icon" height="18" src="./assets/readme/features.svg" style="vertical-align: middle;">&nbsp;&nbsp;PR Checklist

- [ ] Runs `cargo check` without errors.
- [ ] No `dbg!` or `println!` statements.
- [ ] Follows existing file structure and naming.
- [ ] UI changes fit within the existing Ratatui layout.
