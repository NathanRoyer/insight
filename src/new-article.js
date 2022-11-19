let articleIdField;

function createArticle() {
    api_post('/create', articleIdField.value, request => {
        if (request.status == 200) {
            document.location = request.responseText;
        } else {
            status.innerText = request.responseText;
        }
    });
}

function init() {
    articleIdField = element('article-id-field');
    status = element('status');

    element('create-button').addEventListener('click', createArticle);
}