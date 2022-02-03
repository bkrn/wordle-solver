
import * as wasm from "wordle-solver";


export const main = (hard_mode, be_cheaty, target) => {
    return wasm.solve(hard_mode, be_cheaty, target);
}