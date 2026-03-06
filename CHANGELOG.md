# Changelog

All notable changes to Fang are documented here.
## [0.1.0] — 2026-03-06

### Bug Fixes

- Make not found, scattered preview chars, preview scroll with focus ([a6c1de9](https://github.com/theburrowhub/fang/commit/a6c1de9a843ba921b084d2423c11f0cbfd2468bc))

- Scattered preview chars — use terminal.clear() on navigation ([96085b3](https://github.com/theburrowhub/fang/commit/96085b36b572246493cabbb2514c13c58e4a0b76))

- Erase stale preview cells by explicit blank-fill on every render ([eaedaee](https://github.com/theburrowhub/fang/commit/eaedaee5ce4f55791e3ec06e12fd444b8cc095e4))

- Remove path from footer line 2; fix command shell suspension ([d23d577](https://github.com/theburrowhub/fang/commit/d23d57768ddf7883cd38e133a29a55305a6caaf1))

- Use setsid() in child process to prevent interactive shell from stealing TTY ([45bdaac](https://github.com/theburrowhub/fang/commit/45bdaac042923c03c3e29c76251f8cca5f8e28d1))

- Remove 'make completed' message from : commands; footer to 1 line ([b0c2737](https://github.com/theburrowhub/fang/commit/b0c2737100c0b6d85afc20b8ef6dee91117e287a))

- ; split now vertical (left|right); aliases work; iTerm2 executes command ([d88a52f](https://github.com/theburrowhub/fang/commit/d88a52f973e6bd2541be738c01a4b774d3b90e40))

- Clipboard reads image data (PNG/TIFF/JPEG) from macOS clipboard ([e530a67](https://github.com/theburrowhub/fang/commit/e530a67793d36fb932d4086c3b639bfecbd6efff))

- Find git binary in common paths when not in PATH ([4f6d51a](https://github.com/theburrowhub/fang/commit/4f6d51a49908b322308ecb65771f3fb512705c05))

- Cargo fmt + ensure ci-cd.yml is the only workflow ([074b806](https://github.com/theburrowhub/fang/commit/074b80622f04fab9672e162b8c712ffe71372064))

- Resolve all clippy -D warnings and fmt issues ([113ebfe](https://github.com/theburrowhub/fang/commit/113ebfe444fe822f8fba495659b7429f5188e1fa))

- Gate unix-only code with #[cfg(unix)] for Windows CI ([c68e87f](https://github.com/theburrowhub/fang/commit/c68e87fd2a9c5ecae15e3af10eb9867ed1e677bc))

- Install git-cliff from GitHub releases instead of Docker action ([8e37e57](https://github.com/theburrowhub/fang/commit/8e37e57c5f5e2d4a3566fcf88696594f273863c9))

- Release-prep always fires on first release (no tags) ([e282d92](https://github.com/theburrowhub/fang/commit/e282d92f9524b88d501badba197aedf0836cb5d3))


### Features

- U0 - project scaffolding and stub module structure (#1) ([d8e4f14](https://github.com/theburrowhub/fang/commit/d8e4f14f0d6f4f1aabde64ae2f4e16965f77d42e))

- U7 - Full integration — all modules wired into working TUI ([e225c27](https://github.com/theburrowhub/fang/commit/e225c27f2084da6a6b99b723aaf337cf19f349c2))

- Makefile, GitHub Actions CI/release, docs landing page, homebrew formula ([fee436c](https://github.com/theburrowhub/fang/commit/fee436cda97b8bbba1413a8d19adc523460b79d5))

- Merge PRs #10-#13 — sidebar dirname, layout, header bar, command input ([9bf24dc](https://github.com/theburrowhub/fang/commit/9bf24dce1787be705543fa3678af37c00c0e3b0b))

- Relay keystrokes to stdin of running : commands ([33d96b4](https://github.com/theburrowhub/fang/commit/33d96b461c45ac3f42f90e54909cdf3d9dcc28ff))

- ; opens command in new terminal split (zellij/tmux/kitty/wezterm/ghostty/iTerm2/...) ([042582b](https://github.com/theburrowhub/fang/commit/042582b646f8027f90f111189774c4255caffada))

- Git menu (g), window title, open with system (o), new file (n/N) ([41ce286](https://github.com/theburrowhub/fang/commit/41ce286e53f4bf74033a296ee330e45c126aba88))



