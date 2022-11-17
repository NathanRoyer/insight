let ckeckButton;
let submitButton;
let emailField;
let codeField;
let status;
let onAuthentication;
let sendEmailCode;

let token;
let email;

const element = (id) => document.getElementById(id);

function api_post(path, body, callback) {
    let request = new XMLHttpRequest();
    request.responseType = 'text';
    request.open('POST', path, true);
    request.onreadystatechange = () => {
        if (request.readyState === XMLHttpRequest.DONE) callback(request);
    };
    request.send(body);
}

function onCheck() {
    status.innerText = 'Checking...';
    ckeckButton.disabled = true;
    submitButton.disabled = true;
    email = emailField.value;

    api_post(sendEmailCode, email, request => {
        ckeckButton.disabled = false;
        status.innerText = request.responseText;
        if (request.status == 200) {
            submitButton.disabled = false;
        }
    });
}

function onCodeSubmit() {
    status.innerText = 'Submitting code...';
    submitButton.disabled = true;

    api_post('/check-email-code', codeField.value + email, request => {
        submitButton.disabled = false;
        if (request.status == 200) {
            auth.classList.add('hidden');
            token = request.responseText;
            onAuthentication();
        } else {
            status.innerText = request.responseText;
        }
    });
}

function commonInit() {
    status = document.getElementById('status');
    emailField = document.getElementById('email-field');
    codeField = document.getElementById('code-field');
    ckeckButton = document.getElementById('check-button');
    ckeckButton.addEventListener('click', onCheck);

    submitButton = document.getElementById('submit-button');
    submitButton.addEventListener('click', onCodeSubmit);

    ckeckButton.disabled = false;
    submitButton.disabled = true;
}
