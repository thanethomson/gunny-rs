/*
 * Renders all the blog posts to a `public` folder.
 */

function select() {
    return "posts/*.md";
}

function template() {
    return "templates/post.html";
}

function outputPattern() {
    // e.g. ./public/2022/01/original-filename/index.html
    return "public/{{ year }}/{{ month }}/{{ id }}/index.html";
}

function onlyPublished(post) {
    return post.published
}

function extractDate(post) {
    let dateParts = post.published.split('-');
    // For the output pattern.
    post.year = dateParts[0];
    post.month = dateParts[1];
    return post;
}

function renderHtml(post) {
    post.content = markdownToHtml(post.content);
    return post;
}

function process(posts) {
    return posts
        .filter(onlyPublished)
        .map(extractDate)
        .map(renderHtml);
}
