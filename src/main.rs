use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Write};
use std::str;
use std::sync::mpsc;
use std::time::Instant;

use serde_json;

static WORD_BYTES: &'static [u8; 64860] = include_bytes!("../words.txt");
static ANSWER_BYTES: &'static [u8; 11575] = include_bytes!("../answers.txt");
static ANSWER_INDEXES: &'static [u8; 13125] = include_bytes!("../answer_indexes.json");
static WORD_COUNT: usize = 12972;
static ANSWER_COUNT: usize = 2315;
static WORD_LEN: usize = 5;
static OUTCOME_LEN: usize = 253;
// log10(excepted share reduced each guess)
// so bigger values here mean the guesses
// are expected to perform better
static LOG_E_REDUCTION: f64 = 1.36172783602f64;

#[derive(Clone, Copy, Debug)]
enum CellResult {
    B,
    Y,
    G,
}

impl Into<usize> for CellResult {
    fn into(self) -> usize {
        match self {
            CellResult::B => 0,
            CellResult::Y => 1,
            CellResult::G => 2,
        }
    }
}

impl From<char> for CellResult {
    fn from(c: char) -> CellResult {
        match c {
            'y' => CellResult::Y,
            'g' => CellResult::G,
            _ => CellResult::B,
        }
    }
}

struct WordResult([CellResult; 5]);

impl From<[CellResult; 5]> for WordResult {
    fn from(res: [CellResult; 5]) -> WordResult {
        WordResult(res)
    }
}

impl From<String> for WordResult {
    fn from(s: String) -> WordResult {
        let mut res: [CellResult; 5] = [CellResult::B; 5];
        let chars: Vec<char> = s.chars().collect();
        for ix in 0..WORD_LEN {
            res[ix] = chars[ix].into()
        }
        WordResult(res)
    }
}

impl WordResult {
    fn index(&self) -> usize {
        let mut accum = 0usize;
        let mut ix = 0usize;
        for v in self.0 {
            accum += v as usize * 3usize.pow(ix as u32);
            ix += 1;
        }
        accum
    }
}

fn get_word_index(w: String) -> usize {
    let b = w.as_bytes();
    for ix in 0..WORD_COUNT {
        if b == get_word_bytes(ix) {
            return ix
        }
    }
    panic!("Word {} does not exist", w);
}

fn get_word_bytes(word_index: usize) -> [u8; 5] {
    let mut result: [u8; 5] = [0u8; 5];
    for i in 0..5 {
        result[i] = WORD_BYTES[word_index * 5 + i]
    }
    result
}

fn get_answer_bytes(word_index: usize) -> [u8; 5] {
    let mut result: [u8; 5] = [0u8; 5];
    for i in 0..5 {
        result[i] = ANSWER_BYTES[word_index * 5 + i]
    }
    result
}

fn get_word_string(word_index: usize) -> String {
    str::from_utf8(&get_word_bytes(word_index)).unwrap().into()
}

fn get_answer_string(word_index: usize) -> String {
    str::from_utf8(&get_answer_bytes(word_index)).unwrap().into()
}


fn get_outcome(guess_index: usize, target_index: usize) -> WordResult {
    get_outcome_by_word(get_word_bytes(guess_index), get_word_bytes(target_index))
}

fn get_outcome_by_word(guess: [u8; 5], mut target: [u8; 5]) -> WordResult {
    let mut outcome = [CellResult::B; 5];
    let mut remains: Vec<usize> = Vec::with_capacity(5);
    for ix in 0..WORD_LEN {
        if guess[ix] == target[ix] {
            outcome[ix] = CellResult::G;
            target[ix] = 0;
        } else {
            remains.push(ix)
        }
    }
    for ix in remains {
        if let Some(jx) = target.iter().position(|v| *v == guess[ix]) {
            outcome[ix] = CellResult::Y;
            target[jx] = 0;
        }
    }
    outcome.into()
}

