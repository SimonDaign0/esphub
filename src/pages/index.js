const protocol = window.location.protocol.startsWith("https:")
    ? "wss://"
    : "ws://";
const ws = new WebSocket(protocol + window.location.host);
ws.onopen = () => {
    alert("WS open!");
};
ws.onclose = () => {
    alert("WS closed!");
};

const send_button = document.getElementById("send-ws");
send_button.addEventListener("click", () => send());

function send() {
    ws.send("My custom msg over websocket!");
}
