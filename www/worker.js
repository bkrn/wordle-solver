

import("./solver.js").then(solver => {
    self.onmessage = function (e) {
        const {be_cheaty, target} = e.data;
        self.postMessage(solver.main(be_cheaty, target));
    };
    self.postMessage("ready");
})