#[derive(Clone)]
struct Mapping {
    data: Vec<Vec<HashSet<u16>>>,
    available: HashSet<u16>,
    forced_guesses: Vec<usize>
}

impl Mapping {

    fn get_answers(limit: bool) -> Vec<usize> {
        if limit {
            serde_json::from_slice(&ANSWER_INDEXES[..]).unwrap()
        } else {
            (0..WORD_COUNT).collect()
        }
    }

    fn create(limit: bool) -> Self {
        let answers: Vec<usize> = Self::get_answers(limit);
        let mut data = Vec::with_capacity(WORD_COUNT);
        let available = HashSet::from_iter(answers.iter().map(|u| *u as u16));
        for guess_ix in 0..WORD_COUNT {
            data.push(vec![HashSet::default(); 253]);
            for target_ix in &answers {
                data[guess_ix][get_outcome(guess_ix, *target_ix).index()].insert(*target_ix as u16);
            }
        }
        Self { data, available, forced_guesses: Vec::new() }
    }

    fn create_with_guesses(limit: bool, guesses: Vec<usize>) -> Self {
        let answers: Vec<usize> = Self::get_answers(limit);
        let mut data = Vec::with_capacity(WORD_COUNT);
        let available = HashSet::from_iter(answers.iter().map(|u| *u as u16));
        for guess_ix in 0..WORD_COUNT {
            data.push(vec![HashSet::default(); 253]);
            for target_ix in &answers {
                data[guess_ix][get_outcome(guess_ix, *target_ix).index()].insert(*target_ix as u16);
            }
        }
        Self { data, available, forced_guesses: guesses.into_iter().rev().collect() }
    }


    fn update(&mut self, word_ix: usize, result: WordResult) {
        self.available = self.data[word_ix][result.index()].clone();
        for wx in 0..WORD_COUNT {
            for ox in 0..OUTCOME_LEN {
                self.data[wx][ox] = self.data[wx][ox]
                    .intersection(&self.available)
                    .map(|v| *v)
                    .collect()
            }
        }
    }

    // Score based on expected elimination
    #[allow(dead_code)]
    fn get_score_v1(&self, wx: usize) -> f64 {
        let mut words_eliminated = 0f64;
        for word_set in self.data[wx].iter() {
            words_eliminated += (1f64 - (word_set.len() as f64 / self.available.len() as f64))
                * word_set.len() as f64;
        }
        let words_remaining = self.available.len() as f64 - words_eliminated;
        // Settle ties for possibly correct guesses
        words_remaining
            - if self.available.contains(&(wx as u16)) {
                f64::EPSILON
            } else {
                0f64
            }
    }

    // Score based on expected number of turns left
    #[allow(dead_code)]
    fn get_score_v2(&self, wx: usize) -> f64 {
        let mut words_eliminated = 0f64;
        for word_set in self.data[wx].iter() {
            words_eliminated += (1f64 - (word_set.len() as f64 / self.available.len() as f64))
                * word_set.len() as f64;
        }
        let one_turn_p = if self.available.contains(&(wx as u16)) {
            1f64 / self.available.len() as f64
        } else {
            0f64
        };
        // Expected turns left (not counting this one) if I make this guess
        (1f64 - one_turn_p)
            * (1f64 + ((self.available.len() as f64 - words_eliminated).log10() / LOG_E_REDUCTION))
    }

    fn get_score(&self, wx: usize) -> f64 {
        self.get_score_v2(wx)
    }

    fn get_guess(&mut self) -> usize {
        if self.available.len() == 1 {
            return *self.available.iter().next().unwrap() as usize;
        }

        if let Some(guess) = self.forced_guesses.pop() {
            return guess;
        }

        let mut current: (usize, f64) = (0usize, f64::INFINITY);
        for wx in 0..WORD_COUNT {
            let score = self.get_score(wx);
            if score < current.1 {
                current = (wx, score);
            }
        }
        current.0
    }
}

//Will probably use for tests at some point
fn run_solver_with_target(limit: bool, target_str: String) -> Vec<String> {
    solve_for_target_with_mapping(target_str, Mapping::create(limit))
}

