# Use

After cloning you can run with `cargo run --release -- [options]`

Rust is required, installation instructions are here: https://doc.rust-lang.org/cargo/getting-started/installation.html

Can be run as `--interactive`, `--perf`, or `--target`

`--interactive` is the default and will work you through solving today's wordle challenge.

`--target` requires an argument that is the word to solve for and will print off the guesses the solver uses to get to the target. Note that the target must be in wordle's dictionary.

`--perf` runs a performance test over all possible answers in the wordle dictionary. You can specify guesses that the solver will use first by passing them after the `--perf` flag. This runs solvers in paralell and is memory/cpu intesnsive. It is currently set to run with 12 solvers with the CONCURRENCY static in main.rs. This uses about 20 gigs of RAM when run without `--limit` so adjust accordingly.

For all options you can also pass the `--limit` flag which will allow the solver to know the possible answers, a subset of the wordle dictionary. Up to you whether you think this is cheating.