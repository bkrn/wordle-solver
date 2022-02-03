

import("./solver.js").then(solver => {
    self.onmessage = function (e) {
        const {hard_mode, be_cheaty, target} = e.data;
        self.postMessage(solver.main(hard_mode, be_cheaty, target));
    };
    self.postMessage("ready");
})
