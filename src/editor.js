let viewButton;
let viewButtonNormalText;
let markdownEntry;
let saved = true;
let saving = false;
let articleSlug;

function onTextEdit() {
    let elements = document.body.children;
    let descriptionIndex = 0;
    for (let i = 0; i < elements.length; i++) {
        if (elements[i] == this) {
            if (this[this.prop].trim()) {
                article.content[descriptionIndex].text = this[this.prop];
            } else {
                article.content.splice(descriptionIndex, 1);
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

    api_post('/' + articleSlug + '/update', JSON.stringify(article), request => {
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
    if (markdownEntry.value !== article.content) {
        article.content = markdownEntry.value;
        saved = false;
    } else if (!saved && !saving) {
        save();
        saved = true;
    }
    setTimeout(periodicCheck, 500);
}

function protectPost() {
    let body = article.key + token + email;
    api_post('/' + articleSlug + '/protect', body, request => {
        if (request.status == 200) {
            document.location = '/' + articleSlug + '/' + request.responseText;
        } else {
            alert('Error: ' + request.responseText);
        }
    });
}

function onDelete() {
    if (confirm("Delete the post?")) {
        api_post('/' + articleSlug + '/delete', article.key, request => {
            if (request.status == 200) {
                let markdown = markdownEntry.value;
                let home = () => document.location = '/';
                if (confirm("Copy markdown to clipboard?")) {
                    navigator.clipboard.writeText(markdown).then(home, home);
                } else {
                    home();
                }
            } else {
                alert('Error: ' + request.responseText);
            }
        });
    }
}

function base64DecodeUnicode(encoded) {
    return decodeURIComponent(atob(encoded).split('').map(function(c) {
        return '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2);
    }).join(''));
}

function init() {
    sendEmailCode = '/send-email-code-create';
    onAuthentication = protectPost;
    commonInit();

    articleSlug = document.location.pathname.split('/')[1];
    article = JSON.parse(base64DecodeUnicode(article));

    viewButton = element('view-button');
    markdownEntry = element('markdown');

    markdownEntry.value = article.content;
    viewButtonNormalText = viewButton.innerText;

    viewButton.addEventListener('click', () => {
        open('/' + articleSlug, '_blank');
    });

    let deleteButton = element('delete-button');
    deleteButton.addEventListener('click', onDelete);

    let protectButton = element('protect-button');
    if (article.author) protectButton.remove();
    else protectButton.addEventListener('click', () => {
        element('popup').classList.remove('hidden');
    });

    periodicCheck();
}
