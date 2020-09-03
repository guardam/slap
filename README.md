# shlap

shlap (shell [`clap`][clap]) - painless argument parsing.

## Why?

Writing code to parse arguments in a shell scripting language (`bash`, `zsh`,
`fish` etc...) is an extremly verbose, repetitive, error prone, and painful
process.  
This program solves that.

## How?

You declare your CLI in YAML and pass it to shlap's `stdin` and pass all your
script's arguments to shlap as arguments.  
shlap makes sure that the arguments you pass to it conform to your YAML
description, and if not, it exits with an error code and outputs useful error
messages to `stderr`.  
In other words shlap handles the argument parsing logic and validation, your
script only evalutes the code exported by shlap and uses the parsed arguments.  
Here is an example bash script:

```bash
config="path to your YAML config"
eval "$(shlap bash parse -- "$@" <"$config")"
```

The `shlap-parse` subcommand, if the passed arguments conform to the YAML
description, outputs code in the language specified, so you can evaluate it to
have access to the variables containing the parsed arguments.  
Relax, shlap writes to `stdout` ONLY if the YAML config is valid and the
arguments passed conform to it, otherwise it doesn't.

## Supported platforms

At the moment shlap supports <a href="examples/bash">`bash`</a>, <a
href="examples/zsh">`zsh`</a>, <a href="examples/fish">`fish`</a>, <a
href="examples/elvish">`elvish`</a> and <a
href="examples/pwsh">`powershell`</a>.  
We are planning to support more shells.  
If your favourite shell is not supported, make sure to open an issue.

## Completions script generation

Thanks to [clap](#clap), shlap's underlying engine, automatic
completions-script generation is supported.
For example in bash:

```bash
config="path to your YAML config"
shlap bash completions <"$config" >completions.bash
```

`completions.bash` now contains a bash script that provides command
autocompletion for the CLI described in your YAML config file.

## Demo

[![asciicast](https://asciinema.org/a/357515.svg)](https://asciinema.org/a/357515)

## Learning material

This
[example](https://github.com/clap-rs/clap/blob/v2.33.1/examples/17_yaml.yml)
probably contains all the options you'll ever need.  
For additional informations look at [`clap`'s docs](https://docs.rs/clap/2.33.3/clap).

```yaml
name: yml_app
version: "1.0"
about: an example using a .yml file to build a CLI
author: Kevin K. <kbknapp@gmail.com>

# AppSettings can be defined as a list and are **not** ascii case sensitive
# Look here for all the possible settings: https://docs.rs/clap/2.33.3/clap/enum.AppSettings.html
settings:
  - ArgRequiredElseHelp

# All Args must be defined in the 'args:' list where the name of the arg, is the
# key to a Hash object
args:
  # The name of this argument, is 'opt' which will be used to access the value
  # later in your Rust code
  - opt:
      help: example option argument from yaml
      short: o
      long: option
      multiple: true
      takes_value: true
  - pos:
      help: example positional argument from yaml
      index: 1
      # A list of possible values can be defined as a list
      possible_values:
        - fast
        - slow
  - flag:
      help: demo flag argument
      short: F
      multiple: true
      global: true
      # Conflicts, mutual overrides, and requirements can all be defined as a
      # list, where the key is the name of the other argument
      conflicts_with:
        - opt
      requires:
        - pos
  - mode:
      long: mode
      help: shows an option with specific values
      # possible_values can also be defined in this list format
      possible_values: [vi, emacs]
      takes_value: true
  - mvals:
      long: mult-vals
      help: demos an option which has two named values
      # value names can be described in a list, where the help will be shown
      # --mult-vals <one> <two>
      value_names:
        - one
        - two
  - minvals:
      long: min-vals
      multiple: true
      help: you must supply at least two values to satisfy me
      min_values: 2
  - maxvals:
      long: max-vals
      multiple: true
      help: you can only supply a max of 3 values for me!
      max_values: 3

# All subcommands must be listed in the 'subcommand:' object, where the key to
# the list is the name of the subcommand, and all settings for that command are
# are part of a Hash object
subcommands:
  # The name of this subcommand will be 'subcmd' which can be accessed in your
  # Rust code later
  - subcmd:
      about: demos subcommands from yaml
      version: "0.1"
      author: Kevin K. <kbknapp@gmail.com>
      # Subcommand args are exactly like App args
      args:
        - scopt:
            short: B
            multiple: true
            help: example subcommand option
            takes_value: true
        - scpos1:
            help: example subcommand positional
            index: 1

# ArgGroups are supported as well, and must be specified in the 'groups:'
# object of this file
groups:
  # the name of the ArgGoup is specified here
  - min-max-vals:
      # All args and groups that are a part of this group are set here
      args:
        - minvals
        - maxvals
      # setting conflicts is done the same manner as setting 'args:'
      #
      # to make this group required, you could set 'required: true' but for
      # this example we won't do that.
```

Bash example:

```bash
config="path to your YAML config"
eval "$(shlap bash parse _ -- "$@" <"$config")"
[[ -z "$_success" ]] && exit 1

printf '%s\n' \
"opt     = '$_opt_vals'
pos     = '$_pos_vals'
flag    = '$_flag_vals'
mode    = '$_mode_vals'
mvals   = '$_mvals_vals'
minvals = '$_minvals_vals'
maxvals = '$_maxvals_vals'

subcommand   -> '$_subcommand'
subcmd_scopt  = '$_subcmd_scopt_vals'
subcmd_scpos1 = '$_subcmd_scpos1_vals'"
```

For <a href="examples/pwsh">`powershell`</a>, <a
href="examples/fish">`fish`</a>, <a href="examples/zsh">`zsh`</a> and other
examples look <a href="examples">here</a>.

## Elvish

As of `v0.14.1`, elvish doesn't support `eval` yet, so you can use shlap to
generate elvish code, but you can't yet use the generated code inside an
elvish script.  
Luckily there is some work going on for this functionality.

## Credits

This program is solely made possible by [clap](#clap), so many thanks to its
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
