




const worker = new Worker('worker.js');
worker.onmessage = _ => {
    worker.onmessage = function (e) {
        document.getElementById("status").innerText = e.data ? e.data.join('\n') : "Word not in current Wordle dictionary";
    };
    document.getElementById("entry").hidden = false;
    document.getElementById("entry").onsubmit = (e) => {
        e.preventDefault();
        const args = {
            be_cheaty: document.getElementById("cheat").checked,
            target: document.getElementById("word").value,
        };
        worker.postMessage(args);
        document.getElementById("status").innerText = "Working ... this could take a bit"
    }
};





const solve = () => {
    
    

    
}


solve()