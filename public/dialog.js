const dialog = document.getElementById("dialog")

const params = new URLSearchParams(window.location.search)

console.log(params)

if (params.has("error")) {
  dialog.classList.add("error")
  dialog.innerText = params.get("error")
  dialog.show()
} else {
  dialog.classList.add("success")
  dialog.innerText = params.get("success")
  dialog.show()
}

