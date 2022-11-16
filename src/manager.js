let emailField;
let codeField;
let status;
let postList;
let codeInput;
let emailInput;
let checkButton;
let submitButton;
let listPostsButton;

let token;
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

function onCheck() {
    status.innerText = 'Checking...';
    ckeckButton.disabled = true;
    codeInput.classList.add('hidden');
    listPostsButton.classList.add('hidden');
    email = emailField.value;

    api_post('/send-email-code', email, request => {
        ckeckButton.disabled = false;
        status.innerText = request.responseText;
        if (request.status == 200) {
            codeInput.classList.remove('hidden');
            emailInput.classList.add('hidden');
        }
    });
}

function onCodeSubmit() {
    status.innerText = 'Submitting code...';
    submitButton.disabled = true;
    listPostsButton.classList.add('hidden');

    api_post('/check-email-code', codeField.value + email, request => {
        submitButton.disabled = false;
        if (request.status == 200) {
            status.innerText = "Authenticated!";
            codeInput.classList.add('hidden');
            listPostsButton.classList.remove('hidden');
            token = request.responseText;
        } else {
            status.innerText = request.responseText;
        }
    });
}

function onListPosts() {
    status.innerText = 'Listing posts...';
    listPostsButton.disabled = true;

    api_post('/list-posts', token + email, request => {
        listPostsButton.disabled = false;
        if (request.status == 200) {
            status.innerText = 'Click on a post to edit in a new tab';
            postList.innerHTML = "";

            let posts = request.responseText.split('\n');
            for (let i = 0; i < posts.length; i++) {
                let post = posts[i].split(':');
                let id = post[0];
                let title = atob(post[1]);

                let a = document.createElement('a');
                a.innerText = title;
                a.dataset.postId = id;
                a.href = '';
                a.addEventListener('click', onPostClick);

                let li = document.createElement('li');
                li.appendChild(a);
                postList.appendChild(li);
            }
        } else {
            status.innerText = request.responseText;
        }
    });
}

function onPostClick(event) {
    let postId = event.target.dataset.postId;

    api_post('/' + postId + '/get-edit-link', token + email, request => {
        if (request.status == 200) {
            let oneTimeKey = request.responseText;
            open('/' + postId + '/' + oneTimeKey, '_blank');
        } else {
            status.innerText = request.responseText;
        }
    });

    event.preventDefault();
}

function init() {
    codeInput = document.getElementById('code-input');
    emailInput = document.getElementById('email-input');
    postList = document.getElementById('post-list');
    status = document.getElementById('status');
    emailField = document.getElementById('email-field');
    codeField = document.getElementById('code-field');

    ckeckButton = document.getElementById('check-button');
    ckeckButton.addEventListener('click', onCheck);

    submitButton = document.getElementById('submit-button');
    submitButton.addEventListener('click', onCodeSubmit);

    listPostsButton = document.getElementById('list-posts-button');
    listPostsButton.addEventListener('click', onListPosts);
}