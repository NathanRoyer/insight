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

function save() {
    saving = true;
    let request = new XMLHttpRequest();
    viewButton.innerText = 'Saving...';
    viewButton.disabled = true;
    request.responseType = 'text';
    request.open('POST', '/update', true);

    request.onreadystatechange = function() {
        if (request.status != 200) {
            console.error(request.responseText);
            alert('Error while saving; details in web tools.');
        } else {
            viewButton.innerText = viewButtonNormalText;
            viewButton.disabled = false;
            saving = false;
        }
    }

    request.send(JSON.stringify(post));
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

function insert(markdown) {
    let cursor = markdownEntry.selectionStart;
    let before = markdownEntry.value.substring(0, cursor);
    let after = markdownEntry.value.substring(cursor);
    markdownEntry.value = before + markdown + after;
}

function init() {
    post = JSON.parse(atob(post));
    viewButton = document.getElementById('view-button');
    viewButtonNormalText = viewButton.innerText;
    markdownEntry = document.getElementById('markdown');
    markdownEntry.value = post.content;

    setButtonCallback('title', () => insert('\n\n# My Title\n\n'));
    setButtonCallback('image', () => insert('\n\n![label](https://image-url)\n\n'));
    setButtonCallback('link', () => insert('\n\n[label](https://link.com)\n\n'));
    setButtonCallback('list', () => insert('\n\n- Item 1\n- Item 2\n Item 3\n\n'));
    setButtonCallback('view', () => open('/' + post.post, '_blank'));

    periodicCheck();
}
