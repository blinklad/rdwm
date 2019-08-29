# rdwm
## A Rust clone of dwm, using c2rust tooling for initial prototypes

## Goals
Basically just to play around with [c2rust](https://github.com/immunant/c2rust)'s tooling and features, at least initially. 
The moment between rdwm being primarily a c2rust 'fork' of dwm using (relatively) portable and automated codegen and refactor tools,
and rdwm _diverging from upstream dwm entirely_ is yet to be set in stone. 

While not a _goal_ as such, a large priority will be maintaining a reasonable delta between necessary human refactoring and what the initial tooling provides between architectures, toolchains and (inherited or unsquashable) bugs. In any case, this will documented and other build, organisation or refactor tooling will be provided and maintained.

Other than happy hacking, I would love to make a minimalistic, dwm-inspired yet idiomatic clone that could be used as a basis for other Rust WM projects.

## Requisites
### Development dependencies
1. Rust nightly
2. Rust nightly-06-22 (for c2rust)
3. intercept-build tools (eg. bear, cmake)

