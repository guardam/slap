# slap

![Batman slapping Robin meme](https://raw.githubusercontent.com/agnipau/slap/screenshots/batman-slapping-robin.jpg)

slap (shell [`clap`][clap]) - painless argument parsing and dependency check.

## Why?

Writing code to parse arguments in a shell scripting language (`bash`, `zsh`,
`fish` etc...) is an extremly verbose, repetitive, error prone, and painful
process.  
This program solves that.

## How?

You declare your CLI in YAML and pass it to slap's `stdin` and pass all your
script's arguments to slap as arguments.  
slap makes sure that the arguments you pass to it conform to your YAML
description, and if not, it exits with an error code and outputs useful error
messages to `stderr`.  
In other words slap handles the argument parsing logic and validation, your
script only evalutes the code exported by slap and uses the parsed arguments.  
Here is an example bash script:

```bash
config="path to your YAML config"
eval "$(slap parse bash -- "$@" <"$config")"
```

The `slap-parse` subcommand, if the passed arguments conform to the YAML
description, outputs code in the language specified, so you can evaluate it to
have access to the variables containing the parsed arguments.  
Relax, slap writes to `stdout` ONLY if the YAML config is valid and the
arguments passed conform to it, otherwise it doesn't.

## Installation

If you're an **Arch Linux** user, you can install slap from the [AUR](https://aur.archlinux.org/packages/slap-cli-bin/):

```bash
# This will install a binary named `slap`.
yay -S slap-cli-bin
```

If you're a **Rust programmer**, you can install slap with `cargo`.
Make sure to add `~/.cargo/bin` to your `$PATH`.

```bash
# This will install a binary named `slap`.
cargo install slap-cli
```

You can also download a pre-compiled binary (for `linux`, `linux-arm`, `macos`,
`win-msvc`, `win-gnu`, `win32-msvc`) from the
[Releases](https://github.com/agnipau/slap/releases).

## Supported platforms

At the moment slap supports <a href="examples/bash">`bash`</a>, <a
href="examples/zsh">`zsh`</a>, <a href="examples/fish">`fish`</a>, <a
href="examples/elvish">`elvish`</a> and <a
href="examples/pwsh">`powershell`</a>.  
We are planning to support more shells.  
If your favourite shell is not supported, make sure to open an issue.

## Completions script generation

Thanks to [clap][clap], slap's underlying engine, automatic
completions-script generation is supported.
For example in bash:

```bash
config="path to your YAML config"
slap completions bash <"$config" >completions.bash
```

`completions.bash` now contains a bash script that provides command
autocompletion for the CLI described in your YAML config file.

## Dependency check

If your script depends on some programs you can check if they are in `$PATH`
with the `deps` subcommand:

```bash
slap deps curl jq || exit 1
```

If `curl` and `jq` are found in `$PATH` the script will continue its execution
and nothing will be printed, otherwise an error will be written to `stderr` and
slap will exit with a non-zero exit code.

## Absolute path of a script

slap includes a `path` subcommand that simplifies getting the absolute path of
a script:

```bash
# before
abs="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
```

```
# with slap
abs="$(slap path -d "${BASH_SOURCE[0]}")"
```

## Demo

[![asciicast](https://asciinema.org/a/357515.svg)](https://asciinema.org/a/357515)

## Example

Here are two useful bash scripts:

```bash
slap deps curl jq || exit 1

eval "$(slap parse bash _ -- "$@" <<-EOF
name: gh-repo-list
version: "1.0"
about: Outputs JSON containing useful informations about your GitHub repos.

settings:
    - ArgRequiredElseHelp
    - ColoredHelp
    - ColorAuto

args:
    - username:
        help: your GitHub username
        required: true
    - password:
        help: your GitHub password
        required: true
    - iterations:
        help: the number of iterations to do. 0 means there is no limit
        long: iterations
        short: i
        default_value: "0"
EOF
)"; [[ -z "${_success}" ]] && exit 1

page=1
while :; do
    data="$(curl -s -X GET \
        -u "${_username_vals}:${_password_vals}" \
    "https://api.github.com/user/repos?page=${page}&per_page100&type=all")"
    len="$(printf '%s\n' "${data}" | jq '. | length')"
    [[ "${_iterations_vals}" == "0" && "${len}" == 0 ]] && break
    printf '%s\n' "${data}"
    [[ "${page}" == "${_iterations_vals}" ]] && break
    page="$((page + 1))"
done
```

```bash
slap deps jq git || exit 1

eval "$(slap parse bash _ -- "$@" <<-EOF
name: gh-clone-repos
version: "1.0"
about: Uses 'gh-repo-list' to clone all your GitHub repos.

settings:
    - ArgRequiredElseHelp
    - ColoredHelp
    - ColorAuto

args:
    - username:
        help: your GitHub username
        required: true
    - password:
        help: your GitHub password
        required: true
    - git_options:
        help: "additional Git options (for example: --git-options '--depth 1')"
        long: git-options
        takes_value: true
        short: o
        allow_hyphen_values: true
EOF
)"; [[ -z "${_success}" ]] && exit 1

for repo in $(gh-repo-list "${_username_vals}" "${_password_vals}" \
    | jq -r "map(.ssh_url) | join(\"\n\")"); do
    if [[ -n "${_git_options_occurs}" ]]; then
        eval "git clone ${_git_options_vals} ${repo}"
    else
        git clone "${repo}"
    fi
done
```

## Learning material

This YAML <a href="examples/complete.yml">config</a> probably contains all the
options you'll ever need.  
For additional informations look at [`clap`'s
docs](https://docs.rs/clap/2.33.3/clap).

For <a href="examples/pwsh">`powershell`</a>, <a
href="examples/fish">`fish`</a>, <a href="examples/zsh">`zsh`</a> and other
examples look <a href="examples">here</a>.

## Elvish

As of `v0.14.1`, elvish doesn't support `eval` yet, so you can use slap to
generate elvish code, but you can't yet use the generated code inside an
elvish script.  
Luckily there is some work going on for this functionality.

## Credits

This program is solely made possible by [clap][clap], so many thanks to its
authors.

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>

[clap]: https://github.com/clap-rs/clap
