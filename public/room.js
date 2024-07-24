const send_form = document.getElementById("send_message")
const message_list = document.getElementById("message_list")

const stream_url = document.URL + "/stream"
const stream = new EventSource(stream_url)

send_form.addEventListener("submit", handleSendMessage)
stream.addEventListener("message", handleNewMessage)

function handleNewMessage(raw_message) {
  const message = JSON.parse(raw_message.data)
  console.log(message)
  const list_item = document.createElement("li")
  list_item.textContent = `${message.author}: ${message.content}`
  message_list.appendChild(list_item)
}

function handleSendMessage() {
  const message = new FormData(send_form)
  fetch(document.URL, {
    method: "POST",
    body: message
  })
}
