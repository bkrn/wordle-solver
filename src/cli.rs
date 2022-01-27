use std::fs::File;
use std::io::{self, Write};
use std::sync::mpsc;
use std::time::Instant;

mod solver;

use solver::*;


static CONCURRENCY: usize = 12;

/*****************************
         * warning *
you probably want solver.rs
this is glue to the CLI not
the solver side of the app
*****************************/

fn run_solver_with_target(be_cheaty: bool, target_str: String) -> Vec<String> {
    solve_for_target(target_str, WordleSolver::create(be_cheaty))
}

fn solve_for_target(target_str: String, mut solver: WordleSolver) -> Vec<String> {
    let mut res: Vec<String> = Vec::new();
    let s = target_str.as_bytes();
    let mut target: [u8; 5] = [0u8; 5];
    for i in 0..WORD_LEN {
        target[i] = s[i];
    }
    loop {
        let guess = solver.get_guess();
        res.push(get_word_string(guess));
        if get_word_bytes(guess) == target {
            break;
        }
        solver.update(guess, get_clue_by_word(get_word_bytes(guess), target))
    }
    res
}

fn _loop_solver_interactive(
    mut solver: WordleSolver,
    mut guesses: Vec<String>,
) -> Vec<String> {
    if solver.possible_answers.len() <= 1 {
        println!(
            "Success! the word is '{}'",
            get_word_string(solver.possible_answers.drain().next().unwrap() as usize)
        );
        return guesses;
    }
    let guess = solver.get_guess();
    guesses.push(get_word_string(guess));
    println!("Enter \"{}\" into wordle", get_word_string(guess));
    print!("Submit result from wordle: ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("error: unable to read user input");
        solver.update(guess, input.into());
    return _loop_solver_interactive(solver, guesses);
}

fn run_solver_interactive(be_cheaty: bool) {
    println!("Enter input in format like bygbb\nWhere b is blank (or black), y is yellow, and g is green\n");
    println!("Creating intial WordleSolver, please wait\n");
    _loop_solver_interactive(WordleSolver::create(be_cheaty), vec![]);
}

fn update_timer(start: Instant, current: usize, total: usize, avg: f64) {
    let elapsed = start.elapsed().as_secs_f64();
    let p = current as f64 / total as f64;
    let total = elapsed * (1f64 / p);
    print!(
        "{:.2}% done projected total seconds {:.0}, remaining {:.0} -- current avg {:.2}        \r",
        p * 100f64,
        total,
        total - elapsed,
        avg
    )
}

fn test_perf(be_cheaty: bool, guesses: Vec<usize>) -> Vec<Vec<String>> {
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    let solvers = CONCURRENCY;
    let cloners = if solvers > 3 { solvers / 3 } else { 1 };

    let mut results: Vec<Vec<String>> = Vec::new();
    let mut have = 0;

    let mut v: Vec<usize> = get_answers(true);
    v.shuffle(&mut thread_rng());
    let mut words = v.into_iter();

    let (sender, receiver) = mpsc::channel();
    let (ms, mr) = mpsc::sync_channel(cloners);

    let spawn_solver = |ix| {
        let s = sender.clone();
        let m = mr.recv().unwrap();
        std::thread::spawn(move || {
            s.send(solve_for_target(get_word_string(ix), m)).unwrap();
        });
    };

    for _ in 0..cloners {
        let s = ms.clone();
        let g = guesses.clone();
        std::thread::spawn(move || {
            let solver = WordleSolver::create_with_guesses(be_cheaty, g);
            while s.send(solver.clone()).is_ok() {
                continue;
            }
        });
    }

    for _ in 0..solvers {
        let ix = words.next().unwrap();
        spawn_solver(ix);
    }

    let start = Instant::now();
    let mut guess_count = 0;

    for ix in words {
        io::stdout().flush().unwrap();
        results.push(receiver.recv().unwrap());
        guess_count += results.last().map(|v| v.len()).unwrap_or_default();
        have += 1;
        update_timer(
            start,
            have,
            ANSWER_COUNT,
            guess_count as f64 / results.len() as f64,
        );
        spawn_solver(ix);
    }

    while have < ANSWER_COUNT {
        io::stdout().flush().unwrap();
        results.push(receiver.recv().unwrap());
        guess_count += results.last().map(|v| v.len()).unwrap_or_default();
        have += 1;
        update_timer(
            start,
            have,
            ANSWER_COUNT,
            guess_count as f64 / results.len() as f64,
        );
    }

    results
}

fn run_perf_test(be_cheaty: bool, guesses: Option<Vec<usize>>) {
    let now = Instant::now();
    let results = test_perf(be_cheaty, guesses.unwrap_or_default());
    println!(
        "\n{}",
        results
            .iter()
            .map(|v| v.len() as f64)
            .fold(0f64, |a, b| a + b)
            / results.len() as f64
    );
    serde_json::to_writer(&File::create("game_state.json").unwrap(), &results).unwrap();
    println!("{}", now.elapsed().as_secs());
}

fn main() {
    let mut be_cheaty = false;
    let mut is_target = false;
    let mut target = None;
    let mut f: String = String::new();
    let mut guesses: Vec<usize> = Vec::new();
    for arg in std::env::args() {
        if arg == "--be-cheaty" {
            be_cheaty = true
        } else if arg == "--target" {
            is_target = true;
            f = arg
        } else if arg.starts_with("--") {
            f = arg
        } else if is_target {
            target = Some(arg)
        } else if f == "--perf" {
            guesses.push(get_word_index(arg).expect("Word not in dictionary"))
        }
    }
    match f.as_str() {
        "--perf" => run_perf_test(be_cheaty, Some(guesses)),
        "--target" => {
            for guess in
                run_solver_with_target(be_cheaty, target.expect("--target required target string"))
            {
                println!("{}", guess);
            }
        }
        "--help" => println!("--help, --interactive [default], --perf [forced_guesses ...]\nPass in the --be_cheaty flag to use a model that knows possible_answers answers"),
        _ => run_solver_interactive(be_cheaty),
    }
}
