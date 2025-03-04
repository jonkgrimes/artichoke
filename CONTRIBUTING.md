# Contributing to Artichoke

👋 Hi and welcome to [Artichoke]. Thanks for taking the time to contribute!
💪💎🙌

Artichoke aspires to be a Ruby 2.6.3-compatible implementation of the Ruby
programming language. [There is lots to do].

If Artichoke does not run Ruby source code in the same way that MRI does, it is
a bug and we would appreciate if you [filed an issue so we can fix it].

If you would like to contribute code 👩‍💻👨‍💻, find an issue that looks interesting
and leave a comment that you're beginning to investigate. If there is no issue,
please file one before beginning to work on a PR. [Good first issues are labeled
`E-easy`].

## Discussion

If you'd like to engage in a discussion outside of GitHub, you can [join
Artichoke's public Discord server].

## Implementation Philosophy

- Prefer pure Ruby implementations when initially implementing features.
- A feature is not done until it passes [ruby/spec](RUBYSPEC.md).
- Move implementations to Rust for performance, e.g. [using Serde to implement
  the JSON package].
- If there is a Rust crate that does what we need, prefer to use it. Forking is
  OK, too, e.g. [artichoke/rust-onig].

## Setup

Artichoke includes Rust, Ruby, C, and Text sources. Developing on Artichoke
requires configuring several dependencies.

### Rust Toolchain

Artichoke depends on Rust and several compiler plugins for linting and
formatting. The specific version of Rust that Artichoke requires is specified in
the [toolchain file](rust-toolchain).

Toolchain requirements are documented in [`BUILD.md`](BUILD.md#rust-toolchain).

### C Toolchain

Some Artichoke dependencies, like the mruby [`sys`] module and the [`onig`]
crate, build C static libraries and require a C compiler.

Toolchain requirements are documented in [`BUILD.md`](BUILD.md#c-toolchain).

### Ruby

Artichoke requires a recent Ruby 2.x and [bundler] 2.x. The
[`.ruby-version`](.ruby-version) file in this repository specifies Ruby 2.6.3.

Toolchain requirements are documented in [`BUILD.md`](BUILD.md#ruby-toolchain).

Artichoke uses [`rake`](Rakefile) as a task runner. You can see the available
tasks by running:

```console
$ bundle exec rake --tasks
rake build                        # Build Rust workspace
rake build:all                    # Build Rust workspace and sub-workspaces
rake doc                          # Generate Rust API documentation
rake doc:open                     # Generate Rust API documentation and open it in a web browser
rake fmt                          # Format sources
rake fmt:c                        # Format .c and .h sources with clang-format
rake fmt:rust                     # Format Rust sources with rustfmt
rake fmt:text                     # Format text, YAML, and Markdown sources with prettier
rake format                       # Format sources
rake format:c                     # Format .c and .h sources with clang-format
rake format:rust                  # Format Rust sources with rustfmt
rake format:text                  # Format text, YAML, and Markdown sources with prettier
rake lint                         # Lint sources
rake lint:clippy                  # Lint Rust sources with Clippy
rake lint:clippy:restriction      # Lint Rust sources with Clippy restriction pass (unenforced lints)
rake lint:rubocop                 # Run RuboCop
rake lint:rubocop:auto_correct    # Auto-correct RuboCop offenses
rake release:markdown_link_check  # Check for broken links in markdown files
rake spec                         # Run enforced ruby/spec suite
rake test                         # Run Artichoke unit tests
```

To lint Ruby sources, Artichoke uses [RuboCop]. RuboCop runs as part of the
`lint` task. To run RuboCop by itself, invoke the `lint:rubocop` task.

```console
$ bundle exec rake lint
$ bundle exec rake lint:rubocop
```

### Node.js

Node.js is an optional dependency that is used for formatting text sources with
[prettier].

Node.js is only required for formatting if modifying the following filetypes:

- `c`
- `h`
- `html`
- `json`
- `md`
- `toml`
- `yaml`
- `yml`

You will need to install [Node.js]. On macOS, you can install Node.js with
[Homebrew]:

```sh
brew install node
```

## Code Quality

### Linting

Once you [configure a development environment](#setup), run the following to
lint and format sources:

```sh
rake
```

Merges will be blocked by CI if there are lint errors.

### Testing

A PR must have new or existing tests for it to be merged. The [Rust book chapter
on testing] is a good place to start. If you'd like to see some examples in
Artichoke, take a look at the `Value` tests in
[`artichoke-backend/src/value/mod.rs`].

To run tests:

```sh
rake test
```

If you are only working on one crate, it can speed up iteration time to only
build and run tests for that crate:

```sh
cargo test -p artichoke-backend
```

`cargo test` accepts a filter argument that will limit test execution to tests
that substring match. For example, to run all of the Ruby/Rust interop tests:

```sh
cargo test -p artichoke-backend convert
```

Tests are run for every PR. All builds must pass before merging a PR.

## Updating Dependencies

### Rust Toolchain

Upgrades to the Rust toolchain should happen in a dedicated PR that addresses
any changes to ructc warnings and clippy lints. See [artichoke/artichoke#482]
for an example.

### Rust Crates

Version specifiers in `Cargo.toml` are NPM caret-style by default. A version
specifier of `4.1.2` means `4.1.2 <= version < 5.0.0`.

To see what crates are outdated, you can use [cargo-outdated].

If you need to pull in an updated version of a crate for a bugfix or a new
feature, update the version number in `Cargo.toml`. See
[artichoke/artichoke#548] for an example.

Regular dependency bumps are handled by [@dependabot].

[artichoke]: https://github.com/artichoke
[there is lots to do]: https://github.com/artichoke/artichoke/issues
[filed an issue so we can fix it]:
  https://github.com/artichoke/artichoke/issues/new
[good first issues are labeled `e-easy`]:
  https://github.com/artichoke/artichoke/labels/E-easy
[join artichoke's public discord server]: https://discord.gg/QCe2tp2
[using serde to implement the json package]:
  https://github.com/artichoke/artichoke/issues/77
[artichoke/rust-onig]:
  https://github.com/artichoke/rust-onig/tree/artichoke-vendor
[`sys`]: artichoke-backend/src/sys
[`onig`]: https://crates.io/crates/onig
[bundler]: https://bundler.io/
[rubocop]: https://github.com/rubocop-hq/rubocop
[prettier]: https://prettier.io/
[node.js]: https://nodejs.org/en/download/package-manager/
[homebrew]: https://docs.brew.sh/Installation
[rust book chapter on testing]:
  https://doc.rust-lang.org/book/ch11-00-testing.html
[`artichoke-backend/src/value/mod.rs`]: artichoke-backend/src/value.rs
[artichoke/artichoke#482]: https://github.com/artichoke/artichoke/pull/482
[cargo-outdated]: https://github.com/kbknapp/cargo-outdated
[artichoke/artichoke#548]: https://github.com/artichoke/artichoke/pull/548
[@dependabot]: https://dependabot.com/
