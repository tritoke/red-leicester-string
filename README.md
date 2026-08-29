# red-leicester-string

It's stringcheese in Rust :)

There are a number of optimisations:
- Strides (i.e. searching every other byte etc) are handled with Rust's strided crate so occur no memory allocation overhead
- Files are mmap-ed in
- Strides of the file are searched in parallel

Over all this means much larger files can be searched and faster as the aho-corasick automaton built is shared between all threads.
This is only possible due to some excellent library design from [Andrew Gallant](https://github.com/burntsushi/), and the Rust crate eco system.

## Installation
You can install the `stringcheese` binary via `cargo install stringcheese`.

## Examples

For example it should find flag{in_the_readme :)} when run on this file
It can also find flags backwards: }!sdrawkcab m'i{galf
Strides of the file are also supported:
  - 2: ufrlmaPgA{GsgtCrdiSpLerdG}
  - 5: }qSgvySyhbeOYqNpNGwsiuFOmrVrSAtJdqpsLrWO{qJJQgSRnLaLVuolnRRrfFfja

By default strides up to 32 are searched.

All of the encodings of the original stringcheese tool are now suppored.
Some examples:
- base64: ZmxhZ3t3ZSBkbyBhIGxpdHRsZSBlbmNvZGluZ30=
- ROT47: MSHNb0lT PU 96; y|fffd

Additionally there is `--strict` which will prevent partial flags from being output,
i.e. flag{ should not be output on its own

## AI notice

This codebase is entirely hand written, I do not use AI and I would encourage you not to as well :)
