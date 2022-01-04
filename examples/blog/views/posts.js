/*
 * Renders all of the blog posts to a `public` folder.
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

function process(post) {
    // Skip processing all unpublished posts.
    if (!post.published) {
        return null;
    }
    let dateParts = post.published.split('-');
    // For the output pattern.
    post.year = dateParts[0];
    post.month = dateParts[1];
    // Ensure the post's content is rendered as HTML using the built-in
    // markdownToHtml function.
    post.content = markdownToHtml(post.content);
    return post;
}
