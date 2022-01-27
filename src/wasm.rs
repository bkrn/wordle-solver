
mod solver;

use js_sys::Array;
use wasm_bindgen::prelude::*;
use solver::*;


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


fn run_solver_with_target(be_cheaty: bool, target_str: String) -> Option<Vec<String>> {
    if is_valid_word(be_cheaty, target_str.clone()) {
        Some(solve_for_target(target_str, WordleSolver::create(be_cheaty)))
    } else {
        None
    }
}

#[wasm_bindgen]
pub fn solve(be_cheaty: bool, target: String) -> JsValue {
    if let Some(values) = run_solver_with_target(be_cheaty, target) {
        JsValue::from(values.into_iter()
        .map(|x| JsValue::from_str(&x))
        .collect::<Array>())
    } else {
        JsValue::null()
    }
}