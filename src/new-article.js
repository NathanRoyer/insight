let articleIdField;
let createButton;

function createArticle() {
    if (checkValidSlug()) {
        api_post('/create', articleIdField.value, request => {
            if (request.status == 200) {
                document.location = request.responseText;
            } else {
                status.innerText = request.responseText;
            }
        });
    } else {
        alert("The slug you choose is invalid.");
    }
}

function checkValidSlug() {
    let validity = articleIdField.value.length > 0;
    validity &&= articleIdField.validity.valid;
    return validity;
}

function init() {
    articleIdField = element('article-id-field');
    status = element('status');
    createButton = element('create-button');

    createButton.addEventListener('click', createArticle);

    onSlugChange();
}