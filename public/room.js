const send_form = document.getElementById("send_message")
const message_list = document.getElementById("message_list")

const socket_url = document.URL + "/socket"
const websocket = new WebSocket(socket_url)

send_form.addEventListener("submit", handleSendMessage)
websocket.addEventListener("message", handleNewMessage)

function handleNewMessage(message) {
  console.log(message)
  const list_item = document.createElement("li")
  list_item.textContent = message.data
  message_list.appendChild(list_item)
}

function handleSendMessage() {
  const message = new FormData(send_form).get("message")
  websocket.send(message)
}
