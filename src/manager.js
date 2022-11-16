let postList;

function listPosts() {
    status.innerText = 'Listing posts...';

    api_post('/list-posts', token + email, request => {
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

                let viewLink = document.createElement('a');
                viewLink.innerText = title;
                viewLink.href = '/' + id;
                viewLink.target = '_blank';

                let editLink = document.createElement('a');
                editLink.innerText = 'edit';
                editLink.dataset.postId = id;
                editLink.href = '';
                editLink.addEventListener('click', onPostClick);

                let li = document.createElement('li');
                li.appendChild(viewLink);
                li.appendChild(document.createTextNode(' - '));
                li.appendChild(editLink);
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
    onAuthentication = listPosts;
    commonInit();

    postList = element('post-list');
    element('list-posts-button').addEventListener('click', listPosts);
}