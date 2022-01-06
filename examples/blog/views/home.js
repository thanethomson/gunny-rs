/*
 * Renders a summary of the latest blog posts from most recent to least.
 */

function select() {
    return "posts/*.md";
}

function template() {
    return "templates/home.html";
}

function outputPattern() {
    // Only a single output file.
    return "public/index.html";
}

function onlyPublished(post) {
    return post.published
}

function newestFirst(post1, post2) {
    if (post1.published > post2.published) {
        return -1;
    } else if (post1.published < post2.published) {
        return 1;
    }
    return 0;
}

function process(posts) {
    posts = posts.filter(onlyPublished);
    posts.sort(newestFirst);
    // We only want a single output item.
    return [{posts: posts}];
}