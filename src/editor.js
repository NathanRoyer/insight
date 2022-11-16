let viewButton;
let viewButtonNormalText;
let markdownEntry;
let saved = true;
let saving = false;

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

function protectPost() {
    let body = post.key + token + email;
    api_post('/' + post.post + '/protect', body, request => {
        if (request.status == 200) {
            document.location = '/' + post.post + '/' + request.responseText;
        } else {
            alert('Error: ' + request.responseText);
        }
    });
}

function init() {
    onAuthentication = protectPost;
    commonInit();

    post = JSON.parse(atob(post));

    viewButton = element('view-button');
    markdownEntry = element('markdown');

    markdownEntry.value = post.content;
    viewButtonNormalText = viewButton.innerText;

    viewButton.addEventListener('click', () => open('/' + post.post, '_blank'));

    let protectButton = element('protect-button');
    if (post.author) protectButton.remove();
    else protectButton.addEventListener('click', () => {
        element('auth').classList.remove('hidden');
    });

    periodicCheck();
}
