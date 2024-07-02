const form = document.getElementById("login")
const passwordInput = form.querySelector("input[name='password']")

form.addEventListener("submit",async ()=>{
  fetch("/login/checkuser",{
    method : "POST",
    body : new FormData(form)
  })
  .then((res)=> res.text())
  .then(handleResponse)
})

function handleResponse(text){
  console.log(text)

  switch (text) {
    case "NewUser":
      window.location.href="/register"
      break

    case "WrongPassword":
      passwordInput.classList.add("wrong")
      passwordInput.value=""
      passwordInput.placeholder="Wrong password!"
      break

    case "LoggedIn":
      window.location.href="/"
      break
  }
}
