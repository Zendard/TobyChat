const add_user_button = document.getElementById("add_user")
const user_input_field = document.querySelector("input.user_input_field")
let user_counter = 1

add_user_button.addEventListener("click", add_user_field)

function add_user_field() {
  const previous_field = document.querySelector(`input[name='users.${user_counter - 1}']`)
  if (!previous_field.reportValidity()) {
    return
  }

  const new_field = user_input_field.cloneNode(true)
  new_field.value = ""
  new_field.setAttribute('name', `users.${user_counter}`)

  previous_field.after(new_field)

  user_counter++
}
