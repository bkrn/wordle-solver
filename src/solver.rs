use std::collections::HashSet;
use std::str;

use serde_json;

static WORD_BYTES: &'static [u8; 64860] = include_bytes!("../words.txt");
static ANSWER_INDEXES: &'static [u8; 13125] = include_bytes!("../answer_indexes.json");
static WORD_COUNT: usize = 12972;
pub static ANSWER_COUNT: usize = 2315;
pub static WORD_LEN: usize = 5;
// number of possible clues bbbbb, bbbby, ... == 3 ** 5
static CLUE_COUNT: usize = 243;

#[derive(Clone, Copy, Debug)]
enum CluePart {
    B,
    Y,
    G,
}

impl Into<usize> for CluePart {
    fn into(self) -> usize {
        match self {
            CluePart::B => 0,
            CluePart::Y => 1,
            CluePart::G => 2,
        }
    }
}

impl From<char> for CluePart {
    fn from(c: char) -> CluePart {
        match c {
            'y' => CluePart::Y,
            'g' => CluePart::G,
            _ => CluePart::B,
        }
    }
}

pub struct Clue([CluePart; 5]);

impl From<[CluePart; 5]> for Clue {
    fn from(res: [CluePart; 5]) -> Clue {
        Clue(res)
    }
}

impl From<String> for Clue {
    fn from(s: String) -> Clue {
        let mut res: [CluePart; 5] = [CluePart::B; 5];
        let chars: Vec<char> = s.chars().collect();
        for ix in 0..WORD_LEN {
            res[ix] = chars[ix].into()
        }
        Clue(res)
    }
}

impl Clue {
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

pub fn get_word_index(w: String) -> Option<usize> {
    let b = w.as_bytes();
    for ix in 0..WORD_COUNT {
        if b == get_word_bytes(ix) {
            return Some(ix);
        }
    }
    None
}

pub fn is_valid_word(be_cheaty: bool, w: String) -> bool {
    get_word_index(w)
        .map(|ix| get_answers(be_cheaty).contains(&ix))
        .unwrap_or_default()
}

pub fn get_word_bytes(word_index: usize) -> [u8; 5] {
    let mut result: [u8; 5] = [0u8; 5];
    for i in 0..5 {
        result[i] = WORD_BYTES[word_index * 5 + i]
    }
    result
}

pub fn get_word_string(word_index: usize) -> String {
    str::from_utf8(&get_word_bytes(word_index)).unwrap().into()
}

fn get_clue(guess_index: usize, target_index: usize) -> Clue {
    get_clue_by_word(get_word_bytes(guess_index), get_word_bytes(target_index))
}

pub fn get_clue_by_word(guess: [u8; 5], mut target: [u8; 5]) -> Clue {
    let mut outcome = [CluePart::B; 5];
    let mut remains: Vec<usize> = Vec::with_capacity(5);
    for ix in 0..WORD_LEN {
        if guess[ix] == target[ix] {
            outcome[ix] = CluePart::G;
            target[ix] = 0;
        } else {
            remains.push(ix)
        }
    }
    for ix in remains {
        if let Some(jx) = target.iter().position(|v| *v == guess[ix]) {
            outcome[ix] = CluePart::Y;
            target[jx] = 0;
        }
    }
    outcome.into()
}

// get_answers returns the indexes of actual possible
// answers either the entire Wordle dictionary or, if cheaty
// just the words that can actually be an answer in Wordle
pub fn get_answers(be_cheaty: bool) -> Vec<usize> {
    if be_cheaty {
        serde_json::from_slice(&ANSWER_INDEXES[..]).unwrap()
    } else {
        (0..WORD_COUNT).collect()
    }
}

#[derive(Clone)]
pub struct WordleSolver {
    // Game state in the format of []Word -> []Clue -> Bucket
    // Buckets are all possible answers that follow that word+clue
    game_state: Vec<Vec<HashSet<u16>>>,
    // Words that could still be the correct answer given previous guesses
    pub possible_answers: HashSet<u16>,
    // Guesses that the solver will use first, set at creation
    forced_guesses: Vec<usize>,
    hard_mode: bool,
}

impl WordleSolver {
    pub fn create(hard_mode: bool, be_cheaty: bool) -> Self {
        Self::create_with_guesses(hard_mode, be_cheaty, Vec::new())
    }

    pub fn create_with_guesses(hard_mode: bool, be_cheaty: bool, guesses: Vec<usize>) -> Self {
        let answers: Vec<usize> = get_answers(be_cheaty);
        let mut game_state = Vec::with_capacity(WORD_COUNT);
        let possible_answers = HashSet::from_iter(answers.iter().map(|u| *u as u16));
        for guess_ix in 0..WORD_COUNT {
            game_state.push(vec![HashSet::default(); CLUE_COUNT]);
            for target_ix in &answers {
                game_state[guess_ix][get_clue(guess_ix, *target_ix).index()]
                    .insert(*target_ix as u16);
            }
        }
        Self {
            game_state,
            possible_answers,
            forced_guesses: guesses.into_iter().rev().collect(),
            hard_mode,
        }
    }

    // After a guess update the game state to reflect the
    // (hopefully) decreased set of available answers
    pub fn update(&mut self, word_ix: usize, result: Clue) {
        self.possible_answers = self.game_state[word_ix][result.index()].clone();
        for wx in 0..WORD_COUNT {
            for ox in 0..CLUE_COUNT {
                self.game_state[wx][ox] = self.game_state[wx][ox]
                    .intersection(&self.possible_answers)
                    .map(|v| *v)
                    .collect()
            }
        }
    }

    fn get_score(&self, word: usize) -> f64 {
        let mut expected_words_eliminated = 0f64;
        // given a word iterate over the buckets of possible answers given each possible clue
        for bucket in self.game_state[word].iter() {
            // probability of getting a particulkar clue is size of bucket / remaining possible answers
            let p_of_clue = bucket.len() as f64 / self.possible_answers.len() as f64;
            // expected words eliminated by this clue is the proibability of notgetting it
            // times the size of its bucket
            expected_words_eliminated += (1f64 - p_of_clue) * bucket.len() as f64;
        }
        // flip into expected words remaining for reasons that no longer
        // make sense after refactors
        let expected_words_remaining =
            self.possible_answers.len() as f64 - expected_words_eliminated;
        // Settle ties for possibly correct guesses by
        // giving them a small bonus
        let is_possible_answer = self.possible_answers.contains(&(word as u16));
        expected_words_remaining
            - if is_possible_answer {
                f64::EPSILON
            } else {
                0f64
            }
    }

    // Get the next guess either because it is correct
    // we are forced to choose it by the configuration
    // or it will eliminate the greatest number of currently
    // possible answers
    pub fn get_guess(&mut self) -> usize {
        // If we know what the answer is, just return it
        if self.possible_answers.len() == 1 {
            return *self.possible_answers.iter().next().unwrap() as usize;
        }

        // If we've been forced to use TREAD or LIONS just do it
        if let Some(guess) = self.forced_guesses.pop() {
            return guess;
        }

        let mut current: (usize, f64) = (0usize, f64::INFINITY);
        let itr: Vec<usize> = if self.hard_mode {
            self.possible_answers.iter().map(|u| *u as usize).collect()
        } else {
            (0..WORD_COUNT).collect::<Vec<usize>>()
        };
        for word in itr {
            let score = self.get_score(word);
            if score <= current.1 {
                current = (word, score);
            }
        }
        current.0
    }
}