fn solve_for_target_with_mapping(target_str: String, mut mapping: Mapping) -> Vec<String> {
    let mut res: Vec<String> = Vec::new();
    let s = target_str.as_bytes();
    let mut target: [u8; 5] = [0u8; 5];
    for i in 0..WORD_LEN {
        target[i] = s[i];
    }
    loop {
        let guess = mapping.get_guess();
        res.push(get_word_string(guess));
        if get_word_bytes(guess) == target {
            break;
        }
        mapping.update(guess, get_outcome_by_word(get_word_bytes(guess), target))
    }
    res
}

fn _loop_solver_interactive(mut mapping: Mapping, mut guesses: Vec<String>) -> Vec<String> {
    if mapping.available.len() <= 1 {
        println!(
            "Success! the word is '{}'",
            get_word_string(mapping.available.drain().next().unwrap() as usize)
        );
        return guesses;
    }
    let guess = mapping.get_guess();
    guesses.push(get_word_string(guess));
    println!("Enter \"{}\" into wordle", get_word_string(guess));
    print!("Submit result from wordle: ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("error: unable to read user input");
    mapping.update(guess, input.into());
    return _loop_solver_interactive(mapping, guesses);
}

fn run_solver_interactive(limit: bool) {
    println!("Enter input in format like bygbb\nWhere b is blank (or black), y is yellow, and g is green\n");
    println!("Creating intial mapping, please wait\n");
    _loop_solver_interactive(Mapping::create(limit), vec![]);
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

fn test_perf(limit: bool, guesses: Vec<usize>) -> Vec<Vec<String>> {
    use rand::thread_rng;
    use rand::seq::SliceRandom;

    let solvers = 12;
    let cloners = 4;

    let mut results: Vec<Vec<String>> = Vec::new();
    let mut have = 0;

    let mut v: Vec<usize> = (0..ANSWER_COUNT).collect();
    v.shuffle(&mut thread_rng());
    let mut words = v.into_iter();

    let (sender, receiver) = mpsc::channel();
    let (ms, mr) = mpsc::sync_channel(cloners);

    let spawn_solver = |ix| {
        let s = sender.clone();
        let m = mr.recv().unwrap();
        std::thread::spawn(move || {
            s.send(solve_for_target_with_mapping(get_answer_string(ix), m))
                .unwrap();
        });
    };

    for _ in 0..cloners {
        let s = ms.clone();
        let g = guesses.clone();
        std::thread::spawn(move || {
            let mapping = Mapping::create_with_guesses(limit, g);
            loop {
                s.send(mapping.clone()).unwrap();
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

fn run_perf_test(limit: bool, guesses: Option<Vec<usize>>) {
    

    let now = Instant::now();
    let results = test_perf(limit, guesses.unwrap_or_default());
    println!(
        "{}",
        results
            .iter()
            .map(|v| v.len() as f64)
            .fold(0f64, |a, b| a + b)
            / results.len() as f64
    );
    serde_json::to_writer(&File::create("data.json").unwrap(), &results).unwrap();
    println!("{}", now.elapsed().as_secs());
}

fn main() {
    let mut limit = false;
    let mut f: String = String::new();
    let mut guesses: Vec<usize> = Vec::new();
    for arg in std::env::args() {
        if arg == "--limit" {
            limit = true
        } else if arg.starts_with("--") {
            f = arg
        } else if f == "--perf" {
            guesses.push(get_word_index(arg))
        }
    }
    match f.as_str() {
        "--perf" => run_perf_test(limit, Some(guesses)),
        "--target" => {
            for guess in
                run_solver_with_target(limit, std::env::args().nth(2).expect("Require target string"))
            {
                println!("{}", guess);
            }
        }
        "--help" => println!("--help, --interactive [default], --perf [forced_guesses ...]\nPass in the --limit flag to use a model that knows available answers"),
        _ => run_solver_interactive(limit),
    }
}
