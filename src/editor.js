function onTextEdit() {
    let elements = document.body.children;
    let descriptionIndex = 0;
    for (let i = 0; i < elements.length; i++) {
        if (elements[i] == this) {
            if (this[this.prop].trim()) {
                post.content[descriptionIndex].text = this[this.prop];
            } else {
                post.content.splice(descriptionIndex, 1);
                this.nextElementSibling.remove();
                this.remove();
            }
            break;
        }
        if (elements[i].classList.contains('described')) {
            descriptionIndex++;
        }
    }
}

let viewButton;
let viewButtonNormalText;
let markdownEntry;
let saved = true;
let saving = false;
let email;

function api_post(path, body, callback) {
    let request = new XMLHttpRequest();
    request.responseType = 'text';
    request.open('POST', path, true);
    request.onreadystatechange = () => {
        if (request.readyState === XMLHttpRequest.DONE) callback(request);
    };
    request.send(body);
}

function save() {
    saving = true;
    viewButton.innerText = 'Saving...';
    viewButton.disabled = true;

    api_post('/update', JSON.stringify(post), request => {
        if (request.status != 200) {
            console.error(request.responseText);
            alert('Error while saving; details in web tools.');
        } else {
            viewButton.innerText = viewButtonNormalText;
            viewButton.disabled = false;
            saving = false;
        }
    });
}

function periodicCheck() {
    if (markdownEntry.value !== post.content) {
        post.content = markdownEntry.value;
        saved = false;
    } else if (!saved && !saving) {
        save();
        saved = true;
    }
    setTimeout(periodicCheck, 500);
}

function setButtonCallback(btn, callback) {
    let button = document.getElementById(btn + '-button');
    button.addEventListener('click', callback);
}

function protect_post(token) {
    let body = post.key + token + email;
    api_post('/' + post.post + '/protect', body, request => {
        if (request.status == 200) {
            document.location = '/' + post.post + '/' + request.responseText;
        } else {
            alert('Error: ' + request.responseText);
        }
    });
}

function check_code(msg) {
    let code = prompt(msg, '');
    if (code) {
        api_post('/check-email-code', code + email, request => {
            if (request.status == 200) {
                protect_post(request.responseText);
            } else {
                alert('Error: ' + request.responseText);
            }
        });
    }
}

function protect_get_email() {
    email = prompt('Enter your email:', '');
    if (email) {
        api_post('/send-email-code-create', email, request => {
            if (request.status == 200) check_code(request.responseText);
            else alert('Error: ' + request.responseText);
        });
    }
}

function init() {
    post = JSON.parse(atob(post));
    viewButton = document.getElementById('view-button');
    viewButtonNormalText = viewButton.innerText;

    markdownEntry = document.getElementById('markdown');
    markdownEntry.value = post.content;

    let protectButton = document.getElementById('protect-button');
    if (post.author) protectButton.remove();
    else protectButton.addEventListener('click', protect_get_email);

    setButtonCallback('view', () => open('/' + post.post, '_blank'));

    periodicCheck();
}
