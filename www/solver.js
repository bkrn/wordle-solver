
import * as wasm from "wordle-solver";


export const main = (be_cheaty, target) => {
    return wasm.solve(be_cheaty, target);
}