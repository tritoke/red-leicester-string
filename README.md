# red-leicester-string
It's stringcheese in Rust :)

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
