# Changelog

All notable changes to Fang are documented here.
## [0.6.0] — 2026-03-10

### Features

- AI integration with multi-provider support (#18) ([161dfb1](https://github.com/theburrowhub/fang/commit/161dfb1a6bf39d5edb88aaa9f94fcbd9b795cd9f))


## [0.5.0] — 2026-03-10

### Features

- Tmux-aware splits — focus transfer + popup overlay (#28) ([2d9bab7](https://github.com/theburrowhub/fang/commit/2d9bab752449e2db6d9e5e2cca9067f1d59e78aa))


## [0.4.0] — 2026-03-09

### Features

- Shift+Tab cycles panel focus backward (skip hidden panels) (#27) ([91a098a](https://github.com/theburrowhub/fang/commit/91a098aafef75413b6490163be5eda4917408a6b))


## [0.3.1] — 2026-03-07

### Bug Fixes

- Markdown renderer — wrap, aligned tables, clickable links, code styling (#26) ([acf5530](https://github.com/theburrowhub/fang/commit/acf5530e21b996aa060dc0227616e7cf28c26358))


## [0.3.0] — 2026-03-07

### Features

- Sidebar/preview default visibility persisted in settings (#25) ([0eb2d60](https://github.com/theburrowhub/fang/commit/0eb2d60cf41df2f25702852dedb92434a110ab3b))


## [0.2.1] — 2026-03-06

### Bug Fixes

- Add c/C copy-path bindings to keybindings registry ([5889348](https://github.com/theburrowhub/fang/commit/588934847b8c3a70fe56a62a465f36bfd355be6d))


## [0.2.0] — 2026-03-06

### Bug Fixes

- Homebrew tap step is non-fatal if HOMEBREW_TAP_TOKEN not set ([23f17a9](https://github.com/theburrowhub/fang/commit/23f17a9346e6ff0183eda6e095c38fa6b4f2e901))

- Clip long preview lines at panel boundary to prevent overflow ([76bf38d](https://github.com/theburrowhub/fang/commit/76bf38d9a2390cbc97b6d931a9f32b765f3a56ee))

- Ctrl+S opens settings instead of toggling sidebar ([bab6ac9](https://github.com/theburrowhub/fang/commit/bab6ac99f33c9386b108e9bb55d0519af95b1795))

- Remove unused imports in settings_modal.rs ([8aa8296](https://github.com/theburrowhub/fang/commit/8aa829621d546f809b03ad538c2ce9b41a179ae4))

- Consolidate git first screen — Fetch/Pull/Push/Stash get forms ([258761f](https://github.com/theburrowhub/fang/commit/258761ffdf0e4ed17aa2be7ed61fa16e57f7c514))

- Log/Diff/Branches also get forms — only Status and Stash pop are direct ([b8cdee3](https://github.com/theburrowhub/fang/commit/b8cdee33bf3e3d55298e7e9cdaf3d47b2bf41223))

- Stash pop merged into Stash form as SubCmd checkbox ([9566f9b](https://github.com/theburrowhub/fang/commit/9566f9bc32b89dccf2f44d6035e67dad2c7810b9))

- Ci-cd.yml — remove invalid secrets.* from step if: condition ([e6c5db3](https://github.com/theburrowhub/fang/commit/e6c5db3fc8ec56523fe48e40e393c4103b0f940d))


### Features

- Render Markdown files with formatting in preview panel (#19) ([e50529b](https://github.com/theburrowhub/fang/commit/e50529b74ea45fc7ef04eeaed8b3cd88b7b92bc3))

- Persistent settings system with in-app editor (Ctrl+S) ([cff13bc](https://github.com/theburrowhub/fang/commit/cff13bc7a38ecbb9bb98bdbf47b7975e2b47acbe))

- H=Help panel, u=parent dir, dual-label footer (#21) ([d3344e7](https://github.com/theburrowhub/fang/commit/d3344e7b18bb99381c4e6972556def43e68e7f83))

- Git menu 2-screen — form for parametrized operations ([8c9a708](https://github.com/theburrowhub/fang/commit/8c9a708769b76eb4a625b4e30226a052fdca303a))

- C=copy relative path, C=copy absolute path to clipboard (#23) ([b291bb9](https://github.com/theburrowhub/fang/commit/b291bb91a4f8f99ec5548ac1a8bb8466c0266724))

- Git file status indicators in file list (#24) ([74c5ca9](https://github.com/theburrowhub/fang/commit/74c5ca925c4b2e60e119570922831f76f5d33e5e))


## [0.1.1] — 2026-03-06

### Bug Fixes

- Use annotated tag and explicit tag push in release-prep ([6b94697](https://github.com/theburrowhub/fang/commit/6b9469779d61d9689cab399578da5474079428d6))


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



