const protocol = window.location.protocol.startsWith("https:")
    ? "wss://"
    : "ws://";
const serv_stat = document.getElementById("serv_stat");
const recon = document.getElementById("rec_button");
const send_button = document.getElementById("send-ws");

let ws;
send_button.style.display = "none";
serv_stat.style.color = "red";

function initWebSocket() {
    if (ws) {
        ws.onopen = null;
        ws.onclose = null;
    }

    ws = new WebSocket(protocol + window.location.host);

    ws.onopen = () => {
        serv_stat.style.color = "green";
        recon.style.color = "red";
        recon.innerText = "Disconnect";
        send_button.style.display = "block";
    };

    ws.onclose = () => {
        serv_stat.style.color = "red";
        recon.style.color = "green";
        recon.innerText = "Connect";
        send_button.style.display = "none";
    };
}

initWebSocket();

send_button.addEventListener("click", () => send());
recon.addEventListener("click", () => reconnect());

function send() {
    try {
        if (ws.readyState === WebSocket.OPEN) {
            ws.send("TOGGLE: Custom msg over websocket!");
        }
    } catch (err) {
        console.error(err);
    }
}

async function reconnect() {
    if (ws.readyState === WebSocket.OPEN) {
        ws.close();
    } else if (ws.readyState === WebSocket.CONNECTING) {
        console.log("Still attempting to connect...");
    } else {
        initWebSocket();
    }
}
