let articleList;

function listPosts() {
    status.innerText = 'Listing articles...';

    api_post('/list-articles', token + email, request => {
        if (request.status == 200) {
            status.innerText = 'Click on an article to edit in a new tab';
            articleList.innerHTML = "";

            let articles = request.responseText.split('\n');
            for (let i = 0; i < articles.length; i++) {
                let article = articles[i].split(':');
                let id = article[0];
                let title = atob(article[1]);

                let a = document.createElement('a');
                a.innerText = title;
                a.dataset.articleId = id;
                a.href = '';
                a.addEventListener('click', onPostClick);

                let viewLink = document.createElement('a');
                viewLink.innerText = title;
                viewLink.href = '/' + id;
                viewLink.target = '_blank';

                let editLink = document.createElement('a');
                editLink.innerText = 'edit';
                editLink.dataset.articleId = id;
                editLink.href = '';
                editLink.addEventListener('click', onPostClick);

                let li = document.createElement('li');
                li.appendChild(viewLink);
                li.appendChild(document.createTextNode(' - '));
                li.appendChild(editLink);
                articleList.appendChild(li);
            }
        } else {
            status.innerText = request.responseText;
        }
    });
}

function onPostClick(event) {
    let articleId = event.target.dataset.articleId;

    api_post('/' + articleId + '/get-edit-link', token + email, request => {
        if (request.status == 200) {
            let oneTimeKey = request.responseText;
            open('/' + articleId + '/' + oneTimeKey, '_blank');
        } else {
            status.innerText = request.responseText;
        }
    });

    event.preventDefault();
}

function init() {
    sendEmailCode = '/send-email-code';
    onAuthentication = listPosts;
    commonInit();

    articleList = element('article-list');
    element('list-articles-button').addEventListener('click', listPosts);
}